// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Test that relay chain multisigs can be re-created on Asset Hub.
//!
//! This is additional to the tests in the AH and RC migrator pallets. Those tests just check that
//! a sample multisig account with non-zero balance on the relay chain is correctly migtated to
//! Asset Hub. Moreover, it checks that the multisig signatories can re-create the same multisig on
//! Asset Hub while preserving its balance.
//!
//! NOTE: These tests should be written in the E2E chopsticks framework, but since that is not up
//! yet, they are here. This test is also very simple, it is not generic and just uses the Runtime
//! types directly.

use frame_support::{pallet_prelude::*, traits::Currency};
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;
use sp_io::hashing::blake2_256;
use sp_runtime::AccountId32;
use std::str::FromStr;

// toggle here if you need "generics" for Kusama or Westend
type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;
type RelayRuntimeOrigin = polkadot_runtime::RuntimeOrigin;
type AssetHubRuntimeOrigin = asset_hub_polkadot_runtime::RuntimeOrigin;
type RelayRuntimeCall = polkadot_runtime::RuntimeCall;
type AssetHubRuntimeCall = asset_hub_polkadot_runtime::RuntimeCall;

/// Multisig accounts created on the relay chain can be re-created on Asset Hub.
///
/// This tests that multisig accounts created on the relay chain and having a non-zero balance are
/// correctly migrated to Asset Hub and the corresponding multisigs can be re-created on Asset Hub.
pub struct MultisigsStillWork;

/// A Multisig account summary.
#[derive(Clone, PartialEq, Eq, RuntimeDebug)]
pub struct MultisigSummary {
	/// The account that created the multisig by depositing the required amount.
	pub depositor: AccountId32,
	/// Other signatories that can sign for the multisig.
	pub other_signatories: Vec<AccountId32>,
	/// Multisig account ID.
	pub multisig_id: AccountId32,
	/// The number of approvals required to execute a call.
	pub threshold: u32,
	/// The call hash when the multisig was created.
	pub call_hash: [u8; 32],
}

impl RcMigrationCheck for MultisigsStillWork {
	// (sample multisig, balance)
	// The sample multisig is created on the relay chain before migration, then it is given a
	// non-zero balance to test that the multisig account is correctly migrated to Asset Hub.
	type RcPrePayload = (MultisigSummary, u128);

	fn pre_check() -> Self::RcPrePayload {
		let multisig_info = Self::create_sample_multisig_rc();
		let balance = 1000000000000;
		// A non-zero balance would force the multisig account to be migrated to Asset Hub.
		Self::transfer_rc_balance(
			multisig_info.depositor.clone(),
			multisig_info.multisig_id.clone(),
			balance,
		);
		(multisig_info, balance)
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload) {
		let (multisig_info, _) = rc_pre_payload;
		assert_eq!(
			pallet_balances::Pallet::<RelayRuntime>::total_balance(&multisig_info.multisig_id),
			0,
			"Sample multisig account should have no balance on the relay chain after migration."
		);
	}
}

impl AhMigrationCheck for MultisigsStillWork {
	// (sample multisig, balance)
	// The sample multisig is created on the relay chain before migration, then it is given a
	// non-zero balance to test that the multisig account is correctly migrated to Asset Hub.
	type RcPrePayload = (MultisigSummary, u128);
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		()
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		let (multisig_info, balance) = rc_pre_payload;
		// Recreating the multisig on Asset Hub should work.
		let call_hash = Self::recreate_multisig_ah(&multisig_info);
		assert!(
			pallet_multisig::Multisigs::<AssetHubRuntime>::contains_key(
				multisig_info.multisig_id.clone(),
				call_hash.clone()
			),
			"Sample multisig should have been correctly recreated on Asset Hub."
		);
		// Check that the multisig balance from the relay chain is preserved..
		assert_eq!(
			pallet_balances::Pallet::<RelayRuntime>::total_balance(&multisig_info.multisig_id),
			balance,
			"Sample multisig account balance should have been migrated to Asset Hub with the correct balance."
		);
		// Remove the multisig from the Asset Hub to avoid messing up with the next tests.
		pallet_multisig::Multisigs::<AssetHubRuntime>::remove(
			multisig_info.multisig_id.clone(),
			call_hash.clone(),
		);
		// Check that the multisig has been effectively removed
		assert!(
			!pallet_multisig::Multisigs::<AssetHubRuntime>::contains_key(
				multisig_info.multisig_id.clone(),
				call_hash.clone()
			),
			"Sample multisig should have been correctly recreated on Asset Hub."
		);
	}
}

impl MultisigsStillWork {
	// Create a sample multisig on the Relay chain.
	fn create_sample_multisig_rc() -> MultisigSummary {
		let basti =
			AccountId32::from_str("13fvj4bNfrTo8oW6U8525soRp6vhjAFLum6XBdtqq9yP22E7").unwrap();
		let shawn =
			AccountId32::from_str("12hAtDZJGt4of3m2GqZcUCVAjZPALfvPwvtUTFZPQUbdX1Ud").unwrap();
		let kian =
			AccountId32::from_str("1eTPAR2TuqLyidmPT9rMmuycHVm9s9czu78sePqg2KHMDrE").unwrap();
		let mut other_signatories = vec![basti.clone(), kian.clone()];
		let mut signatories = vec![shawn.clone(), basti.clone(), kian.clone()];
		signatories.sort();
		other_signatories.sort();
		let multisig_id =
			pallet_multisig::Pallet::<RelayRuntime>::multi_account_id(&signatories, 2);
		// Just a placeholder call to make the multisig valid.
		let call =
			Box::new(RelayRuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
				dest: shawn.clone().into(),
				value: 10000000000,
			}));
		let threshold = 2;
		frame_support::assert_ok!(pallet_multisig::Pallet::<RelayRuntime>::as_multi(
			RelayRuntimeOrigin::signed(shawn.clone()),
			threshold,
			other_signatories.clone(),
			None,
			call.clone(),
			Weight::zero(),
		));
		let call_hash = blake2_256(&call.encode());

		MultisigSummary {
			depositor: shawn.clone(),
			other_signatories,
			multisig_id,
			threshold: threshold as u32,
			call_hash,
		}
	}

	// Transfer balance from a source account to a destination account on the Relay chain.
	fn transfer_rc_balance(source: AccountId32, dest: AccountId32, amount: u128) {
		frame_support::assert_ok!(pallet_balances::Pallet::<RelayRuntime>::transfer_allow_death(
			RelayRuntimeOrigin::signed(source),
			dest.into(),
			amount
		));
	}

	// Recreate a multisig on Asset Hub and return the call hash.
	fn recreate_multisig_ah(multisig_info: &MultisigSummary) -> [u8; 32] {
		// Just a placeholder call to make the multisig valid.
		let call =
			Box::new(AssetHubRuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
				dest: multisig_info.depositor.clone().into(),
				value: 10000000000,
			}));
		// Recreate the multisig on Asset Hub.
		frame_support::assert_ok!(pallet_multisig::Pallet::<AssetHubRuntime>::as_multi(
			AssetHubRuntimeOrigin::signed(multisig_info.depositor.clone()),
			multisig_info.threshold as u16,
			multisig_info.other_signatories.clone(),
			None,
			call.clone(),
			Weight::zero(),
		));
		blake2_256(&call.encode())
	}
}
