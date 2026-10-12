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
use crate::{
	mock::*, BlockLimits, Error, Event, Limits, MigratedBalances, MigrationStage,
	RcMigratedBalance, RcMigrationStage,
};
use frame_support::{assert_noop, assert_ok};
use migrator_types::sibling_account;
use sp_runtime::{AccountId32, DispatchError::BadOrigin};
use xcm::prelude::*;

type Migrator = AccountsMigrator<Test>;
type Stage = MigrationStage<AccountId, u64, u64>;

fn assert_stage(expected: Stage) {
	assert_eq!(RcMigrationStage::<Test>::get(), expected);
}

fn ah_dest() -> Location {
	Location::new(0, [Parachain(AH_PARA_ID)])
}

/// The instruction that credits `who` on Asset Hub, as `send_teleport` builds it.
fn deposit(who: AccountId, amount: u128) -> Instruction<()> {
	DepositAsset {
		assets: AssetFilter::Definite((Location::parent(), amount).into()),
		beneficiary: Location::new(0, [Junction::AccountId32 { network: None, id: who.into() }]),
	}
}

/// How many accounts each teleport sent so far credits: one `DepositAsset` per account.
fn accounts_per_message() -> Vec<usize> {
	sent()
		.iter()
		.map(|(dest, message)| {
			assert_eq!(*dest, ah_dest());
			message.0.iter().filter(|i| matches!(i, DepositAsset { .. })).count()
		})
		.collect()
}

/// `(count, amount)` of every `AccountsTeleported` event so far.
fn teleported_events() -> Vec<(u32, u128)> {
	System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Rc2Migrator(Event::AccountsTeleported { count, amount }) =>
				Some((count, amount)),
			_ => None,
		})
		.collect()
}

fn total_issuance() -> u128 {
	pallet_balances::TotalIssuance::<Test>::get()
}

fn free(who: &AccountId) -> u128 {
	pallet_balances::Pallet::<Test>::free_balance(who)
}

/// The `i`th of many accounts.
fn account(i: u32) -> AccountId {
	let mut id = [0u8; 32];
	id[..4].copy_from_slice(&i.to_le_bytes());
	AccountId32::new(id)
}

/// One account per row of the table in `accounts.md`.
#[test]
fn migrate_one_account_per_case() {
	new_test_ext().execute_with(|| {
		// GIVEN one account per case.
		let alice = ALICE; // free balance
		fund(&alice, 100);
		let pot = pallet_account(); // module account
		fund(&pot, 100);
		let charlie = CHARLIE; // below the existential deposit
		force_below_ed(&charlie, ED - 1);
		let dave = DAVE; // the manager
		fund(&dave, 100);
		let para = child_sov(2000); // para sovereign
		fund(&para, 100);
		let sibl: AccountId = sibling_account(3000); // sibling-format account
		fund(&sibl, 100);
		let issuance = total_issuance();

		// WHEN one block withdraws them all.
		let out = Migrator::migrate_many(None, &[dave.clone()]);
		assert_eq!(out.last_key, None);
		let mut to_ah = out.to_ah;
		// sorting just to make it easier to assert
		to_ah.sort();

		// THEN each lands where its row says.
		let mut expected = vec![
			(alice.clone(), 100),         // free balance
			(sibling_account(2000), 100), // para sovereign, translated
			(sibl.clone(), 100),          // sibling-format, as is
		];
		expected.sort();
		assert_eq!(to_ah, expected);
		for who in [&alice, &para, &sibl] {
			assert!(!frame_system::Account::<Test>::contains_key(who), "{who:?} is reaped");
		}
		assert_eq!(free(&pot), 100, "module account");
		assert_eq!(free(&charlie), ED - 1, "below the existential deposit");
		assert_eq!(free(&dave), 100, "the manager");

		// AND what was burned is exactly what left.
		let left: u128 = to_ah.iter().map(|(_, amount)| amount).sum();
		assert_eq!(total_issuance(), issuance - left);
	});
}

#[test]
fn migrate_many_resumes_from_cursor() {
	new_test_ext().execute_with(|| {
		// GIVEN a limit of 3 accounts a block, and 4 accounts.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 3, xcm_per_block: 1 });
		for i in 0..4 {
			fund(&account(i), 100);
		}
		assert_eq!(total_issuance(), 400);

		// WHEN the first block runs. THEN it withdraws 3 and leaves a cursor.
		let first = Migrator::migrate_many(None, &[]);
		assert_eq!(first.to_ah.len(), 3);
		assert_eq!(total_issuance(), 100);
		let last_key = first.last_key.expect("one account is left");

		// WHEN the next block continues after the cursor. THEN it withdraws the rest.
		let second = Migrator::migrate_many(Some(last_key), &[]);
		assert_eq!(second.to_ah.len(), 1);
		assert_eq!(second.last_key, None);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn migrate_many_skips_unhandled_accounts() {
	new_test_ext().execute_with(|| {
		// GIVEN a plain account and one with a reserve.
		fund(&ALICE, 100);
		fund(&BOB, 100);
		reserve(&BOB, 40);

		// WHEN a block runs. THEN only the plain account is withdrawn; the other is untouched.
		let out = Migrator::migrate_many(None, &[]);
		assert_eq!(out.to_ah, vec![(ALICE, 100)]);
		assert_eq!(pallet_balances::Pallet::<Test>::free_balance(&BOB), 60);
		assert_eq!(pallet_balances::Pallet::<Test>::reserved_balance(&BOB), 40);
		assert_eq!(total_issuance(), 100);
	});
}

#[test]
fn accounts_stage_teleports_to_asset_hub() {
	new_test_ext().execute_with(|| {
		// GIVEN two plain accounts and the stage about to start.
		fund(&ALICE, 100);
		fund(&BOB, 250);
		RcMigrationStage::<Test>::put(Stage::AccountsInit);

		// WHEN the stage initialises. THEN the ledger holds the issuance and nothing is sent.
		run_blocks(1);
		assert_stage(Stage::AccountsOngoing { last_key: None });
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 350, ..Default::default() }
		);
		assert!(sent().is_empty());

		// WHEN the block of withdrawals runs. THEN one teleport carries both, everything is
		// booked as moved to Asset Hub, and the stage is done.
		run_blocks(1);
		assert_stage(Stage::AccountsDone);
		let sent = sent();
		assert_eq!(sent.len(), 1);
		let (dest, Xcm(message)) = &sent[0];
		assert_eq!(*dest, ah_dest());
		// the total is teleported in, then one deposit per account (in storage order)
		assert_eq!(
			message[..2],
			[
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				ReceiveTeleportedAsset((Location::parent(), 350u128).into()),
			]
		);
		assert_eq!(message.len(), 4);
		assert!(message.contains(&deposit(ALICE, 100)));
		assert!(message.contains(&deposit(BOB, 250)));
		assert_eq!(teleported_events(), vec![(2, 350)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 350, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn accounts_stage_sends_in_chunks_and_resumes() {
	new_test_ext().execute_with(|| {
		// GIVEN 7 accounts of 100, limits of 2 accounts a message and 2 messages a block, and the
		// stage initialised.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 2, xcm_per_block: 2 });
		for i in 0..7 {
			fund(&account(i), 100);
		}
		assert_eq!(total_issuance(), 700);
		RcMigrationStage::<Test>::put(Stage::AccountsInit);
		run_blocks(1);

		// WHEN the first block runs. THEN the 2 messages carry 4 accounts and the stage carries a
		// cursor.
		run_blocks(1);
		assert!(matches!(
			RcMigrationStage::<Test>::get(),
			Stage::AccountsOngoing { last_key: Some(_) }
		));
		assert_eq!(accounts_per_message(), vec![2, 2]);
		assert_eq!(total_issuance(), 300);

		// WHEN the next block runs. THEN the last 3 leave in a full and a half message, and
		// the stage is done.
		run_blocks(1);
		assert_stage(Stage::AccountsDone);
		assert_eq!(accounts_per_message(), vec![2, 2, 2, 1]);
		assert_eq!(total_issuance(), 0);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 700, ..Default::default() }
		);
	});
}

#[test]
fn accounts_stage_retries_a_block_whose_send_failed() {
	new_test_ext().execute_with(|| {
		// GIVEN the stage initialised with one plain account.
		fund(&ALICE, 100);
		RcMigrationStage::<Test>::put(Stage::AccountsInit);
		run_blocks(1);
		let migrated = RcMigratedBalance::<Test>::get();

		// WHEN a block runs and its send fails. THEN nothing moved.
		SendFails::set(true);
		run_blocks(1);
		assert_stage(Stage::AccountsOngoing { last_key: None });
		assert_eq!(pallet_balances::Pallet::<Test>::free_balance(&ALICE), 100);
		assert_eq!(total_issuance(), 100);
		assert_eq!(RcMigratedBalance::<Test>::get(), migrated);
		assert!(sent().is_empty());

		// WHEN the next block runs with a working router. THEN the same range leaves.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::AccountsDone);
		assert_eq!(sent().len(), 1);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn set_limits_checks_origin_and_zero() {
	new_test_ext().execute_with(|| {
		let limits = BlockLimits { items_per_xcm: 2, xcm_per_block: 2 };
		assert_noop!(Rc2Migrator::set_limits(RuntimeOrigin::signed(ALICE), limits), BadOrigin);
		assert_noop!(
			Rc2Migrator::set_limits(
				RuntimeOrigin::signed(ADMIN),
				BlockLimits { items_per_xcm: 0, ..limits }
			),
			Error::<Test>::ZeroLimit
		);
		assert_ok!(Rc2Migrator::set_limits(RuntimeOrigin::signed(ADMIN), limits));
		assert_eq!(Limits::<Test>::get(), limits);
	});
}
