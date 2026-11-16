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

/// Start Sweep with the recorded balance set to the current Relay Chain issuance.
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

/// The expected teleport message to the sweep beneficiary on Asset Hub.
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

		// THEN both balances are burned, the accounts are reaped, and the stage moves on.
		assert_stage(Stage::SweepDust { last_key: None });
		for pot in [pallet_account(), other_pot()] {
			assert!(!frame_system::Account::<Test>::contains_key(&pot), "{pot:?} is reaped");
		}
		assert_eq!(total_issuance(), 0);
		assert_eq!(swept_events(), vec![(pallet_account(), 100), (other_pot(), 250)]);
		// AND one teleport pays the total to the beneficiary, recorded as moved to Asset Hub.
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

		// WHEN the stage runs. THEN it moves on without sending or recording anything. Only the
		// pot that exists is reported, with nothing swept.
		run_blocks(1);
		assert_stage(Stage::SweepDust { last_key: None });
		assert!(sent().is_empty());
		assert_eq!(swept_events(), vec![(other_pot(), 0)]);
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

/// `free` of `who`, and its `(providers, consumers)`.
fn free_and_refs(who: &AccountId) -> (u128, (u32, u32)) {
	let account = frame_system::Account::<Test>::get(who);
	(account.data.free, (account.providers, account.consumers))
}

#[test]
fn a_pot_another_pallet_provides_for_is_left_as_an_empty_record() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot another pallet keeps alive, as the on-demand pallet does its pot, and a
		// plain one.
		frame_system::Pallet::<Test>::inc_providers(&pallet_account());
		fund(&pallet_account(), 100);
		fund(&other_pot(), 250);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN both balances are burned and teleported. The plain pot is reaped, and the other
		// stays as an empty record with only the other pallet's reference.
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(free_and_refs(&pallet_account()), (0, (1, 0)));
		assert!(
			!frame_system::Account::<Test>::contains_key(other_pot()),
			"the plain pot is reaped"
		);
		assert_eq!(swept_events(), vec![(pallet_account(), 100), (other_pot(), 250)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(350)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 350, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn a_pot_below_the_existential_deposit_is_swept_whole() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot below the existential deposit, kept alive by another pallet.
		let pot = pallet_account();
		force_below_ed(&pot, ED - 1);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN all of it is burned, recorded and teleported, with nothing lost as dust.
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(swept_events(), vec![(pot.clone(), ED - 1)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(ED - 1)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: ED - 1, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
		assert_eq!(free_and_refs(&pot), (0, (1, 0)));
	});
}
