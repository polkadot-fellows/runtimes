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
use frame_support::traits::{
	fungible::{Mutate, MutateHold},
	tokens::{Fortitude, Precision, Preservation},
	ReservableCurrency,
};
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

#[test]
fn a_pot_is_swept_whole_whatever_form_its_balance_is_in() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot with free balance, a reserve, a named hold, a lock and a freeze. The freeze
		// is the larger of the two, so it alone keeps balance frozen if it is not released.
		let pot = pallet_account();
		fund(&pot, 500);
		reserve(&pot, 40);
		stray_hold(&pot, 7);
		lock(&pot, 30);
		freeze(&pot, 60);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN the entire balance is burned and teleported, and the pot is reaped with all its
		// balance records.
		assert_stage(Stage::SweepDust { last_key: None });
		assert!(!frame_system::Account::<Test>::contains_key(&pot), "the pot is reaped");
		assert!(pallet_balances::Holds::<Test>::get(&pot).is_empty());
		assert!(pallet_balances::Locks::<Test>::get(&pot).is_empty());
		assert!(pallet_balances::Freezes::<Test>::get(&pot).is_empty());
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

		// THEN the referenced pot is emptied but its record stays, and both balances are
		// teleported.
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

#[test]
fn a_pot_whose_hold_record_disagrees_with_its_balance_is_still_emptied() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot whose hold record says more than it has reserved, and a plain pot.
		let pot = pallet_account();
		fund(&pot, 100);
		stray_hold(&pot, 7);
		pallet_balances::Holds::<Test>::mutate(&pot, |holds| holds[0].amount = 50);
		fund(&other_pot(), 250);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN both are emptied. What the pot's balance says it holds is burned, and the wrong
		// hold record is removed with it.
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(swept_events(), vec![(pot.clone(), 100), (other_pot(), 250)]);
		assert!(!frame_system::Account::<Test>::contains_key(&pot), "the pot is reaped");
		assert!(pallet_balances::Holds::<Test>::get(&pot).is_empty());
		assert_eq!(sent(), vec![teleport_to_beneficiary(350)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 350, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
	});
}

/// Give `who` two provider references from other pallets, as the on-demand pallet gives its pot
/// one. With them, the balances pallet lets the free balance fall below the existential deposit
/// while something is held or reserved.
fn kept_alive_by_other_pallets(who: &AccountId) {
	frame_system::Pallet::<Test>::inc_providers(who);
	frame_system::Pallet::<Test>::inc_providers(who);
}

/// Burn `amount` of `who`'s free balance through the balances pallet.
fn spend(who: &AccountId, amount: u128) {
	<Balances as Mutate<AccountId>>::burn_from(
		who,
		amount,
		Preservation::Expendable,
		Precision::Exact,
		Fortitude::Polite,
	)
	.expect("can spend");
}

/// `(free, reserved, frozen)` of `who`, and its `(providers, consumers)`.
fn balance_and_refs(who: &AccountId) -> ((u128, u128, u128), (u32, u32)) {
	let account = frame_system::Account::<Test>::get(who);
	(
		(account.data.free, account.data.reserved, account.data.frozen),
		(account.providers, account.consumers),
	)
}

#[test]
fn a_pot_whose_hold_the_balances_pallet_will_not_release_is_still_emptied() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot with nothing free and less than the existential deposit on hold.
		let pot = pallet_account();
		kept_alive_by_other_pallets(&pot);
		fund(&pot, 18);
		stray_hold(&pot, 8);
		spend(&pot, 10);
		assert_eq!((free(&pot), held(&pot)), (0, 8));
		// AND the balances pallet refuses to release that hold, because the free balance would
		// stay below the existential deposit.
		assert!(frame_support::hypothetically!(<Balances as MutateHold<AccountId>>::release(
			&PreimageHoldReason::get(),
			&pot,
			8,
			Precision::Exact,
		))
		.is_err());
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN the pot is emptied, and all of it is recorded and teleported.
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(swept_events(), vec![(pot.clone(), 8)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(8)]);
		assert!(pallet_balances::Holds::<Test>::get(&pot).is_empty());
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 8, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
		// AND the record stays for the other pallets, without the balances pallet's reference.
		assert_eq!(balance_and_refs(&pot), ((0, 0, 0), (2, 0)));
	});
}

#[test]
fn a_pot_below_the_existential_deposit_is_swept_not_lost_as_dust() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot whose free balance and reserve add up to less than the existential deposit.
		let pot = pallet_account();
		kept_alive_by_other_pallets(&pot);
		fund(&pot, 15);
		<Balances as ReservableCurrency<AccountId>>::reserve(&pot, 8).unwrap();
		spend(&pot, 6);
		assert_eq!(balance_and_refs(&pot).0, (1, 8, 0));
		// AND unreserving through the balances pallet would destroy all of it as dust, without
		// this pallet recording it.
		assert_eq!(
			frame_support::hypothetically!({
				<Balances as ReservableCurrency<AccountId>>::unreserve(&pot, 8);
				total_issuance()
			}),
			0
		);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN all of it is recorded and teleported.
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(swept_events(), vec![(pot.clone(), 9)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(9)]);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 9, ..Default::default() }
		);
		assert_eq!(total_issuance(), 0);
		assert_eq!(balance_and_refs(&pot), ((0, 0, 0), (2, 0)));
	});
}
