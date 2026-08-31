// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The paid coinage flow: paying for anonymous unload tokens with the external asset and spending
//! them through the paid-unload-token ring.

use super::*;
use core::slice;
use frame_support::traits::UnixTime;
use indiv_pallet_coinage::{
	Call as CoinageCall, Config as CoinageConfig, PAID_UNLOAD_TOKEN_CONTEXT_BASE,
	UNLOADING_RECYCLER_CONTEXT,
};
use indiv_support::traits::Alias;
use sp_runtime::bounded_vec;

// Helper function to build the unload extrinsic using a Paid Unload Token
#[allow(clippy::too_many_arguments)]
fn build_unload_paid_ext(
	// Paid token provider info
	paid_token_secret: &VrfSecret,
	paid_token_ring_index: u32,
	period: u32,
	// Secrets being withdrawn from recycler
	recycler_secrets: &[VrfSecret],
	// Recycler info
	value: i8,
	index: u32,
	// The specific call
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());

	// 1. Calculate the inherited implication hash
	let rest_ext_for_implication = (
		(tx_ext.0 .0 .6.clone(), tx_ext.0 .0 .7.clone()),
		tx_ext.0 .1.clone(),
		tx_ext.0 .2.clone(),
		tx_ext.0 .3.clone(),
		tx_ext.0 .4.clone(),
		tx_ext.0 .5.clone(),
		tx_ext.0 .6.clone(),
		tx_ext.0 .7.clone(),
		tx_ext.0 .8.clone(),
		tx_ext.0 .9.clone(),
		tx_ext.0 .10.clone(),
	);

	let encoded_implications = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext_for_implication;
		let implication_implicit = &rest_ext_for_implication.implicit().unwrap();
		(implication_base, implication_explicit, implication_implicit).encode()
	};

	// proven_msg: the hash of the inherited implication alone (signed by alias proofs)
	let proven_msg = sp_io::hashing::blake2_256(&encoded_implications);

	// 2. Generate Alias Proofs (Recycler RingVRF) — must be created before the paid token proof
	// because the paid token proof signs over the alias proofs.
	let mut alias_proofs_vec = Vec::new();
	let ring_members = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
		COINAGE_INSTANCE_ID,
		value,
		index,
	);

	for secret in recycler_secrets.iter() {
		let member = Crypto::member_from_secret(secret);
		let commitment =
			Crypto::open(recycler_ring_size(), &member, ring_members.clone().into_iter())
				.expect("Recycler member should be in the recycler ring");

		// Alias proofs sign the proven_msg (inherited implication hash).
		let (proof, _alias) = Crypto::create(
			commitment,
			secret,
			UNLOADING_RECYCLER_CONTEXT.as_ref(),
			proven_msg.as_ref(),
		)
		.unwrap();

		alias_proofs_vec.push(proof);
	}

	let alias_proofs: frame_support::BoundedVec<_, _> =
		alias_proofs_vec.try_into().expect("Alias proofs exceed MaxConsolidation");

	// intent_msg: hash of (alias_proofs, inherited_implication) — signed by the paid token proof
	// so that alias proofs can't be tampered with after signing.
	let intent_msg =
		sp_io::hashing::blake2_256(&[alias_proofs.encode(), encoded_implications].concat());

	// 3. Generate Paid Unload Token Proof (Payment RingVRF)
	let paid_token_context = {
		let mut c = [0u8; 32];
		c[..28].copy_from_slice(PAID_UNLOAD_TOKEN_CONTEXT_BASE.as_ref());
		c[28..32].copy_from_slice(&period.to_le_bytes());
		c
	};

	let paid_token_member = Crypto::member_from_secret(paid_token_secret);

	// Get members of the Paid Token ring
	let paid_token_members = indiv_pallet_coinage::Pallet::<Runtime>::get_paid_token_ring_members(
		period,
		paid_token_ring_index,
	);

	let commitment = Crypto::open(
		paid_unload_token_ring_size(),
		&paid_token_member,
		paid_token_members.into_iter(),
	)
	.expect("Paid token member should be in the ring");

	// The paid token proof signs the intent_msg (which includes alias_proofs)
	let (paid_token_proof, _) =
		Crypto::create(commitment, paid_token_secret, &paid_token_context[..], &intent_msg[..])
			.unwrap();

	let paid_token_ring_revision =
		indiv_pallet_coinage::Pallet::<Runtime>::get_paid_token_ring_revision(
			period,
			paid_token_ring_index,
		)
		.expect("paid token ring should exist");

	// 4. Construct the AsCoinageInfo
	let info = indiv_pallet_coinage::extension::AsCoinageInfo::AsUnloadTokenPaid {
		proof: paid_token_proof,
		paid_token_ring_revision,
		period,
		paid_token_ring_index,
		alias_proofs,
	};

	// 5. Update the extension (index 0.0.5)
	tx_ext.0 .0 .5 = indiv_pallet_coinage::extension::AsCoinage::<Runtime>::new(Some(info));

	finalize_uxt(call, tx_ext)
}

#[test]
fn coinage_paid_full_story() {
	new_test_ext().execute_with(|| {
		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_external_asset_address = pair_to_account_id(&alice_pair);

		// Values
		let denomination_initial: i8 = 1; // $2
		let denomination_split: i8 = 0; // $1
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount_initial = asset_unit.checked_shl(denomination_initial as u32).unwrap();

		// Fund Alice's external-asset address.
		// She needs enough for the deposit + 3 fees (Initial unload, Consolidation, Offboard).
		// One extra fee of headroom: every fee is swapped through the conversion pool, so each
		// payment moves the price slightly and the later quotes come out a bit above the first.
		let fee_amount: u128 =
			Coinage::get_paid_unload_token_fee_in_asset(COINAGE_INSTANCE_ID).unwrap();
		let min_balance = FungibleExternalAsset::minimum_balance();
		FungibleExternalAsset::mint_into(
			&alice_external_asset_address,
			asset_amount_initial + fee_amount * 4 + min_balance,
		)
		.unwrap();

		// ─────────────────────────────────────
		// Action 1: Initiate Onboarding (Load Recycler)
		// ─────────────────────────────────────

		let alice_recycler_secret_0 = Crypto::new_secret([42u8; 32]);
		let alice_recycler_member_0 = Crypto::member_from_secret(&alice_recycler_secret_0);
		let proof_of_ownership =
			Crypto::sign(&alice_recycler_secret_0, &alice_external_asset_address.encode()).unwrap();

		let load_call = CoinageCall::<Runtime>::load_recycler_with_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
			value: denomination_initial,
			member_key: alice_recycler_member_0,
			proof_of_ownership,
		};
		exec_signed(&alice_pair, load_call.into());

		// Verification
		let (_, r_val) =
			indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(alice_recycler_member_0)
				.unwrap();
		assert_eq!(r_val, denomination_initial);

		// Override onboarding size so the ring can be built with just 1 member
		let recycler_id = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination_initial,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id, 1u32);

		// ─────────────────────────────────────
		// Action 2: Pay for Unload Token
		// ─────────────────────────────────────

		// Alice creates a key for the payment ring
		let alice_payment_secret_1 = Crypto::new_secret([101u8; 32]);
		let alice_payment_member_1 = Crypto::member_from_secret(&alice_payment_secret_1);
		let payment_proof_1 =
			Crypto::sign(&alice_payment_secret_1, &alice_external_asset_address.encode()).unwrap();

		let pay_fee_call =
			CoinageCall::<Runtime>::pay_for_recycler_unload_fee_token_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				member_key: alice_payment_member_1,
				proof_of_ownership: payment_proof_1,
				max_fee: unload_token_fee_in_asset(),
			};
		exec_signed(&alice_pair, pay_fee_call.into());

		// Determine paid token details for unload
		let now_secs = Timestamp::now().as_secs() as u32;
		let period_duration: u32 = <Runtime as CoinageConfig>::PaidUnloadTokenTimePeriod::get();
		let period = now_secs / period_duration;
		// In this sequential test environment, the ring index starts at 0.
		let payment_ring_index = 0;

		// ─────────────────────────────────────
		// Action 3: Wait for Builds (Recycler & Payment Ring)
		// ─────────────────────────────────────

		// Override onboarding size for the paid token collection
		let paid_id =
			indiv_pallet_coinage::Pallet::<Runtime>::paid_token_collection_identifier(period);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(paid_id, 1u32);

		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		// ─────────────────────────────────────
		// Action 4: Unload into Private Coin (using Paid Token)
		// ─────────────────────────────────────

		// Get ring index and revision after build
		let r_idx_0: u32 = 0; // First ring for this value
		let r_rev_0 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination_initial,
			r_idx_0,
		)
		.unwrap();

		let shielded_alice_coin_0_pair = sr25519::Pair::from_seed(&[99u8; 32]);
		let shielded_alice_coin_0 = pair_to_account_id(&shielded_alice_coin_0_pair);

		// Calculate aliases for the payload
		let aliases_vec: Vec<Alias> = slice::from_ref(&alice_recycler_secret_0)
			.iter()
			.map(|secret| {
				Crypto::alias_in_context(secret, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap()
			})
			.collect();

		let unload_call = RuntimeCall::Coinage(
			indiv_pallet_coinage::Call::<Runtime>::unload_recycler_into_coin {
				instance_id: COINAGE_INSTANCE_ID,
				aliases: aliases_vec.try_into().unwrap(),
				value: denomination_initial,
				index: r_idx_0,
				revision: r_rev_0,
				to: shielded_alice_coin_0.clone(),
			},
		);

		let uxt = build_unload_paid_ext(
			&alice_payment_secret_1,
			payment_ring_index,
			period,
			slice::from_ref(&alice_recycler_secret_0),
			denomination_initial,
			r_idx_0,
			unload_call,
		);

		Executive::apply_extrinsic(uxt)
			.expect("transaction is valid")
			.expect("dispatch succeeds");

		// Verify coin created
		let coin0 =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_coin_0).unwrap();
		assert_eq!(coin0.value, denomination_initial);

		// ─────────────────────────────────────
		// Action 5: Split ($2 -> $1 + $1)
		// ─────────────────────────────────────
		let shielded_alice_coin_1_pair = sr25519::Pair::from_seed(&[101u8; 32]);
		let shielded_alice_coin_1 = pair_to_account_id(&shielded_alice_coin_1_pair);
		let shielded_alice_coin_2_pair = sr25519::Pair::from_seed(&[102u8; 32]);
		let shielded_alice_coin_2 = pair_to_account_id(&shielded_alice_coin_2_pair);

		let split_call = CoinageCall::<Runtime>::split {
			split_into: bounded_vec![(
				denomination_split,
				bounded_vec![shielded_alice_coin_1.clone(), shielded_alice_coin_2.clone()],
			)],
		};
		exec_as_coin(&shielded_alice_coin_0_pair, split_call.into());

		// ─────────────────────────────────────
		// Action 6: Recycle & Consolidate (Paid Flow)
		// ─────────────────────────────────────

		let coin_pair_1 = shielded_alice_coin_1_pair;
		let coin_account_1 = pair_to_account_id(&coin_pair_1);
		let coin_pair_2 = shielded_alice_coin_2_pair;
		let coin_account_2 = pair_to_account_id(&coin_pair_2);

		// Load Coin 1
		let alice_recycler_secret_1 = Crypto::new_secret([51u8; 32]);
		let alice_recycler_member_1 = Crypto::member_from_secret(&alice_recycler_secret_1);
		let proof_1 = Crypto::sign(&alice_recycler_secret_1, &coin_account_1.encode()).unwrap();

		exec_as_coin(
			&coin_pair_1,
			CoinageCall::<Runtime>::load_recycler_with_coin {
				member_key: alice_recycler_member_1,
				proof_of_ownership: proof_1,
			}
			.into(),
		);

		// Load Coin 2
		let alice_recycler_secret_2 = Crypto::new_secret([52u8; 32]);
		let alice_recycler_member_2 = Crypto::member_from_secret(&alice_recycler_secret_2);
		let proof_2 = Crypto::sign(&alice_recycler_secret_2, &coin_account_2.encode()).unwrap();

		exec_as_coin(
			&coin_pair_2,
			CoinageCall::<Runtime>::load_recycler_with_coin {
				member_key: alice_recycler_member_2,
				proof_of_ownership: proof_2,
			}
			.into(),
		);

		// Pay for Unload Token (Consolidation)
		let alice_payment_secret_2 = Crypto::new_secret([102u8; 32]);
		let alice_payment_member_2 = Crypto::member_from_secret(&alice_payment_secret_2);
		let payment_proof_2 =
			Crypto::sign(&alice_payment_secret_2, &alice_external_asset_address.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::pay_for_recycler_unload_fee_token_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				member_key: alice_payment_member_2,
				proof_of_ownership: payment_proof_2,
				max_fee: unload_token_fee_in_asset(),
			}
			.into(),
		);

		// Override onboarding size for the denomination_split recycler collection
		let recycler_id_split =
			indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
				COINAGE_INSTANCE_ID,
				denomination_split,
			);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_split, 1u32);

		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		// Unload (Consolidate 2 x $1 -> $2)
		let (_, val_cons) =
			indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(alice_recycler_member_1)
				.unwrap();
		let idx_cons: u32 = 0;
		let rev_cons = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			val_cons,
			idx_cons,
		)
		.unwrap();

		let shielded_consolidated_pair = sr25519::Pair::from_seed(&[150u8; 32]);
		let shielded_consolidated = pair_to_account_id(&shielded_consolidated_pair);

		let recycler_secrets =
			vec![alice_recycler_secret_1.clone(), alice_recycler_secret_2.clone()];
		let aliases_vec: Vec<Alias> = recycler_secrets
			.iter()
			.map(|secret| {
				Crypto::alias_in_context(secret, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap()
			})
			.collect();

		let unload_call = RuntimeCall::Coinage(
			indiv_pallet_coinage::Call::<Runtime>::unload_recycler_into_coin {
				instance_id: COINAGE_INSTANCE_ID,
				aliases: aliases_vec.try_into().unwrap(),
				value: val_cons,
				index: idx_cons,
				revision: rev_cons,
				to: shielded_consolidated.clone(),
			},
		);

		let uxt = build_unload_paid_ext(
			&alice_payment_secret_2,
			payment_ring_index,
			period,
			&recycler_secrets,
			val_cons,
			idx_cons,
			unload_call,
		);
		Executive::apply_extrinsic(uxt)
			.expect("consolidation valid")
			.expect("consolidation success");

		let consolidated_coin =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_consolidated).unwrap();
		assert_eq!(consolidated_coin.value, denomination_initial); // $2

		// ─────────────────────────────────────
		// Action 7: Offboard (Paid Flow)
		// ─────────────────────────────────────

		let final_coin_pair = shielded_consolidated_pair;
		let final_coin_account = pair_to_account_id(&final_coin_pair);

		// Load into Recycler
		let alice_recycler_secret_offboard = Crypto::new_secret([60u8; 32]);
		let alice_recycler_member_offboard =
			Crypto::member_from_secret(&alice_recycler_secret_offboard);
		let proof_offboard =
			Crypto::sign(&alice_recycler_secret_offboard, &final_coin_account.encode()).unwrap();

		exec_as_coin(
			&final_coin_pair,
			CoinageCall::<Runtime>::load_recycler_with_coin {
				member_key: alice_recycler_member_offboard,
				proof_of_ownership: proof_offboard,
			}
			.into(),
		);

		// Pay for Unload Token (Offboard)
		let alice_payment_secret_3 = Crypto::new_secret([103u8; 32]);
		let alice_payment_member_3 = Crypto::member_from_secret(&alice_payment_secret_3);
		let payment_proof_3 =
			Crypto::sign(&alice_payment_secret_3, &alice_external_asset_address.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::pay_for_recycler_unload_fee_token_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				member_key: alice_payment_member_3,
				proof_of_ownership: payment_proof_3,
				max_fee: unload_token_fee_in_asset(),
			}
			.into(),
		);

		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		// Offboard to the external asset
		let (_, val_off) = indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(
			alice_recycler_member_offboard,
		)
		.unwrap();
		let idx_off: u32 = 0;
		let rev_off = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			val_off,
			idx_off,
		)
		.unwrap();

		let aliases_vec: Vec<Alias> = slice::from_ref(&alice_recycler_secret_offboard)
			.iter()
			.map(|secret| {
				Crypto::alias_in_context(secret, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap()
			})
			.collect();

		let unload_call = RuntimeCall::Coinage(
			indiv_pallet_coinage::Call::<Runtime>::unload_recycler_into_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				aliases: aliases_vec.try_into().unwrap(),
				value: val_off,
				index: idx_off,
				revision: rev_off,
				to: alice_external_asset_address.clone(),
				max_fee: 0,
			},
		);

		let uxt = build_unload_paid_ext(
			&alice_payment_secret_3,
			payment_ring_index,
			period,
			slice::from_ref(&alice_recycler_secret_offboard),
			val_off,
			idx_off,
			unload_call,
		);

		let balance_before = FungibleExternalAsset::balance(&alice_external_asset_address);
		Executive::apply_extrinsic(uxt)
			.expect("offboard valid")
			.expect("offboard success");
		let balance_after = FungibleExternalAsset::balance(&alice_external_asset_address);

		assert_eq!(balance_after, balance_before + asset_amount_initial);
	});
}
