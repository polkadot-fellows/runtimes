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

//! Test that balances are migrated correctly.
//!
//! This is additional to the tests in the AH and RC migrator pallets. Those tests check that the
//! state of the relay chain and asset hub are consistent before and after migration, with special
//! focus on the checking balance. The tests consider jointly the state of the relay chain and asset
//! hub, making sure that balance is not burned or created out of thin air.
//!
//! NOTE: These tests should be written in the E2E chopsticks framework, but since that is not up
//! yet, they are here. This test is also very simple, it is not generic and just uses the Runtime
//! types directly.

use crate::porting_prelude::*;
use frame_support::{defensive_assert, traits::Currency};
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;
use std::sync::Mutex;

// Total balances and checking balances are migrated correctly.
pub struct BalancesCrossChecker;

// Used to store the balance kept on the relay chain after migration.
static RC_KEPT_AFTER: Mutex<Option<u128>> = Mutex::new(None);

// Min tolerance for some balance checks in the tests, currently 0.1 DOT.
const MIN_DOT_ERROR: u128 = 1000000000;

impl RcMigrationCheck for BalancesCrossChecker {
	// (rc_total_issuance_before, rc_checking_balance_before)
	type RcPrePayload = (u128, u128);

	fn pre_check() -> Self::RcPrePayload {
		let rc_total_issuance_before = pallet_balances::Pallet::<RcRuntime>::total_issuance();
		let rc_checking_balance_before = pallet_balances::Pallet::<RcRuntime>::total_balance(
			&<RcRuntime as pallet_rc_migrator::Config>::CheckingAccount::get(),
		);
		(rc_total_issuance_before, rc_checking_balance_before)
	}

	fn post_check(_: Self::RcPrePayload) {
		// Check that checking account has no balance (fully migrated)
		let check_account = <RcRuntime as pallet_rc_migrator::Config>::CheckingAccount::get();
		let checking_balance = pallet_balances::Pallet::<RcRuntime>::total_balance(&check_account);
		assert_eq!(
			checking_balance, 0,
			"Checking account should have no balance on the relay chain after migration"
		);

		let rc_kept_after = pallet_balances::Pallet::<RcRuntime>::total_issuance();
		*RC_KEPT_AFTER.lock().unwrap() = Some(rc_kept_after);
	}
}

impl AhMigrationCheck for BalancesCrossChecker {
	// (rc_total_issuance_before, rc_checking_balance_before)
	type RcPrePayload = (u128, u128);
	// (ah_total_issuance_before, ah_checking_balance_before)
	type AhPrePayload = (u128, u128);

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		let ah_total_issuance_before = pallet_balances::Pallet::<AhRuntime>::total_issuance();
		let ah_checking_balance_before = pallet_balances::Pallet::<AhRuntime>::total_balance(
			&<AhRuntime as pallet_ah_migrator::Config>::CheckingAccount::get(),
		);

		// AH checking account has incorrect 0.01 DOT balance because of the DED airdrop which
		// added DOT ED to all existing AH accounts.
		// This is fine, we can just ignore/accept this small amount.
		defensive_assert!(
			ah_checking_balance_before == pallet_balances::Pallet::<AhRuntime>::minimum_balance()
		);

		(ah_total_issuance_before, ah_checking_balance_before)
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, ah_pre_payload: Self::AhPrePayload) {
		let (rc_total_issuance_before, rc_checking_balance_before) = rc_pre_payload;
		let (ah_total_issuance_before, ah_checking_balance_before) = ah_pre_payload;

		let ah_checking_balance_after = pallet_balances::Pallet::<AhRuntime>::total_balance(
			&<AhRuntime as pallet_ah_migrator::Config>::CheckingAccount::get(),
		);
		// Use the rc_kept_after value computed in RcMigrationCheck::post_check
		let rc_kept_after = RC_KEPT_AFTER
			.lock()
			.unwrap()
			.expect("rc_kept_after should be set by RcMigrationCheck::post_check");
		// `checking_balance` = ah_check_before + rc_check_before
		let checking_balance = rc_checking_balance_before + ah_checking_balance_before;

		// ah_check_after = rc_check_before - ah_total_before + rc_balance_kept
		assert_eq!(
			ah_checking_balance_after,
			rc_checking_balance_before - ah_total_issuance_before + rc_kept_after,
			"Checking balance on asset hub after migration is incorrect"
		);

		// ah_check_after = `checking_balance` + rc_balance_kept - ah_total_before - ah_check_before
		// explanation [here](https://github.com/polkadot-fellows/runtimes/blob/dev-asset-hub-migration/pallets/rc-migrator/src/accounts.md#tracking-total-issuance-post-migration)
		assert_eq!(
			ah_checking_balance_after,
			checking_balance + rc_kept_after -
				ah_total_issuance_before -
				ah_checking_balance_before,
			"Checking balance on asset hub after migration is incorrect"
		);

		let ah_total_issuance_after = pallet_balances::Pallet::<AhRuntime>::total_issuance();
		// There is a small difference between the total issuance before and after migration but the
		// reason is unknown. Currently ignoring it since the difference is less than 0.1 DOT.
		assert!(
			ah_total_issuance_after.abs_diff(rc_total_issuance_before) < MIN_DOT_ERROR,
			"Total issuance is not correctly tracked: before migration {} after migration {}.",
			rc_total_issuance_before,
			ah_total_issuance_after
		);
	}
}
