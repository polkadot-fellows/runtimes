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
//! a sample multisig account with non-zero balance on the relay chain is correctly migrated to
//! Asset Hub. Moreover, it checks that the multisig signatories can re-create the same multisig on
//! Asset Hub while preserving its balance.
//!
//! NOTE: These tests should be written in the E2E chopsticks framework, but since that is not up
//! yet, they are here. This test is also very simple, it is not generic and just uses the Runtime
//! types directly.

use crate::porting_prelude::*;

use frame_support::{pallet_prelude::*, traits::Currency};
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;
use sp_application_crypto::Ss58Codec;
use sp_io::hashing::blake2_256;
use sp_runtime::AccountId32;
use std::str::FromStr;

/// Multisig accounts created on the relay chain can be re-created on Asset Hub.
///
/// This tests that multisig accounts created on the relay chain and having a non-zero balance are
/// correctly migrated to Asset Hub and the corresponding multisigs can be re-created on Asset Hub.
pub struct MultisigsAccountIdStaysTheSame;

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

impl RcMigrationCheck for MultisigsAccountIdStaysTheSame {
	// sample multisig info
	// The sample multisig is created on the relay chain before migration.
	type RcPrePayload = MultisigSummary;

	fn pre_check() -> Self::RcPrePayload {
		// Create a sample multisig on the relay chain.
		let multisig_info = Self::create_sample_multisig_rc();

		assert!(
			pallet_multisig::Multisigs::<RcRuntime>::contains_key(
				multisig_info.multisig_id.clone(),
				multisig_info.call_hash
			),
			"Sample multisig {:?} should have been correctly created on the relay chain.",
			multisig_info.multisig_id.clone().to_ss58check()
		);

		multisig_info
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload) {
		let multisig_info = rc_pre_payload;
		assert!(
			!pallet_multisig::Multisigs::<RcRuntime>::contains_key(
				multisig_info.depositor.clone(),
				multisig_info.call_hash
			),
			"Sample multisig {:?} should have been removed from the relay chain after migration.",
			multisig_info.multisig_id.clone().to_ss58check()
		);
	}
}

impl AhMigrationCheck for MultisigsAccountIdStaysTheSame {
	// sample multisig info
	// The sample multisig is created on the relay chain before migration and then recreated on
	// Asset Hub. We need to check that the multisig account ID stays the same.
	type RcPrePayload = MultisigSummary;
	type AhPrePayload = ();

	fn pre_check(rc_pre_payload: Self::RcPrePayload) -> Self::AhPrePayload {
		let multisig_info = rc_pre_payload;
		assert!(
			!pallet_multisig::Multisigs::<AhRuntime>::contains_key(
				multisig_info.depositor.clone(),
				multisig_info.call_hash
			),
			"Sample multisig {:?} should not be present on Asset Hub before migration.",
			multisig_info.multisig_id.clone().to_ss58check()
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		let multisig_info = rc_pre_payload;
		// Recreating the multisig on Asset Hub should work.
		let call_hash = Self::recreate_multisig_ah(&multisig_info);
		assert!(
			pallet_multisig::Multisigs::<AhRuntime>::contains_key(
				multisig_info.multisig_id.clone(),
				call_hash
			),
			"Sample multisig {:?} should have been correctly re-created on Asset Hub.",
			multisig_info.multisig_id.clone().to_ss58check()
		);
		// Remove the multisig from the Asset Hub to avoid messing up with the next tests.
		pallet_multisig::Multisigs::<AhRuntime>::remove(
			multisig_info.multisig_id.clone(),
			call_hash,
		);
		// Check that the multisig has been effectively removed
		assert!(
			!pallet_multisig::Multisigs::<AhRuntime>::contains_key(
				multisig_info.multisig_id.clone(),
				call_hash
			),
			"Sample multisig {:?} should have been correctly removed from Asset Hub after tests.",
			multisig_info.multisig_id.clone().to_ss58check()
		);
	}
}

impl MultisigsAccountIdStaysTheSame {
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
		signatories.iter().for_each(Self::fund_account);

		let multisig_id = pallet_multisig::Pallet::<RcRuntime>::multi_account_id(&signatories, 2);
		// Just a placeholder call to make the multisig valid.
		let call = Box::new(RcRuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
			dest: shawn.clone().into(),
			value: 1,
		}));
		let threshold = 2;
		frame_support::assert_ok!(pallet_multisig::Pallet::<RcRuntime>::as_multi(
			RcRuntimeOrigin::signed(shawn.clone()),
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

	// Recreate a multisig on Asset Hub and return the call hash.
	fn recreate_multisig_ah(multisig_info: &MultisigSummary) -> [u8; 32] {
		// Just a placeholder call to make the multisig valid.
		let call = Box::new(AhRuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
			dest: multisig_info.depositor.clone().into(),
			value: 1,
		}));
		// Recreate the multisig on Asset Hub.
		frame_support::assert_ok!(pallet_multisig::Pallet::<AhRuntime>::as_multi(
			AhRuntimeOrigin::signed(multisig_info.depositor.clone()),
			multisig_info.threshold as u16,
			multisig_info.other_signatories.clone(),
			None,
			call.clone(),
			Weight::zero(),
		));
		blake2_256(&call.encode())
	}

	fn fund_account(account: &AccountId32) {
		// Amount does not mater, just deposit a lot
		let _ = pallet_balances::Pallet::<AhRuntime>::deposit_creating(
			account,
			10_000_000_000_000_000_000,
		);
	}
}
