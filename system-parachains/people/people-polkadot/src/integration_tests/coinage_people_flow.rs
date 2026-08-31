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

//! The full coinage story of a recognized person spending free unload tokens.

use super::*;
use core::slice;
use frame_support::traits::UnixTime;
use indiv_pallet_coinage::{
	Call as CoinageCall, Config as CoinageConfig, FREE_UNLOAD_TOKEN_CONTEXT_BASE,
	UNLOADING_RECYCLER_CONTEXT,
};
use indiv_support::traits::Alias;
use sp_runtime::bounded_vec;

struct UnloadRequest<'a> {
	person_secret: &'a VrfSecret,
	person_ring_index: RingIndex,
	period: u32,
	counter: u32,
	recycler_secrets: &'a [VrfSecret],
	value: i8,
	index: u32,
	revision: u32,
}

fn aliases_for_recycler_secrets(recycler_secrets: &[VrfSecret]) -> Vec<Alias> {
	recycler_secrets
		.iter()
		.map(|secret| Crypto::alias_in_context(secret, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap())
		.collect()
}

// Helper function to build the unload extrinsic (common logic for coin and asset destinations)
fn build_unload_common(request: &UnloadRequest<'_>, call: RuntimeCall) -> UncheckedExtrinsic {
	// 1. Construct the base extension
	let mut tx_ext = base_tx_ext(call.clone());

	// 2. Calculate the inherited implication hash (the message to be signed by proofs)
	// This hash is derived from the call and all extensions *except* the one being constructed
	// (AsCoinage). Only the extensions after AsCoinage contribute to its inherited implication.

	let rest_ext_for_implication = (
		(
			tx_ext.0 .0 .6.clone(), // AsResources
			tx_ext.0 .0 .7.clone(), // AuthorizeCall
		),
		tx_ext.0 .1.clone(),  // RestrictOrigin
		tx_ext.0 .2.clone(),  // CheckNonZeroSender
		tx_ext.0 .3.clone(),  // CheckSpecVersion
		tx_ext.0 .4.clone(),  // CheckTxVersion
		tx_ext.0 .5.clone(),  // CheckGenesis
		tx_ext.0 .6.clone(),  // CheckEra
		tx_ext.0 .7.clone(),  // CheckNonce
		tx_ext.0 .8.clone(),  // CheckWeight
		tx_ext.0 .9.clone(),  // ChargeAssetTxPayment
		tx_ext.0 .10.clone(), // CheckMetadataHash
	);

	let encoded_implications = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext_for_implication;
		let implication_implicit = &rest_ext_for_implication.implicit().unwrap();
		(implication_base, implication_explicit, implication_implicit).encode()
	};

	// proven_msg: the hash of the inherited implication alone (signed by alias proofs)
	let proven_msg = sp_io::hashing::blake2_256(&encoded_implications);

	// 3. Generate Alias Proofs (Recycler RingVRF) — must be created before the people proof
	// because the people proof signs over the alias proofs.
	let mut alias_proofs_vec = Vec::new();
	let ring_members = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
		COINAGE_INSTANCE_ID,
		request.value,
		request.index,
	);

	for secret in request.recycler_secrets.iter() {
		let member = Crypto::member_from_secret(secret);
		let commitment =
			Crypto::open(recycler_ring_size(), &member, ring_members.clone().into_iter())
				.expect("Recycler member should be in the recycler ring");

		// Alias proofs sign the proven_msg (inherited implication hash).
		let (proof, _alias) = Crypto::create(
			commitment,
			secret,
			indiv_pallet_coinage::UNLOADING_RECYCLER_CONTEXT.as_ref(),
			proven_msg.as_ref(),
		)
		.unwrap();

		alias_proofs_vec.push(proof);
	}

	let alias_proofs: frame_support::BoundedVec<_, _> =
		alias_proofs_vec.try_into().expect("Alias proofs exceed MaxConsolidation");

	// intent_msg: hash of (alias_proofs, inherited_implication) — signed by the people proof
	// so that alias proofs can't be tampered with after signing.
	let intent_msg =
		sp_io::hashing::blake2_256(&[alias_proofs.encode(), encoded_implications].concat());

	// 4. Generate Unload Token Proof (Personhood RingVRF)
	let unload_token_context = {
		let mut c = [0u8; 32];
		c[..24].copy_from_slice(FREE_UNLOAD_TOKEN_CONTEXT_BASE.as_ref());
		c[24..28].copy_from_slice(&request.period.to_le_bytes());
		c[28..32].copy_from_slice(&request.counter.to_le_bytes());
		c
	};

	// Get members of the Personhood ring
	let person_member = Crypto::member_from_secret(request.person_secret);
	let person_members = indiv_pallet_members::RingKeys::<Runtime>::get((
		indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
		request.person_ring_index,
		0u32,
	));
	if person_members.is_empty() {
		panic!("Personhood ring {} is empty or does not exist.", request.person_ring_index);
	}
	let person_commitment = Crypto::open(
		verifiable::ring::RingDomainSize::Domain11,
		&person_member,
		person_members.into_iter(),
	)
	.expect("Person member should be in the ring");

	// The people proof signs the intent_msg (which includes alias_proofs)
	let (unload_token_vrf_proof, _) = Crypto::create(
		person_commitment,
		request.person_secret,
		&unload_token_context[..],
		&intent_msg[..],
	)
	.unwrap();
	let person_revision = indiv_pallet_members::Root::<Runtime>::get(
		*indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
		request.person_ring_index,
	)
	.map(|root| root.revision)
	.expect("personhood ring root must exist for unload proofs");

	// Wrap it in pallet-people's `MembershipProof`.
	let unload_token_proof = indiv_pallet_people::MembershipProof::<Runtime> {
		proof: unload_token_vrf_proof,
		ring: request.person_ring_index,
		revision: person_revision,
	};

	// 5. Construct the AsCoinageInfo
	let info = indiv_pallet_coinage::extension::AsCoinageInfo::AsUnloadTokenPeople {
		proof: unload_token_proof,
		period: request.period,
		counter: request.counter,
		alias_proofs,
	};

	// 6. Update the extension (index 0.0.5)
	tx_ext.0 .0 .5 = indiv_pallet_coinage::extension::AsCoinage::<Runtime>::new(Some(info));

	finalize_uxt(call, tx_ext)
}

fn build_unload_ext(request: UnloadRequest<'_>, to: AccountId32) -> UncheckedExtrinsic {
	let aliases_vec = aliases_for_recycler_secrets(request.recycler_secrets);

	// Construct the specific call payload
	let call =
		RuntimeCall::Coinage(indiv_pallet_coinage::Call::<Runtime>::unload_recycler_into_coin {
			instance_id: COINAGE_INSTANCE_ID,
			aliases: aliases_vec.clone().try_into().unwrap(),
			value: request.value,
			index: request.index,
			revision: request.revision,
			to,
		});

	build_unload_common(&request, call)
}

fn build_unload_external_asset_ext(
	request: UnloadRequest<'_>,
	to: AccountId32,
) -> UncheckedExtrinsic {
	let aliases_vec = aliases_for_recycler_secrets(request.recycler_secrets);

	// Construct the specific call payload
	let call = RuntimeCall::Coinage(
		indiv_pallet_coinage::Call::<Runtime>::unload_recycler_into_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			aliases: aliases_vec.clone().try_into().unwrap(),
			value: request.value,
			index: request.index,
			revision: request.revision,
			to,
			max_fee: 0,
		},
	);

	build_unload_common(&request, call)
}

// Flow:
// * Alice onboards $4 of the external asset into the coinage system
// * Alice splits $4 coin into two $2 coins
// * Alice sends one $2 coin to Bob
// * Alice ages her remaining $2 coin, splits it into two $1 coins
// * Alice recycles her two $1 coins and consolidates them into one $2 coin
// * Alice offboards her $2 coin back to the external asset in the regular asset system
#[test]
fn coinage_full_story() {
	new_test_ext().execute_with(|| {
		// Define Alice and values
		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_external_asset_address = pair_to_account_id(&alice_pair);
		// Story values: Start with $4 (exponent 2), split to $2 (exponent 1).
		let denomination_initial: i8 = 2;
		let denomination_split: i8 = 1;

		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount_initial = asset_unit.checked_shl(denomination_initial as u32).unwrap();
		let asset_amount_split = asset_unit.checked_shl(denomination_split as u32).unwrap();

		// Fund Alice's external-asset address with the external asset.
		FungibleExternalAsset::mint_into(&alice_external_asset_address, asset_amount_initial)
			.unwrap();

		// Setup Alice as full Person. (Required for free unload tokens)
		let alice_person_secret = Crypto::new_secret([10u8; 32]);
		let alice_person_member = Crypto::member_from_secret(&alice_person_secret);

		// Setup the people pallet environment for Alice to be recognized
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(
			indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
			1,
		);
		DummyDim::reserve_ids(RuntimeOrigin::root(), 3).unwrap();
		DummyDim::recognize_personhood(
			RuntimeOrigin::root(),
			vec![(1, alice_person_member)].try_into().unwrap(),
		)
		.unwrap();
		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();

		let alice_person_ring_index = 0;
		assert!(
			!indiv_pallet_members::RingKeys::<Runtime>::get((
				indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
				alice_person_ring_index,
				0u32
			))
			.is_empty(),
			"Alice person ring should be created"
		);

		// ─────────────────────────────────────
		// Action 1: Initiate Onboarding
		// ─────────────────────────────────────

		// Alice generates a ring-vrf key
		let alice_ring_vrf_key_0_secret = Crypto::new_secret([42u8; 32]);
		let alice_ring_vrf_key_0 = Crypto::member_from_secret(&alice_ring_vrf_key_0_secret);

		// Proof of ownership: Sign the owner's account ID with the member key.
		let proof_of_ownership =
			Crypto::sign(&alice_ring_vrf_key_0_secret, &alice_external_asset_address.encode())
				.unwrap();

		// Alice calls put_asset_in_recycler (load_recycler_with_external_asset)
		let load_call = CoinageCall::<Runtime>::load_recycler_with_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
			value: denomination_initial,
			member_key: alice_ring_vrf_key_0,
			proof_of_ownership,
		};
		exec_signed(&alice_pair, load_call.into());

		// On-chain verification:
		// Asset transferred from alice_external_asset_address to the system account (on hold).
		assert_eq!(FungibleExternalAsset::balance(&alice_external_asset_address), 0);

		// Key put in the recycler pending queue.
		let (_, r_denomination) =
			indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(alice_ring_vrf_key_0)
				.unwrap();
		assert_eq!(r_denomination, denomination_initial);

		// Override onboarding size so the ring can be built with just 1 member.
		let recycler_id = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination_initial,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id, 1u32);

		// ─────────────────────────────────────
		// Action 2: Complete Onboarding
		// ─────────────────────────────────────

		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();

		// Alice generates a keypair (shielded_alice_coin_0).
		let shielded_alice_coin_0_pair = sr25519::Pair::from_seed(&[99u8; 32]);
		let shielded_alice_coin_0 = pair_to_account_id(&shielded_alice_coin_0_pair);

		// Determine UnloadToken parameters
		let now_secs = Timestamp::now().as_secs() as u32;
		let period_duration: u32 =
			<Runtime as CoinageConfig>::UnloadTokenTimePeriodPeopleLitePeople::get();
		let mut period = now_secs / period_duration;
		let mut counter = 0; // First use this period

		// Get ring index and revision after build
		let r_index_1: u32 = 0;
		let r_revision_1 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination_initial,
			r_index_1,
		)
		.unwrap();

		// Alice calls unload_recycler_into_coin
		let uxt = build_unload_ext(
			UnloadRequest {
				person_secret: &alice_person_secret,
				person_ring_index: alice_person_ring_index,
				period,
				counter,
				recycler_secrets: slice::from_ref(&alice_ring_vrf_key_0_secret),
				value: denomination_initial,
				index: r_index_1,
				revision: r_revision_1,
			},
			shielded_alice_coin_0.clone(),
		);

		Executive::apply_extrinsic(uxt)
			.expect("transaction is valid")
			.expect("dispatch succeeds");

		// On-chain verification:
		let coin0 =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_coin_0).unwrap();
		assert_eq!(coin0.value, denomination_initial);
		assert_eq!(coin0.age, 0);

		// ─────────────────────────────────────
		// Action 3: Split
		// ─────────────────────────────────────

		// Generate new shielded accounts (coin-1 and coin-2)
		let shielded_alice_coin_1_pair = sr25519::Pair::from_seed(&[101u8; 32]);
		let shielded_alice_coin_1 = pair_to_account_id(&shielded_alice_coin_1_pair);
		let shielded_alice_coin_2_pair = sr25519::Pair::from_seed(&[102u8; 32]);
		let shielded_alice_coin_2 = pair_to_account_id(&shielded_alice_coin_2_pair);

		let split_call = CoinageCall::<Runtime>::split {
			// Split value 2 into two coins of value 1 ($2)
			split_into: bounded_vec![(
				denomination_split,
				bounded_vec![shielded_alice_coin_1.clone(), shielded_alice_coin_2.clone()],
			)],
		};

		// Execute the split signed by the coin owner (coin-0)
		exec_as_coin(&shielded_alice_coin_0_pair, split_call.into());

		// On-chain verification:
		assert!(
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_coin_0).is_none()
		);
		let coin1 =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_coin_1).unwrap();
		assert_eq!(coin1.value, denomination_split);
		assert_eq!(coin1.age, 1);
		let coin2 =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_coin_2).unwrap();
		assert_eq!(coin2.value, denomination_split);
		assert_eq!(coin2.age, 1);

		// ─────────────────────────────────────
		// Action 4 & 5: Transfer to Bob
		// ─────────────────────────────────────

		// Action 4: Alice sends shielded_alice_coin_1_pair private key to Bob (offchain).

		// Action 5: Bob completes the transfer.
		// Bob generates his shielded account (coin-0)
		let shielded_bob_coin_0_pair = sr25519::Pair::from_seed(&[201u8; 32]);
		let shielded_bob_coin_0 = pair_to_account_id(&shielded_bob_coin_0_pair);

		// Bob calls transfer using the key Alice sent him (shielded_alice_coin_1_pair)
		let transfer_call = CoinageCall::<Runtime>::transfer { to: shielded_bob_coin_0.clone() };

		exec_as_coin(&shielded_alice_coin_1_pair, transfer_call.into());

		// On-chain verification:
		assert!(
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_coin_1).is_none()
		);
		let bob_coin =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_bob_coin_0).unwrap();
		assert_eq!(bob_coin.value, denomination_split);
		assert_eq!(bob_coin.age, 2); // Age was 1, now 2.

		// ─────────────────────────────────────
		// Action 6: Recycle and consolidate (Split -> Recycle -> Consolidate)
		// ─────────────────────────────────────

		// Define the value for the split coins: 1 -> 0 + 0
		let denomination_split_smaller: i8 = denomination_split - 1;

		// 1. (Setup) Split Alice's remaining coin (value=1) into two smaller coins (value=0)
		let shielded_alice_split_1_pair = sr25519::Pair::from_seed(&[141u8; 32]);
		let shielded_alice_split_1 = pair_to_account_id(&shielded_alice_split_1_pair);
		let shielded_alice_split_2_pair = sr25519::Pair::from_seed(&[142u8; 32]);
		let shielded_alice_split_2 = pair_to_account_id(&shielded_alice_split_2_pair);

		let split_call = CoinageCall::<Runtime>::split {
			// Split value 1 into two coins of value 0
			split_into: bounded_vec![(
				denomination_split_smaller,
				bounded_vec![shielded_alice_split_1.clone(), shielded_alice_split_2.clone()]
			)],
		};

		exec_as_coin(&shielded_alice_coin_2_pair, split_call.into());

		let coin_s1 =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_split_1).unwrap();
		assert_eq!(coin_s1.value, denomination_split_smaller);

		// 3. Consolidate the two split coins.

		// Alice generates ring-vrf keys for recycling
		let alice_recycler_secret_3 = Crypto::new_secret([44u8; 32]);
		let alice_recycler_member_3 = Crypto::member_from_secret(&alice_recycler_secret_3);
		let alice_recycler_secret_4 = Crypto::new_secret([45u8; 32]);
		let alice_recycler_member_4 = Crypto::member_from_secret(&alice_recycler_secret_4);

		// Alice calls load_recycler_with_coin for both coins
		let proof_of_ownership_3 =
			Crypto::sign(&alice_recycler_secret_3, &shielded_alice_split_1.encode()).unwrap();
		let load_coin_call_1 = CoinageCall::<Runtime>::load_recycler_with_coin {
			member_key: alice_recycler_member_3,
			proof_of_ownership: proof_of_ownership_3,
		};
		// Use the pairs resulting from the split
		exec_as_coin(&shielded_alice_split_1_pair, load_coin_call_1.into());

		let proof_of_ownership_4 =
			Crypto::sign(&alice_recycler_secret_4, &shielded_alice_split_2.encode()).unwrap();
		let load_coin_call_2 = CoinageCall::<Runtime>::load_recycler_with_coin {
			member_key: alice_recycler_member_4,
			proof_of_ownership: proof_of_ownership_4,
		};
		exec_as_coin(&shielded_alice_split_2_pair, load_coin_call_2.into());

		// Verification
		let (_, val_3) =
			indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(alice_recycler_member_3)
				.unwrap();
		let (_, val_4) =
			indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(alice_recycler_member_4)
				.unwrap();
		assert_eq!(val_3, denomination_split_smaller);
		assert_eq!(val_4, denomination_split_smaller);

		// Override onboarding size for the new recycler collection
		let recycler_id_smaller =
			indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
				COINAGE_INSTANCE_ID,
				denomination_split_smaller,
			);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_smaller, 1u32);

		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();

		// Alice calls unload_recycler_into_coin consolidating the two coins.
		let shielded_alice_coin_consolidated_pair = sr25519::Pair::from_seed(&[150u8; 32]);
		let shielded_alice_coin_consolidated =
			pair_to_account_id(&shielded_alice_coin_consolidated_pair);

		// Get unload token params
		let now_secs = Timestamp::now().as_secs() as u32;
		let current_period = now_secs / period_duration;
		// Update counter based on previous usage in this period
		counter = if current_period == period { counter + 1 } else { 0 };
		period = current_period;

		// Get ring index and revision after build
		let r_idx_3: u32 = 0;
		let r_rev_3 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination_split_smaller,
			r_idx_3,
		)
		.unwrap();

		let uxt = build_unload_ext(
			UnloadRequest {
				person_secret: &alice_person_secret,
				person_ring_index: alice_person_ring_index,
				period,
				counter,
				recycler_secrets: &[
					alice_recycler_secret_3.clone(),
					alice_recycler_secret_4.clone(),
				],
				value: denomination_split_smaller,
				index: r_idx_3,
				revision: r_rev_3,
			},
			shielded_alice_coin_consolidated.clone(),
		);

		Executive::apply_extrinsic(uxt).expect("tx valid").expect("dispatch success");

		// On-chain verification: new coin created (value=1, age=0)
		let consolidated_coin =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&shielded_alice_coin_consolidated)
				.unwrap();
		assert_eq!(consolidated_coin.value, denomination_split);
		assert_eq!(consolidated_coin.age, 0);

		// ─────────────────────────────────────
		// Action 7: Initiate offboarding
		// ─────────────────────────────────────

		let shielded_alice_coin_final_pair = shielded_alice_coin_consolidated_pair.clone();

		// Execution: Load coin into recycler
		let alice_recycler_secret_offboard = Crypto::new_secret([46u8; 32]);
		let alice_recycler_member_offboard =
			Crypto::member_from_secret(&alice_recycler_secret_offboard);

		let shielded_alice_coin_final_account = pair_to_account_id(&shielded_alice_coin_final_pair);
		let proof_of_ownership_offboard = Crypto::sign(
			&alice_recycler_secret_offboard,
			&shielded_alice_coin_final_account.encode(),
		)
		.unwrap();

		let load_coin_call = CoinageCall::<Runtime>::load_recycler_with_coin {
			member_key: alice_recycler_member_offboard,
			proof_of_ownership: proof_of_ownership_offboard,
		};
		exec_as_coin(&shielded_alice_coin_final_pair, load_coin_call.into());

		// Verification
		let (_, c_val_off) = indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(
			alice_recycler_member_offboard,
		)
		.unwrap();
		assert_eq!(c_val_off, denomination_split); // value 1

		// Override onboarding size for the recycler collection (value 1)
		let recycler_id_split =
			indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
				COINAGE_INSTANCE_ID,
				denomination_split,
			);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_split, 1u32);

		// ─────────────────────────────────────
		// Action 8: Complete offboarding
		// ─────────────────────────────────────

		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();

		// Alice calls unload_recycler_into_external_asset to her external-asset address.

		// Get unload token params
		let now_secs = Timestamp::now().as_secs() as u32;
		let current_period = now_secs / period_duration;
		counter = if current_period == period { counter + 1 } else { 0 };

		// Get ring index and revision after build
		let r_idx_off: u32 = 0;
		let r_rev_off = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			c_val_off,
			r_idx_off,
		)
		.unwrap();

		let uxt = build_unload_external_asset_ext(
			UnloadRequest {
				person_secret: &alice_person_secret,
				person_ring_index: alice_person_ring_index,
				period: current_period,
				counter,
				recycler_secrets: slice::from_ref(&alice_recycler_secret_offboard),
				value: c_val_off,
				index: r_idx_off,
				revision: r_rev_off,
			},
			alice_external_asset_address.clone(),
		);

		let alice_external_asset_balance_before =
			FungibleExternalAsset::balance(&alice_external_asset_address);

		Executive::apply_extrinsic(uxt).expect("tx valid").expect("dispatch success");

		// On-chain verification: asset transferred to alice_external_asset_address.
		let alice_external_asset_balance_after =
			FungibleExternalAsset::balance(&alice_external_asset_address);

		// The expected amount transferred is 2^1 * Unit (asset_amount_split).
		let expected_transfer = asset_amount_split;

		assert_eq!(
			alice_external_asset_balance_after,
			alice_external_asset_balance_before + expected_transfer
		);
	});
}

// How the test works:
// * Bootstrap Alice as a recognized person so unload-token proofs are valid.
// * Case A (age == 0): onboard external asset -> unload to coin -> direct offboard succeeds.
// * Case B (age > 0): onboard -> unload -> age the coin -> offboard through recycler unload into
//   external asset (the privacy-preserving route).
// * Case C (age > 0): onboard -> unload -> age the coin -> direct offboard succeeds.
#[test]
fn coinage_direct_offboard_age0_and_age_gt_0_paths() {
	new_test_ext().execute_with(|| {
		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_external_asset_address = pair_to_account_id(&alice_pair);
		let denomination: i8 = 1;

		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let expected_asset_amount = asset_unit.checked_shl(denomination as u32).unwrap();
		let period_duration: u32 =
			<Runtime as CoinageConfig>::UnloadTokenTimePeriodPeopleLitePeople::get();

		// Setup Alice as full Person (required for free unload token path).
		let alice_person_secret = Crypto::new_secret([10u8; 32]);
		let alice_person_member = Crypto::member_from_secret(&alice_person_secret);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(
			indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
			1,
		);
		DummyDim::reserve_ids(RuntimeOrigin::root(), 3).unwrap();
		DummyDim::recognize_personhood(
			RuntimeOrigin::root(),
			vec![(1, alice_person_member)].try_into().unwrap(),
		)
		.unwrap();
		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();

		let alice_person_ring_index = 0;
		assert!(!indiv_pallet_members::RingKeys::<Runtime>::get((
			indiv_pallet_people::PEOPLE_MEMBER_IDENTIFIER,
			alice_person_ring_index,
			0u32
		))
		.is_empty());

		// ---------------------------------------------------------------------
		// Case A: age == 0 direct offboard succeeds.
		// ---------------------------------------------------------------------
		FungibleExternalAsset::mint_into(&alice_external_asset_address, expected_asset_amount)
			.unwrap();

		let recycler_secret_a = Crypto::new_secret([42u8; 32]);
		let recycler_member_a = Crypto::member_from_secret(&recycler_secret_a);
		let proof_a =
			Crypto::sign(&recycler_secret_a, &alice_external_asset_address.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::load_recycler_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination,
				member_key: recycler_member_a,
				proof_of_ownership: proof_a,
			}
			.into(),
		);

		let recycler_id_a = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_a, 1u32);
		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();
		let idx_a: u32 = 0;
		let rev_a = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination,
			idx_a,
		)
		.unwrap();

		let coin_a_pair = sr25519::Pair::from_seed(&[99u8; 32]);
		let coin_a_account = pair_to_account_id(&coin_a_pair);

		let now_secs = Timestamp::now().as_secs() as u32;
		let mut period = now_secs / period_duration;
		let mut counter = 0;

		let unload_a = build_unload_ext(
			UnloadRequest {
				person_secret: &alice_person_secret,
				person_ring_index: alice_person_ring_index,
				period,
				counter,
				recycler_secrets: slice::from_ref(&recycler_secret_a),
				value: denomination,
				index: idx_a,
				revision: rev_a,
			},
			coin_a_account.clone(),
		);
		Executive::apply_extrinsic(unload_a)
			.expect("tx valid")
			.expect("dispatch success");

		let age0_coin =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&coin_a_account).unwrap();
		assert_eq!(age0_coin.age, 0);

		let balance_before_direct = FungibleExternalAsset::balance(&alice_external_asset_address);

		exec_as_coin(
			&coin_a_pair,
			CoinageCall::<Runtime>::direct_offboard_coin_into_external_asset {
				to: alice_external_asset_address.clone(),
			}
			.into(),
		);

		assert!(indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&coin_a_account).is_none());
		assert_eq!(
			FungibleExternalAsset::balance(&alice_external_asset_address),
			balance_before_direct + expected_asset_amount
		);

		// ---------------------------------------------------------------------
		// Case B: age > 0 offboard through the recycler (the privacy-preserving route).
		// ---------------------------------------------------------------------
		FungibleExternalAsset::mint_into(&alice_external_asset_address, expected_asset_amount)
			.unwrap();

		let recycler_secret_b = Crypto::new_secret([43u8; 32]);
		let recycler_member_b = Crypto::member_from_secret(&recycler_secret_b);
		let proof_b =
			Crypto::sign(&recycler_secret_b, &alice_external_asset_address.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::load_recycler_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination,
				member_key: recycler_member_b,
				proof_of_ownership: proof_b,
			}
			.into(),
		);

		let recycler_id_b = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_b, 1u32);
		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();
		let idx_b: u32 = 0;
		let rev_b = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination,
			idx_b,
		)
		.unwrap();

		let coin_b_pair = sr25519::Pair::from_seed(&[100u8; 32]);
		let coin_b_account = pair_to_account_id(&coin_b_pair);

		let now_secs = Timestamp::now().as_secs() as u32;
		let current_period = now_secs / period_duration;
		counter = if current_period == period { counter + 1 } else { 0 };
		period = current_period;

		let unload_b = build_unload_ext(
			UnloadRequest {
				person_secret: &alice_person_secret,
				person_ring_index: alice_person_ring_index,
				period,
				counter,
				recycler_secrets: slice::from_ref(&recycler_secret_b),
				value: denomination,
				index: idx_b,
				revision: rev_b,
			},
			coin_b_account.clone(),
		);
		Executive::apply_extrinsic(unload_b)
			.expect("tx valid")
			.expect("dispatch success");

		// Age this coin once so the recycler offboard operates on a coin with non-zero age.
		let next_pair = sr25519::Pair::from_seed(&[110u8; 32]);
		let aged_account = pair_to_account_id(&next_pair);
		exec_as_coin(
			&coin_b_pair,
			CoinageCall::<Runtime>::transfer { to: aged_account.clone() }.into(),
		);
		let aged_pair = next_pair;

		let aged_coin = indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&aged_account).unwrap();
		assert_eq!(aged_coin.age, 1);

		// Offboard the aged coin through the recycler.
		let recycler_secret_off = Crypto::new_secret([44u8; 32]);
		let recycler_member_off = Crypto::member_from_secret(&recycler_secret_off);
		let proof_off = Crypto::sign(&recycler_secret_off, &aged_account.encode()).unwrap();

		exec_as_coin(
			&aged_pair,
			CoinageCall::<Runtime>::load_recycler_with_coin {
				member_key: recycler_member_off,
				proof_of_ownership: proof_off,
			}
			.into(),
		);

		let (_, v_off) =
			indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(recycler_member_off)
				.unwrap();
		let recycler_id_off =
			indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
				COINAGE_INSTANCE_ID,
				v_off,
			);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_off, 1u32);
		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();
		let idx_off: u32 = 0;
		let rev_off = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			v_off,
			idx_off,
		)
		.unwrap();

		let now_secs = Timestamp::now().as_secs() as u32;
		let current_period = now_secs / period_duration;
		let counter_off = if current_period == period { counter + 1 } else { 0 };

		let balance_before_recycler_off =
			FungibleExternalAsset::balance(&alice_external_asset_address);

		let offboard_uxt = build_unload_external_asset_ext(
			UnloadRequest {
				person_secret: &alice_person_secret,
				person_ring_index: alice_person_ring_index,
				period: current_period,
				counter: counter_off,
				recycler_secrets: slice::from_ref(&recycler_secret_off),
				value: v_off,
				index: idx_off,
				revision: rev_off,
			},
			alice_external_asset_address.clone(),
		);
		Executive::apply_extrinsic(offboard_uxt)
			.expect("tx valid")
			.expect("dispatch success");

		let expected_offboard_amount = asset_unit.checked_shl(v_off as u32).unwrap();
		assert_eq!(
			FungibleExternalAsset::balance(&alice_external_asset_address),
			balance_before_recycler_off + expected_offboard_amount
		);

		// ---------------------------------------------------------------------
		// Case C: age > 0 direct offboard succeeds.
		// ---------------------------------------------------------------------
		FungibleExternalAsset::mint_into(&alice_external_asset_address, expected_asset_amount)
			.unwrap();

		let recycler_secret_c = Crypto::new_secret([45u8; 32]);
		let recycler_member_c = Crypto::member_from_secret(&recycler_secret_c);
		let proof_c =
			Crypto::sign(&recycler_secret_c, &alice_external_asset_address.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::load_recycler_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination,
				member_key: recycler_member_c,
				proof_of_ownership: proof_c,
			}
			.into(),
		);

		let recycler_id_c = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_c, 1u32);
		// Onboarding and ring building happen in separate blocks.
		advance_block();
		advance_block();
		let idx_c: u32 = 0;
		let rev_c = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination,
			idx_c,
		)
		.unwrap();

		let coin_c_pair = sr25519::Pair::from_seed(&[101u8; 32]);
		let coin_c_account = pair_to_account_id(&coin_c_pair);

		let now_secs = Timestamp::now().as_secs() as u32;
		let period_c = now_secs / period_duration;
		let counter_c = if period_c == current_period { counter_off + 1 } else { 0 };

		let unload_c = build_unload_ext(
			UnloadRequest {
				person_secret: &alice_person_secret,
				person_ring_index: alice_person_ring_index,
				period: period_c,
				counter: counter_c,
				recycler_secrets: slice::from_ref(&recycler_secret_c),
				value: denomination,
				index: idx_c,
				revision: rev_c,
			},
			coin_c_account.clone(),
		);
		Executive::apply_extrinsic(unload_c)
			.expect("tx valid")
			.expect("dispatch success");

		// Age this coin once so the direct offboard operates on a coin with non-zero age.
		let aged_c_pair = sr25519::Pair::from_seed(&[111u8; 32]);
		let aged_c_account = pair_to_account_id(&aged_c_pair);
		exec_as_coin(
			&coin_c_pair,
			CoinageCall::<Runtime>::transfer { to: aged_c_account.clone() }.into(),
		);

		let aged_c_coin =
			indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&aged_c_account).unwrap();
		assert_eq!(aged_c_coin.age, 1);

		let balance_before_direct_aged =
			FungibleExternalAsset::balance(&alice_external_asset_address);
		let consumed_tokens_before_direct =
			indiv_pallet_coinage::ConsumedFreeUnloadTokens::<Runtime>::iter_prefix(period_c)
				.count();

		exec_as_coin(
			&aged_c_pair,
			CoinageCall::<Runtime>::direct_offboard_coin_into_external_asset {
				to: alice_external_asset_address.clone(),
			}
			.into(),
		);

		System::assert_has_event(
			indiv_pallet_coinage::Event::<Runtime>::CoinOffboardedIntoExternalAsset {
				instance_id: COINAGE_INSTANCE_ID,
				to: alice_external_asset_address.clone(),
				value: denomination,
				amount: expected_asset_amount,
			}
			.into(),
		);

		// Direct offboarding bypasses recyclers, so it must not consume a free unload token.
		assert_eq!(
			indiv_pallet_coinage::ConsumedFreeUnloadTokens::<Runtime>::iter_prefix(period_c)
				.count(),
			consumed_tokens_before_direct,
		);

		assert!(indiv_pallet_coinage::CoinsByOwner::<Runtime>::get(&aged_c_account).is_none());
		assert_eq!(
			FungibleExternalAsset::balance(&alice_external_asset_address),
			balance_before_direct_aged + expected_asset_amount
		);
	});
}
