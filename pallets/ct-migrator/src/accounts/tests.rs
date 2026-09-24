// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Unit tests for the receiving side of the accounts stage.

use super::*;
use crate::mock::*;
use frame_support::assert_ok;
use migrator_types::PortableHoldReason;
use sp_runtime::AccountId32;

type Receiver = AccountsReceiver<Test>;

#[test]
fn receive_mints_free_and_holds_exactly() {
	new_test_ext().execute_with(|| {
		let alice = acc(1); // regular account: liquid + migrated reserve
		let charlie = acc(3); // already exists locally, receives on top

		// GIVEN charlie already has a local balance.
		<Balances as Mutate<AccountId32>>::mint_into(&charlie, 100).unwrap();
		let ti_before = total_issuance();

		// WHEN a batch arrives with one fresh and one pre-existing account.
		Receiver::receive(vec![
			portable_account(&alice, 50, vec![(PortableHoldReason::RegistrarDeposit, 500)]),
			portable_account(
				&charlie,
				30,
				vec![
					(PortableHoldReason::ProxyDeposit, 70),
					(PortableHoldReason::UnattributedReserve, 5),
				],
			),
		]);

		// THEN each account holds exactly what was sent, split free vs held per reason.
		assert_eq!(free(&alice), 50);
		assert_eq!(held(HoldReason::RegistrarDeposit, &alice), 500);
		assert_eq!(free(&charlie), 100 + 30);
		assert_eq!(held(HoldReason::ProxyDeposit, &charlie), 70);
		assert_eq!(held(HoldReason::UnattributedReserve, &charlie), 5);

		// AND issuance grew by exactly the minted total, which is also tracked for the final
		// reconciliation.
		assert_eq!(total_issuance(), ti_before + 550 + 105);
		assert_eq!(CtMintedTotal::<Test>::get(), 655);
		assert_eq!(
			migrator_events(),
			vec![Event::AccountsReceived { count_good: 2, count_bad: 0 }]
		);
	});
}

#[test]
fn sub_ed_free_keeps_the_ed_and_holds_the_rest() {
	new_test_ext().execute_with(|| {
		let bob = acc(2); // deposit holder whose liquid dust followed the deposit (free < ED)

		// GIVEN bob does not exist. WHEN his free part cannot provide the ED (ED is 10).
		Receiver::receive(vec![portable_account(
			&bob,
			2,
			vec![(PortableHoldReason::RegistrarDeposit, 40)],
		)]);

		// THEN the ED stays free and the hold takes the rest. The account is provided for by its
		// own balance, and nothing was burned.
		assert_eq!(free(&bob), ED);
		assert_eq!(held(HoldReason::RegistrarDeposit, &bob), 32);
		assert_eq!(frame_system::Pallet::<Test>::providers(&bob), 1);
		assert_eq!(frame_system::Pallet::<Test>::consumers(&bob), 1);
		assert_eq!(CtMintedTotal::<Test>::get(), 42);
		assert_eq!(total_issuance(), 42);

		// AND WHEN a later stage releases the migrated reserve, the record is honoured up to what
		// is held and the 8 that stayed free is reported as a shortfall.
		assert_ok!(
			Receiver::release_migrated_deposit(HoldReason::RegistrarDeposit, &bob, 40),
			(32, 8)
		);
		assert_eq!(free(&bob), 42);
		assert_eq!(held(HoldReason::RegistrarDeposit, &bob), 0);
		assert_eq!(total_issuance(), 42);

		// AND a record asking for more than arrived is honoured up to what is held, the rest
		// being reported as a shortfall.
		hypothetically_release_shortfall(&bob);
	});
}

fn hypothetically_release_shortfall(who: &AccountId32) {
	frame_support::hypothetically!({
		Receiver::receive(vec![portable_account(
			who,
			0,
			vec![(PortableHoldReason::RegistrarDeposit, 30)],
		)]);
		assert_ok!(
			Receiver::release_migrated_deposit(HoldReason::RegistrarDeposit, who, 50),
			(30, 20)
		);
		assert_eq!(held(HoldReason::RegistrarDeposit, who), 0);
	});
}

#[test]
fn receive_parks_bad_account_without_poisoning_batch() {
	new_test_ext().execute_with(|| {
		let eve = acc(5); // integrates fine
		let dave = acc(4); // mint overflows total issuance -> must park

		// GIVEN some existing issuance so a u128::MAX mint overflows.
		<Balances as Mutate<AccountId32>>::mint_into(&eve, 100).unwrap();

		let bad = portable_account(&dave, u128::MAX, vec![]);
		Receiver::receive(vec![portable_account(&eve, 60, vec![]), bad.clone()]);

		// THEN the good account integrated and the bad one is parked verbatim. The batch is not
		// refused: one bad record must not strand the good ones, and the migration cannot stop
		// mid-run to deal with it. The parked entry is what makes it recoverable afterwards.
		assert_eq!(free(&eve), 160);
		assert_eq!(FailedAccounts::<Test>::get(&dave), Some(bad));
		assert_eq!(free(&dave), 0, "the failed account must be fully rolled back");
		assert_eq!(frame_system::Pallet::<Test>::providers(&dave), 0);
		assert_eq!(CtMintedTotal::<Test>::get(), 60, "only successful mints are tracked");
		assert_eq!(
			migrator_events(),
			vec![Event::AccountsReceived { count_good: 1, count_bad: 1 }]
		);
	});
}
