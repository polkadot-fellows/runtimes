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

//! Integration tests for the non-anonymous coinage flow.
//!
//! This tests the `unload_recycler_into_external_asset_non_anonymous` extrinsic
//! which allows users to unload from a recycler while paying the fee explicitly
//! (non-anonymously) rather than through the anonymous unload token ring.
//!
//! Also tests the fee-from-output flow where the fee is deducted from the unloaded amount.

use super::*;
use indiv_pallet_coinage::{
	Call as CoinageCall, Config as CoinageConfig, FeeCurrency, UnloadRecyclerInput,
	UNLOADING_RECYCLER_CONTEXT,
};
use indiv_support::traits::Alias;
use sp_io::hashing::blake2_256;
use sp_runtime::bounded_vec;

/// Helper function to build the unload extrinsic using AsUnloadTokenFromOutput.
/// The fee is deducted from the unloaded assets.
///
/// All secrets and their recycler info are provided together. The first secret's recycler
/// is used as the fee recycler (validated in extension for spam protection).
#[allow(clippy::too_many_arguments)]
fn build_unload_fee_from_output_ext(
	// All secrets in order (first one is for fee recycler, validated in extension)
	secrets: &[&VrfSecret],
	// Recycler info for each secret: (value, index) pairs
	recycler_info: &[(i8, u32)],
	// Fee recycler revision (for the first secret's recycler)
	fee_recycler_revision: u32,
	// The specific call
	call: RuntimeCall,
) -> UncheckedExtrinsic {
	assert!(!secrets.is_empty(), "Must have at least one secret");
	assert_eq!(secrets.len(), recycler_info.len(), "Secrets and recycler info must match");

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

	let implication_base = (TX_EXT_VERSION, &call);
	let implication_explicit = &rest_ext_for_implication;
	let implication_implicit = rest_ext_for_implication.implicit().unwrap();
	let inherited_implication = (implication_base, implication_explicit, &implication_implicit);
	let msg_hash = inherited_implication.using_encoded(sp_io::hashing::blake2_256);

	// 2. Generate the other alias proofs (all secrets except the first)
	let mut other_alias_proofs_vec = Vec::new();
	for (secret, (value, index)) in secrets[1..].iter().zip(recycler_info[1..].iter()) {
		let member = Crypto::member_from_secret(secret);
		let ring_members = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
			COINAGE_INSTANCE_ID,
			*value,
			*index,
		);

		let commitment = Crypto::open(recycler_ring_size(), &member, ring_members.into_iter())
			.expect("Recycler member should be in the recycler ring");

		let (proof, _) = Crypto::create(
			commitment,
			secret,
			UNLOADING_RECYCLER_CONTEXT.as_ref(),
			msg_hash.as_ref(),
		)
		.unwrap();

		other_alias_proofs_vec.push(proof);
	}

	// 2b. Generate first alias proof signing the other proofs, retry counter, and inherited
	// implication.
	let (fee_recycler_value, fee_recycler_index) = recycler_info[0];
	let retry_counter = 0u8;
	let intent_msg = (&other_alias_proofs_vec, retry_counter, &inherited_implication)
		.using_encoded(sp_io::hashing::blake2_256);
	let first_alias_proof = {
		let member = Crypto::member_from_secret(secrets[0]);
		let ring_members = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
			COINAGE_INSTANCE_ID,
			fee_recycler_value,
			fee_recycler_index,
		);

		let commitment = Crypto::open(recycler_ring_size(), &member, ring_members.into_iter())
			.expect("Recycler member should be in the recycler ring");

		let (proof, _) = Crypto::create(
			commitment,
			secrets[0],
			UNLOADING_RECYCLER_CONTEXT.as_ref(),
			intent_msg.as_ref(),
		)
		.unwrap();

		proof
	};

	// Combine: first proof + other proofs
	let mut alias_proofs_vec = vec![first_alias_proof];
	alias_proofs_vec.extend(other_alias_proofs_vec);
	let alias_proofs = alias_proofs_vec.try_into().expect("Proofs exceed MaxConsolidation");

	// 3. Construct the AsCoinageInfo
	let info = indiv_pallet_coinage::extension::AsCoinageInfo::AsUnloadTokenFromOutput {
		fee_recycler_value,
		fee_recycler_index,
		fee_recycler_revision,
		retry_counter,
		alias_proofs,
	};

	// 4. Update the extension (index 0.0.5)
	tx_ext.0 .0 .5 = indiv_pallet_coinage::extension::AsCoinage::<Runtime>::new(Some(info));

	finalize_uxt(call, tx_ext)
}

/// Test the non-anonymous unload flow with native fee payment.
///
/// This test demonstrates the full lifecycle:
/// 1. Load the external asset into a recycler
/// 2. Wait for recycler build
/// 3. Unload using non-anonymous method (signer pays fee in native currency)
#[test]
fn coinage_non_anonymous_native_fee() {
	new_test_ext().execute_with(|| {
		// Fund FeeDestination with existential deposit so it can receive native fees
		let fee_dest = <Runtime as CoinageConfig>::FeeDestination::get();
		Balances::mint_into(&fee_dest, ExistentialDeposit::get()).unwrap();

		advance_block();

		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_account = pair_to_account_id(&alice_pair);

		// Values
		let denomination: i8 = 1; // $2
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount = asset_unit.checked_shl(denomination as u32).unwrap();

		// Fund Alice with the external asset (only for loading into recycler, fee is paid in
		// native)
		let min_balance = FungibleExternalAsset::minimum_balance();
		FungibleExternalAsset::mint_into(&alice_account, asset_amount + min_balance).unwrap();

		// Fund Alice with native balance to pay the fee
		let fee_native: Balance = Coinage::get_paid_unload_token_fee_in_native();
		Balances::mint_into(&alice_account, fee_native * 2).unwrap();

		let alice_native_before = Balances::free_balance(&alice_account);

		// ─────────────────────────────────────
		// Action 1: Load into Recycler
		// ─────────────────────────────────────

		let alice_recycler_secret = Crypto::new_secret([42u8; 32]);
		let alice_recycler_member = Crypto::member_from_secret(&alice_recycler_secret);
		let proof_of_ownership =
			Crypto::sign(&alice_recycler_secret, &alice_account.encode()).unwrap();

		let load_call = CoinageCall::<Runtime>::load_recycler_with_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
			value: denomination,
			member_key: alice_recycler_member,
			proof_of_ownership,
		};
		exec_signed(&alice_pair, load_call.into());

		// Verify loading
		let r_val =
			indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::get(alice_recycler_member);
		assert_eq!(r_val, Some((COINAGE_INSTANCE_ID, denomination)));
		let r_idx: u32 = 0;

		// ─────────────────────────────────────
		// Action 2: Wait for Recycler Build
		// ─────────────────────────────────────

		// Override onboarding size so the ring can be built with just 1 member
		let recycler_id = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id, 1u32);
		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		// Verify recycler was built
		let r_rev = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination,
			r_idx,
		)
		.expect("Recycler ring should exist");

		// ─────────────────────────────────────
		// Action 3: Unload using Non-Anonymous Method
		// ─────────────────────────────────────

		// Destination for unloaded funds (use Alice's own account which is already funded)
		let dest_account = alice_account.clone();

		// Get ring members for proof creation
		let ring_members = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
			COINAGE_INSTANCE_ID,
			denomination,
			r_idx,
		);

		// Calculate alias
		let alias: Alias =
			Crypto::alias_in_context(&alice_recycler_secret, &UNLOADING_RECYCLER_CONTEXT[..])
				.unwrap();

		// Build input for unified proven_msg format
		let revision = r_rev;
		let input = UnloadRecyclerInput {
			value: denomination,
			index: r_idx,
			revision,
			aliases: bounded_vec![alias],
		};
		let inputs = vec![input.clone()];

		// Calculate proven_msg (unified format with
		// unload_recyclers_into_external_asset_non_anonymous)
		let proven_msg =
			blake2_256(&(COINAGE_INSTANCE_ID, &inputs, &dest_account, &alice_account).encode());

		// Create proof
		let member = Crypto::member_from_secret(&alice_recycler_secret);
		let commitment = Crypto::open(recycler_ring_size(), &member, ring_members.into_iter())
			.expect("Member should be in ring");
		let (proof, _) = Crypto::create(
			commitment,
			&alice_recycler_secret,
			UNLOADING_RECYCLER_CONTEXT.as_ref(),
			proven_msg.as_ref(),
		)
		.expect("Proof creation should succeed");

		let alias_proofs = vec![proof].try_into().expect("Should fit in bounded vec");

		// Get balances before unload
		let dest_external_asset_before = FungibleExternalAsset::balance(&dest_account);

		// Execute non-anonymous unload with native fee
		let unload_call =
			CoinageCall::<Runtime>::unload_recycler_into_external_asset_non_anonymous {
				instance_id: COINAGE_INSTANCE_ID,
				input,
				alias_proofs,
				to: dest_account.clone(),
				fee_currency: FeeCurrency::Native,
				max_fee: unload_token_fee_in_native(),
			};
		exec_signed(&alice_pair, unload_call.into());

		// ─────────────────────────────────────
		// Verify Results
		// ─────────────────────────────────────

		// dest_external_asset_before was measured AFTER load (alice lost asset_amount)
		// After unload: alice gained asset_amount back
		// So external asset balance increased by asset_amount
		let dest_external_asset_after = FungibleExternalAsset::balance(&dest_account);
		assert_eq!(dest_external_asset_after - dest_external_asset_before, asset_amount);

		// Alice's native balance should have decreased by fee
		let alice_native_after = Balances::free_balance(&alice_account);
		assert!(alice_native_before > alice_native_after, "Native fee should be charged");
	});
}

/// Test the non-anonymous unload flow with external asset fee payment.
#[test]
fn coinage_non_anonymous_external_asset_fee() {
	new_test_ext().execute_with(|| {
		// Fund FeeDestination with minimum external asset balance so it can receive external asset
		// fees
		let fee_dest = <Runtime as CoinageConfig>::FeeDestination::get();
		let min_balance = FungibleExternalAsset::minimum_balance();
		FungibleExternalAsset::mint_into(&fee_dest, min_balance).unwrap();

		advance_block();

		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_account = pair_to_account_id(&alice_pair);

		let denomination: i8 = 2; // $4 (larger than fee)
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount = asset_unit.checked_shl(denomination.max(0) as u32).unwrap();

		// Fund Alice with extra external asset balance for the fee
		let fee_amount: u128 =
			Coinage::get_paid_unload_token_fee_in_asset(COINAGE_INSTANCE_ID).unwrap();
		let min_balance = FungibleExternalAsset::minimum_balance();
		FungibleExternalAsset::mint_into(
			&alice_account,
			asset_amount + fee_amount * 2 + min_balance,
		)
		.unwrap();

		// ─────────────────────────────────────
		// Action 1: Load into Recycler
		// ─────────────────────────────────────

		let alice_recycler_secret = Crypto::new_secret([43u8; 32]);
		let alice_recycler_member = Crypto::member_from_secret(&alice_recycler_secret);
		let proof_of_ownership =
			Crypto::sign(&alice_recycler_secret, &alice_account.encode()).unwrap();

		let load_call = CoinageCall::<Runtime>::load_recycler_with_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
			value: denomination,
			member_key: alice_recycler_member,
			proof_of_ownership,
		};
		exec_signed(&alice_pair, load_call.into());

		let r_idx: u32 = 0;

		// ─────────────────────────────────────
		// Action 2: Wait for Recycler Build
		// ─────────────────────────────────────

		// Override onboarding size so the ring can be built with just 1 member
		let recycler_id = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id, 1u32);
		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		let r_rev = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination,
			r_idx,
		)
		.expect("Recycler ring should exist after build");

		// ─────────────────────────────────────
		// Action 3: Unload with ExternalAsset Fee
		// ─────────────────────────────────────

		let dest_account = alice_account.clone(); // Unload to self

		let ring_members = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
			COINAGE_INSTANCE_ID,
			denomination,
			r_idx,
		);

		let alias: Alias =
			Crypto::alias_in_context(&alice_recycler_secret, &UNLOADING_RECYCLER_CONTEXT[..])
				.unwrap();

		// Build input for unified proven_msg format
		let revision = r_rev;
		let input = UnloadRecyclerInput {
			value: denomination,
			index: r_idx,
			revision,
			aliases: bounded_vec![alias],
		};
		let inputs = vec![input.clone()];

		// Calculate proven_msg (unified format)
		let proven_msg =
			blake2_256(&(COINAGE_INSTANCE_ID, &inputs, &dest_account, &alice_account).encode());

		let member = Crypto::member_from_secret(&alice_recycler_secret);
		let commitment =
			Crypto::open(recycler_ring_size(), &member, ring_members.into_iter()).unwrap();
		let (proof, _) = Crypto::create(
			commitment,
			&alice_recycler_secret,
			UNLOADING_RECYCLER_CONTEXT.as_ref(),
			proven_msg.as_ref(),
		)
		.unwrap();

		let alias_proofs = bounded_vec![proof];

		let alice_external_asset_before = FungibleExternalAsset::balance(&alice_account);

		// Execute non-anonymous unload with external asset fee
		let unload_call =
			CoinageCall::<Runtime>::unload_recycler_into_external_asset_non_anonymous {
				instance_id: COINAGE_INSTANCE_ID,
				input,
				alias_proofs,
				to: dest_account.clone(),
				fee_currency: FeeCurrency::ExternalAsset,
				max_fee: unload_token_fee_in_asset(),
			};
		exec_signed(&alice_pair, unload_call.into());

		// Alice received asset_amount back but paid fee_amount (in external asset)
		// Net change = asset_amount - fee_amount (could be negative if fee > asset)
		let alice_external_asset_after = FungibleExternalAsset::balance(&alice_account);
		if asset_amount >= fee_amount {
			assert_eq!(
				alice_external_asset_after - alice_external_asset_before,
				asset_amount - fee_amount
			);
		} else {
			// Fee is larger than asset amount (due to test config), so balance decreased
			assert_eq!(
				alice_external_asset_before - alice_external_asset_after,
				fee_amount - asset_amount
			);
		}
	});
}

/// Test non-anonymous unload from multiple recyclers (consolidation).
#[test]
fn coinage_non_anonymous_multi_recycler() {
	new_test_ext().execute_with(|| {
		// Fund FeeDestination with existential deposit so it can receive native fees
		let fee_dest = <Runtime as CoinageConfig>::FeeDestination::get();
		Balances::mint_into(&fee_dest, ExistentialDeposit::get()).unwrap();

		advance_block();

		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_account = pair_to_account_id(&alice_pair);

		// Load two coins of different values
		let denomination_1: i8 = 0; // $1
		let denomination_2: i8 = 1; // $2
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount_1 = asset_unit; // $1
		let asset_amount_2 = asset_unit * 2; // $2
		let total_amount = asset_amount_1 + asset_amount_2; // $3

		// Fund Alice with the external asset (only for loading into recyclers, fee is paid in
		// native)
		let min_balance = FungibleExternalAsset::minimum_balance();
		FungibleExternalAsset::mint_into(&alice_account, total_amount + min_balance).unwrap();

		// Fund Alice with native balance to pay the fee
		let fee_native: Balance = Coinage::get_paid_unload_token_fee_in_native();
		Balances::mint_into(&alice_account, fee_native * 2).unwrap();

		// ─────────────────────────────────────
		// Action 1: Load Two Coins into Recyclers
		// ─────────────────────────────────────

		// First coin ($1)
		let alice_secret_1 = Crypto::new_secret([44u8; 32]);
		let alice_member_1 = Crypto::member_from_secret(&alice_secret_1);
		let proof_1 = Crypto::sign(&alice_secret_1, &alice_account.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::load_recycler_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination_1,
				member_key: alice_member_1,
				proof_of_ownership: proof_1,
			}
			.into(),
		);

		let idx_1: u32 = 0;

		// Second coin ($2)
		let alice_secret_2 = Crypto::new_secret([45u8; 32]);
		let alice_member_2 = Crypto::member_from_secret(&alice_secret_2);
		let proof_2 = Crypto::sign(&alice_secret_2, &alice_account.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::load_recycler_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination_2,
				member_key: alice_member_2,
				proof_of_ownership: proof_2,
			}
			.into(),
		);

		let idx_2: u32 = 0;

		// ─────────────────────────────────────
		// Action 2: Wait for Recycler Builds
		// ─────────────────────────────────────

		// Override onboarding size so the ring can be built with just 1 member
		let recycler_id_1 = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination_1,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_1, 1u32);
		let recycler_id_2 = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination_2,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_2, 1u32);
		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		let rev_1 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination_1,
			idx_1,
		)
		.expect("Recycler 1 ring should exist");
		let rev_2 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination_2,
			idx_2,
		)
		.expect("Recycler 2 ring should exist");

		// ─────────────────────────────────────
		// Action 3: Unload Both Recyclers in One Call
		// ─────────────────────────────────────

		// Use Alice's own account as destination (already funded)
		let dest_account = alice_account.clone();

		// Build inputs
		let alias_1: Alias =
			Crypto::alias_in_context(&alice_secret_1, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap();
		let alias_2: Alias =
			Crypto::alias_in_context(&alice_secret_2, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap();

		let inputs: BoundedVec<_, <Runtime as CoinageConfig>::MaxConsolidation> = bounded_vec![
			indiv_pallet_coinage::UnloadRecyclerInput {
				value: denomination_1,
				index: idx_1,
				revision: rev_1,
				aliases: bounded_vec![alias_1],
			},
			indiv_pallet_coinage::UnloadRecyclerInput {
				value: denomination_2,
				index: idx_2,
				revision: rev_2,
				aliases: bounded_vec![alias_2],
			},
		];

		// Compute proven_msg for multi-recycler call
		let proven_msg =
			blake2_256(&(COINAGE_INSTANCE_ID, &inputs, &dest_account, &alice_account).encode());

		// Create proofs
		let ring_members_1 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
			COINAGE_INSTANCE_ID,
			denomination_1,
			idx_1,
		);
		let member_1 = Crypto::member_from_secret(&alice_secret_1);
		let commitment_1 =
			Crypto::open(recycler_ring_size(), &member_1, ring_members_1.into_iter()).unwrap();
		let (proof_1, _) = Crypto::create(
			commitment_1,
			&alice_secret_1,
			UNLOADING_RECYCLER_CONTEXT.as_ref(),
			proven_msg.as_ref(),
		)
		.unwrap();

		let ring_members_2 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_members(
			COINAGE_INSTANCE_ID,
			denomination_2,
			idx_2,
		);
		let member_2 = Crypto::member_from_secret(&alice_secret_2);
		let commitment_2 =
			Crypto::open(recycler_ring_size(), &member_2, ring_members_2.into_iter()).unwrap();
		let (proof_2, _) = Crypto::create(
			commitment_2,
			&alice_secret_2,
			UNLOADING_RECYCLER_CONTEXT.as_ref(),
			proven_msg.as_ref(),
		)
		.unwrap();

		let alias_proofs = bounded_vec![proof_1, proof_2];

		let dest_external_asset_before = FungibleExternalAsset::balance(&dest_account);

		// Execute multi-recycler non-anonymous unload
		let unload_call =
			CoinageCall::<Runtime>::unload_recyclers_into_external_asset_non_anonymous {
				instance_id: COINAGE_INSTANCE_ID,
				inputs,
				alias_proofs,
				to: dest_account.clone(),
				fee_currency: FeeCurrency::Native,
				// One fee per recycler, and this call unloads two.
				max_fee: unload_token_fee_in_native() * 2,
			};
		exec_signed(&alice_pair, unload_call.into());

		// Verify destination received total amount ($1 + $2 = $3)
		let dest_external_asset_after = FungibleExternalAsset::balance(&dest_account);
		assert_eq!(dest_external_asset_after - dest_external_asset_before, total_amount);
	});
}

/// Test the fee-from-output flow where the fee is deducted from the unloaded assets.
///
/// This uses the `AsUnloadTokenFromOutput` extension to validate the first proof in the
/// extension (spam protection) and deduct the fee from the output.
#[test]
fn coinage_fee_from_output() {
	new_test_ext().execute_with(|| {
		// Fund FeeDestination with minimum external asset balance so it can receive fees
		let fee_dest = <Runtime as CoinageConfig>::FeeDestination::get();
		let min_balance = FungibleExternalAsset::minimum_balance();
		FungibleExternalAsset::mint_into(&fee_dest, min_balance).unwrap();

		advance_block();

		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_account = pair_to_account_id(&alice_pair);

		// Use a larger denomination to ensure it covers the fee: the unload value must exceed the
		// fee in asset (~0.156 HOLLAR on Polkadot, with `COINAGE_ASSET_UNIT` = 0.01 HOLLAR), and
		// the runtime additionally enforces `MinimumExponentForOutputUnloadFee` (4).
		let denomination: i8 = 5; // 0.32 HOLLAR, ~2x the fee
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount = asset_unit.checked_shl(denomination.max(0) as u32).unwrap();
		let fee_amount: u128 =
			Coinage::get_paid_unload_token_fee_in_asset(COINAGE_INSTANCE_ID).unwrap();

		// Fund Alice with the external asset for loading
		FungibleExternalAsset::mint_into(&alice_account, asset_amount + min_balance).unwrap();

		// ─────────────────────────────────────
		// Action 1: Load into Recycler
		// ─────────────────────────────────────

		let alice_recycler_secret = Crypto::new_secret([50u8; 32]);
		let alice_recycler_member = Crypto::member_from_secret(&alice_recycler_secret);
		let proof_of_ownership =
			Crypto::sign(&alice_recycler_secret, &alice_account.encode()).unwrap();

		let load_call = CoinageCall::<Runtime>::load_recycler_with_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
			value: denomination,
			member_key: alice_recycler_member,
			proof_of_ownership,
		};
		exec_signed(&alice_pair, load_call.into());

		let r_idx: u32 = 0;

		// ─────────────────────────────────────
		// Action 2: Wait for Recycler Build
		// ─────────────────────────────────────

		// Override onboarding size so the ring can be built with just 1 member
		let recycler_id = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id, 1u32);
		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		let revision = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination,
			r_idx,
		)
		.expect("Recycler ring should exist after build");

		// ─────────────────────────────────────
		// Action 3: Unload using Fee-From-Output Method
		// ─────────────────────────────────────

		let dest_account = alice_account.clone();

		// Calculate alias for call parameter
		let alias: Alias =
			Crypto::alias_in_context(&alice_recycler_secret, &UNLOADING_RECYCLER_CONTEXT[..])
				.unwrap();
		let aliases = bounded_vec![alias];

		let dest_external_asset_before = FungibleExternalAsset::balance(&dest_account);
		let pool = fee_conversion_pool_account();
		let pool_asset_before = FungibleExternalAsset::balance(&pool);
		let fee_dest_native_before = Balances::free_balance(&fee_dest);

		// Build the unload call
		let unload_call = CoinageCall::<Runtime>::unload_recycler_into_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			aliases,
			value: denomination,
			index: r_idx,
			revision,
			to: dest_account.clone(),
			max_fee: unload_token_fee_in_asset(),
		};

		// Build and execute the transaction with fee-from-output extension
		let uxt = build_unload_fee_from_output_ext(
			&[&alice_recycler_secret], // All secrets (just one)
			&[(denomination, r_idx)],  // Recycler info for each secret
			revision,                  // Fee recycler revision
			unload_call.into(),
		);
		Executive::apply_extrinsic(uxt)
			.expect("transaction is valid")
			.expect("dispatch succeeds");

		// ─────────────────────────────────────
		// Verify Results
		// ─────────────────────────────────────

		// The asset the fee cost went into the pool, and the fee destination was paid in native.
		assert_eq!(FungibleExternalAsset::balance(&pool) - pool_asset_before, fee_amount);
		assert_eq!(
			Balances::free_balance(&fee_dest) - fee_dest_native_before,
			Coinage::get_paid_unload_token_fee_in_native()
		);

		// Destination should receive (asset_amount - fee)
		let dest_external_asset_after = FungibleExternalAsset::balance(&dest_account);
		assert_eq!(
			dest_external_asset_after - dest_external_asset_before,
			asset_amount - fee_amount
		);
	});
}

/// Test the fee-from-output flow with multiple recyclers (consolidation).
///
/// This uses the `AsUnloadTokenFromOutput` extension to validate the first proof in the
/// extension (spam protection) and deduct the fee from the combined output of multiple recyclers.
#[test]
fn coinage_fee_from_output_multi_recycler() {
	new_test_ext().execute_with(|| {
		// Fund FeeDestination with minimum external asset balance so it can receive fees
		let fee_dest = <Runtime as CoinageConfig>::FeeDestination::get();
		let min_balance = FungibleExternalAsset::minimum_balance();
		FungibleExternalAsset::mint_into(&fee_dest, min_balance).unwrap();

		advance_block();

		let alice_pair = Sr25519Keyring::Alice.pair();
		let alice_account = pair_to_account_id(&alice_pair);

		// Load two coins of different values. Each unloaded value must exceed the fee in asset
		// (~0.156 HOLLAR on Polkadot) and satisfy `MinimumExponentForOutputUnloadFee` (4).
		let denomination_1: i8 = 5; // 0.32 HOLLAR
		let denomination_2: i8 = 6; // 0.64 HOLLAR
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount_1 = asset_unit.checked_shl(denomination_1.max(0) as u32).unwrap();
		let asset_amount_2 = asset_unit.checked_shl(denomination_2.max(0) as u32).unwrap();
		let total_amount = asset_amount_1 + asset_amount_2;

		// Fund Alice with the external asset for loading both coins
		FungibleExternalAsset::mint_into(&alice_account, total_amount + min_balance).unwrap();

		// ─────────────────────────────────────
		// Action 1: Load Two Coins into Recyclers
		// ─────────────────────────────────────

		// First coin ($2)
		let alice_secret_1 = Crypto::new_secret([60u8; 32]);
		let alice_member_1 = Crypto::member_from_secret(&alice_secret_1);
		let proof_1 = Crypto::sign(&alice_secret_1, &alice_account.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::load_recycler_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination_1,
				member_key: alice_member_1,
				proof_of_ownership: proof_1,
			}
			.into(),
		);

		let idx_1: u32 = 0;

		// Second coin ($4)
		let alice_secret_2 = Crypto::new_secret([61u8; 32]);
		let alice_member_2 = Crypto::member_from_secret(&alice_secret_2);
		let proof_2 = Crypto::sign(&alice_secret_2, &alice_account.encode()).unwrap();

		exec_signed(
			&alice_pair,
			CoinageCall::<Runtime>::load_recycler_with_external_asset {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination_2,
				member_key: alice_member_2,
				proof_of_ownership: proof_2,
			}
			.into(),
		);

		let idx_2: u32 = 0;

		// ─────────────────────────────────────
		// Action 2: Wait for Recycler Builds
		// ─────────────────────────────────────

		// Override onboarding size so the ring can be built with just 1 member
		let recycler_id_1 = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination_1,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_1, 1u32);
		let recycler_id_2 = indiv_pallet_coinage::Pallet::<Runtime>::recycler_collection_identifier(
			COINAGE_INSTANCE_ID,
			denomination_2,
		);
		indiv_pallet_members::OnboardingSize::<Runtime>::insert(recycler_id_2, 1u32);
		// Onboarding and ring building happen in separate blocks
		advance_block();
		advance_block();

		let revision_1 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination_1,
			idx_1,
		)
		.expect("Recycler 1 ring should exist");
		let revision_2 = indiv_pallet_coinage::Pallet::<Runtime>::get_recycler_ring_revision(
			COINAGE_INSTANCE_ID,
			denomination_2,
			idx_2,
		)
		.expect("Recycler 2 ring should exist");

		// ─────────────────────────────────────
		// Action 3: Unload Both Recyclers with Fee-From-Output (Separate Calls)
		// ─────────────────────────────────────

		let dest_account = alice_account.clone();

		let dest_external_asset_before = FungibleExternalAsset::balance(&dest_account);
		let pool = fee_conversion_pool_account();
		let pool_asset_before = FungibleExternalAsset::balance(&pool);
		let fee_dest_native_before = Balances::free_balance(&fee_dest);

		// ── Unload First Recycler ──
		// Each fee is swapped through the conversion pool, so it must be quoted right before its
		// own unload: the first swap moves the price the second quote is made at.
		let fee_amount_1: u128 =
			Coinage::get_paid_unload_token_fee_in_asset(COINAGE_INSTANCE_ID).unwrap();
		let alias_1: Alias =
			Crypto::alias_in_context(&alice_secret_1, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap();

		let unload_call_1 = CoinageCall::<Runtime>::unload_recycler_into_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			aliases: bounded_vec![alias_1],
			value: denomination_1,
			index: idx_1,
			revision: revision_1,
			to: dest_account.clone(),
			max_fee: unload_token_fee_in_asset(),
		};

		let uxt_1 = build_unload_fee_from_output_ext(
			&[&alice_secret_1],
			&[(denomination_1, idx_1)],
			revision_1,
			unload_call_1.into(),
		);
		Executive::apply_extrinsic(uxt_1)
			.expect("transaction is valid")
			.expect("dispatch succeeds");

		// ── Unload Second Recycler ──
		let fee_amount_2: u128 =
			Coinage::get_paid_unload_token_fee_in_asset(COINAGE_INSTANCE_ID).unwrap();
		let alias_2: Alias =
			Crypto::alias_in_context(&alice_secret_2, &UNLOADING_RECYCLER_CONTEXT[..]).unwrap();

		let unload_call_2 = CoinageCall::<Runtime>::unload_recycler_into_external_asset {
			instance_id: COINAGE_INSTANCE_ID,
			aliases: bounded_vec![alias_2],
			value: denomination_2,
			index: idx_2,
			revision: revision_2,
			to: dest_account.clone(),
			max_fee: unload_token_fee_in_asset(),
		};

		let uxt_2 = build_unload_fee_from_output_ext(
			&[&alice_secret_2],
			&[(denomination_2, idx_2)],
			revision_2,
			unload_call_2.into(),
		);
		Executive::apply_extrinsic(uxt_2)
			.expect("transaction is valid")
			.expect("dispatch succeeds");

		// ─────────────────────────────────────
		// Verify Results
		// ─────────────────────────────────────

		// The asset both fees cost went into the pool, and the fee destination was paid in native.
		let fees = fee_amount_1 + fee_amount_2;
		assert_eq!(FungibleExternalAsset::balance(&pool) - pool_asset_before, fees);
		assert_eq!(
			Balances::free_balance(&fee_dest) - fee_dest_native_before,
			Coinage::get_paid_unload_token_fee_in_native() * 2
		);

		// Destination should receive the total unloaded amount minus both fees.
		let dest_external_asset_after = FungibleExternalAsset::balance(&dest_account);
		assert_eq!(dest_external_asset_after - dest_external_asset_before, total_amount - fees);
	});
}
