// Copyright (C) Polkadot Fellows.
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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use migrator_types::PortableHoldReason;

#[test]
fn receive_mints_free_and_holds() {
	new_test_ext().execute_with(|| {
		let alice: AccountId = 1; // fresh account with a deposit
		let bob: AccountId = 3; // already exists here, receives on top

		// GIVEN bob already has a balance here.
		assert_ok!(<Balances as Mutate<AccountId>>::mint_into(&bob, 100));
		let issuance = total_issuance();

		// WHEN a batch arrives.
		CtMigrator::do_receive_accounts(vec![
			portable_account(&alice, 50, vec![(PortableHoldReason::RegistrarDeposit, 500)]),
			portable_account(
				&bob,
				30,
				vec![
					(PortableHoldReason::ProxyDeposit, 70),
					(PortableHoldReason::UnattributedReserve, 5),
				],
			),
		]);

		// THEN each holds exactly what was sent, free and held per reason.
		assert_eq!(free(&alice), 50);
		assert_eq!(held(HoldReason::RegistrarDeposit, &alice), 500);
		assert_eq!(free(&bob), 100 + 30);
		assert_eq!(held(HoldReason::ProxyDeposit, &bob), 70);
		assert_eq!(held(HoldReason::UnattributedReserve, &bob), 5);

		// AND issuance grew by what was minted, which is tracked for the reconciliation.
		assert_eq!(total_issuance(), issuance + 550 + 105);
		assert_eq!(CtMintedTotal::<Test>::get(), 655);
		assert_eq!(
			migrator_events(),
			vec![Event::AccountsReceived { count_good: 2, count_bad: 0 }]
		);
	});
}

/// The existential deposit stays free, so a deposit holder that arrives with less free than
/// that gets its hold placed short.
#[test]
#[should_panic(expected = "Defensive failure")]
fn receive_defends_a_short_hold() {
	new_test_ext().execute_with(|| {
		let alice: AccountId = 1; // a deposit, nothing free

		// GIVEN alice does not exist here. WHEN her free part cannot cover the ED.
		CtMigrator::do_receive_accounts(vec![portable_account(
			&alice,
			0,
			vec![(PortableHoldReason::RegistrarDeposit, 40)],
		)]);
	});
}

#[test]
fn receive_parks_a_bad_account_and_keeps_the_batch() {
	new_test_ext().execute_with(|| {
		let alice: AccountId = 1; // mints fine
		let bob: AccountId = 3; // mint overflows the issuance

		// GIVEN some issuance, so a `u128::MAX` mint overflows.
		assert_ok!(<Balances as Mutate<AccountId>>::mint_into(&alice, 100));
		let bad = portable_account(&bob, u128::MAX, vec![]);

		// WHEN a batch with a bad account arrives. THEN the good one is minted, the bad one is
		// rolled back and parked as received.
		CtMigrator::do_receive_accounts(vec![portable_account(&alice, 60, vec![]), bad.clone()]);
		assert_eq!(free(&alice), 160);
		assert_eq!(FailedAccounts::<Test>::get(bob), Some(bad));
		assert_eq!(free(&bob), 0);
		assert_eq!(frame_system::Pallet::<Test>::providers(&bob), 0);
		assert_eq!(CtMintedTotal::<Test>::get(), 60);
		assert_eq!(
			migrator_events(),
			vec![Event::AccountsReceived { count_good: 1, count_bad: 1 }]
		);
	});
}
