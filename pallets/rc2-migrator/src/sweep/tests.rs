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

use crate::{mock::*, Event, MigratedBalances, RcMigratedBalance, RcMigrationStage};
use xcm::prelude::*;

/// Put the machine at the Sweep stage, with the ledger holding what is on chain, as the accounts
/// stage leaves it.
fn at_sweep() {
	RcMigratedBalance::<Test>::put(MigratedBalances {
		kept: total_issuance(),
		..Default::default()
	});
	RcMigrationStage::<Test>::put(Stage::Sweep);
}

/// `(pot, amount)` of every `PotSwept` event so far.
fn swept_events() -> Vec<(AccountId, u128)> {
	System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Rc2Migrator(Event::PotSwept { who, amount }) => Some((who, amount)),
			_ => None,
		})
		.collect()
}

/// The teleport that pays `amount` to the sweep beneficiary on Asset Hub.
fn teleport_to_beneficiary(amount: u128) -> (Location, Xcm<()>) {
	let message = Xcm(vec![
		UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		ReceiveTeleportedAsset((Location::parent(), amount).into()),
		DepositAsset {
			assets: AssetFilter::Definite((Location::parent(), amount).into()),
			beneficiary: Location::new(
				0,
				[Junction::AccountId32 { network: None, id: SWEEP_BENEFICIARY.into() }],
			),
		},
	]);
	(ah_dest(), message)
}

#[test]
fn sweep_burns_the_pots_and_teleports_the_total() {
	new_test_ext().execute_with(|| {
		// GIVEN two pots with balance.
		fund(&pallet_account(), 100);
		fund(&other_pot(), 250);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN both pots are burned and reaped, and the stage moves on.
		assert_stage(Stage::SweepDust { last_key: None });
		for pot in [pallet_account(), other_pot()] {
			assert!(!frame_system::Account::<Test>::contains_key(&pot), "{pot:?} is reaped");
		}
		assert_eq!(total_issuance(), 0);
		assert_eq!(swept_events(), vec![(pallet_account(), 100), (other_pot(), 250)]);
		// AND one teleport pays the total to the beneficiary, booked as moved to Asset Hub.
		assert_eq!(sent(), vec![teleport_to_beneficiary(350)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 350, ..Default::default() }
		);
	});
}

#[test]
fn sweep_with_nothing_in_the_pots_sends_nothing() {
	new_test_ext().execute_with(|| {
		// GIVEN one pot that does not exist and one that exists with nothing in it, as the
		// on-demand pot does once its revenue is claimed.
		force_below_ed(&other_pot(), 0);
		at_sweep();
		let migrated = RcMigratedBalance::<Test>::get();

		// WHEN the stage runs. THEN it moves on without sending or booking anything.
		run_blocks(1);
		assert_stage(Stage::SweepDust { last_key: None });
		assert!(sent().is_empty());
		assert!(swept_events().is_empty());
		assert_eq!(RcMigratedBalance::<Test>::get(), migrated);
	});
}

#[test]
fn sweep_retries_when_the_send_fails() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot with balance and a router that refuses everything.
		fund(&pallet_account(), 100);
		at_sweep();
		let migrated = RcMigratedBalance::<Test>::get();
		SendFails::set(true);

		// WHEN the stage runs. THEN the burn is rolled back with the send, and the stage stays.
		run_blocks(1);
		assert_stage(Stage::Sweep);
		assert_eq!(free(&pallet_account()), 100);
		assert_eq!(total_issuance(), 100);
		assert_eq!(RcMigratedBalance::<Test>::get(), migrated);
		assert!(swept_events().is_empty());

		// WHEN the router recovers. THEN the next block sweeps it.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(sent(), vec![teleport_to_beneficiary(100)]);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn a_pot_is_swept_whole_whatever_form_its_balance_is_in() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot with free balance, a reserve, a named hold and a lock.
		let pot = pallet_account();
		fund(&pot, 500);
		reserve(&pot, 40);
		stray_hold(&pot, 7);
		lock(&pot, 30);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN all of it is released and burned, the pot is reaped, and all of it is teleported.
		assert_stage(Stage::SweepDust { last_key: None });
		assert!(!frame_system::Account::<Test>::contains_key(&pot), "the pot is reaped");
		assert_eq!(swept_events(), vec![(pot, 500)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(500)]);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn a_referenced_pot_is_drained_to_an_empty_record() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot another pallet references, so it cannot be reaped, and a plain one.
		fund(&pallet_account(), 100);
		add_consumer(&pallet_account());
		fund(&other_pot(), 250);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN the referenced pot is emptied but its record stays, and both are teleported.
		assert_stage(Stage::SweepDust { last_key: None });
		let record = frame_system::Account::<Test>::get(pallet_account());
		assert_eq!((record.data.free, record.consumers), (0, 1), "the record stays, empty");
		assert_eq!(swept_events(), vec![(pallet_account(), 100), (other_pot(), 250)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(350)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 350, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
	});
}
