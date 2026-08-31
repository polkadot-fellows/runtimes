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

//! Integration test for the `InfallibleUnpaidSigned` extension with
//! `load_recycler_with_external_asset_unpaid`.
//!
//! Verifies that an account can load a recycler using all of its external-asset
//! balance (Expendable preservation) without paying any fee.

use super::*;

/// Build an extrinsic using the `InfallibleUnpaidSigned` extension.
fn build_infallible_unpaid_ext(who: &sr25519::Pair, call: RuntimeCall) -> UncheckedExtrinsic {
	let mut tx_ext = base_tx_ext(call.clone());

	let who_account = pair_to_account_id(who);
	let nonce = frame_system::Pallet::<Runtime>::account_nonce(&who_account);

	// Set the AsCoinage extension to InfallibleUnpaidSigned.
	tx_ext.0 .0 .5 = indiv_pallet_coinage::extension::AsCoinage::<Runtime>::new(Some(
		indiv_pallet_coinage::extension::AsCoinageInfo::InfallibleUnpaidSigned { nonce },
	));

	// Sign with VerifySignature.
	let rest_ext = (
		(
			tx_ext.0 .0 .2.clone(),
			tx_ext.0 .0 .3.clone(),
			tx_ext.0 .0 .4.clone(),
			tx_ext.0 .0 .5.clone(),
			tx_ext.0 .0 .6.clone(),
			tx_ext.0 .0 .7.clone(),
		),
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

	let msg = {
		let implication_base = (TX_EXT_VERSION, &call);
		let implication_explicit = &rest_ext;
		let implication_implicit = &rest_ext.implicit().unwrap();
		let encoded_implications =
			(implication_base, implication_explicit, implication_implicit).encode();
		sp_io::hashing::blake2_256(&encoded_implications)
	};

	let raw_sig = who.sign(&msg);
	tx_ext.0 .0 .1 = pallet_verify_signature::VerifySignature::<Runtime>::new_with_signature(
		MultiSignature::from(raw_sig),
		who_account,
	);

	finalize_uxt(call, tx_ext)
}

/// An account funded with exactly the asset amount can load a recycler using
/// `Expendable` preservation through the infallible unpaid extension,
/// draining its entire external-asset balance without paying any fee.
#[test]
fn infallible_unpaid_load_expendable_drains_balance() {
	new_test_ext().execute_with(|| {
		advance_block();

		let bob_pair = Sr25519Keyring::Bob.pair();
		let bob_account = pair_to_account_id(&bob_pair);

		let denomination: i8 = 1;
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount = asset_unit.checked_shl(denomination as u32).unwrap();

		// Fund Bob with exactly the asset amount (no extra for existential deposit).
		FungibleExternalAsset::mint_into(&bob_account, asset_amount).unwrap();
		assert_eq!(FungibleExternalAsset::balance(&bob_account), asset_amount);

		// Prepare the recycler member key and proof of ownership.
		let recycler_secret = Crypto::new_secret([99u8; 32]);
		let recycler_member = Crypto::member_from_secret(&recycler_secret);
		let proof_of_ownership = Crypto::sign(&recycler_secret, &bob_account.encode()).unwrap();

		let call = RuntimeCall::Coinage(
			indiv_pallet_coinage::Call::load_recycler_with_external_asset_unpaid {
				instance_id: COINAGE_INSTANCE_ID,
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination,
				member_key: recycler_member,
				proof_of_ownership,
			},
		);

		let uxt = build_infallible_unpaid_ext(&bob_pair, call);
		Executive::apply_extrinsic(uxt)
			.expect("transaction is valid")
			.expect("dispatch succeeds");

		// Recycler loaded.
		assert!(indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::contains_key(
			recycler_member
		),);

		// Entire external-asset balance was consumed, no fee charged.
		assert_eq!(FungibleExternalAsset::balance(&bob_account), 0);
	});
}

/// A batched unpaid load loads several recyclers in one transaction through the real
/// `InfallibleUnpaidSigned` tx-extension tuple, consuming the aggregate cost with no fee.
#[test]
fn infallible_unpaid_load_batch_drains_balance() {
	new_test_ext().execute_with(|| {
		advance_block();

		let bob_pair = Sr25519Keyring::Bob.pair();
		let bob_account = pair_to_account_id(&bob_pair);

		let denomination: i8 = 1;
		let asset_unit: Balance = COINAGE_ASSET_UNIT;
		let asset_amount = asset_unit.checked_shl(denomination as u32).unwrap();

		// Two inner items, each with a distinct member key, funded with exactly the aggregate cost.
		let n = 2u32;
		let total_cost = asset_amount * n as Balance;
		FungibleExternalAsset::mint_into(&bob_account, total_cost).unwrap();

		let members: Vec<_> = (0..n)
			.map(|i| {
				let secret = Crypto::new_secret([i as u8; 32]);
				let member = Crypto::member_from_secret(&secret);
				let proof_of_ownership = Crypto::sign(&secret, &bob_account.encode()).unwrap();
				(member, proof_of_ownership)
			})
			.collect();

		let items = members
			.iter()
			.map(|(member, proof)| indiv_pallet_coinage::UnpaidLoadInput {
				preservation: indiv_pallet_coinage::CodecPreservation::Expendable,
				value: denomination,
				member_key: *member,
				proof_of_ownership: *proof,
			})
			.collect::<Vec<_>>()
			.try_into()
			.expect("two items fit within MaxBatchUnpaidLoad");

		let call = RuntimeCall::Coinage(
			indiv_pallet_coinage::Call::load_recycler_with_external_asset_unpaid_batch {
				instance_id: COINAGE_INSTANCE_ID,
				items,
			},
		);

		let uxt = build_infallible_unpaid_ext(&bob_pair, call);
		Executive::apply_extrinsic(uxt)
			.expect("transaction is valid")
			.expect("dispatch succeeds");

		// Every member key was loaded into a recycler by the single batch transaction.
		for (member, _) in &members {
			assert!(indiv_pallet_coinage::RecyclersCoinToRecycler::<Runtime>::contains_key(member));
		}

		// The aggregate cost of the whole batch was consumed, no fee charged.
		assert_eq!(FungibleExternalAsset::balance(&bob_account), 0);
	});
}
