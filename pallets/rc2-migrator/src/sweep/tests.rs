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

use crate::{
	mock::*, BlockLimits, Event, Limits, Manager, MigratedBalances, RcMigratedBalance,
	RcMigrationStage,
};
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

/// The expected teleport message to Asset Hub, with one deposit per `(account, amount)`.
fn teleport(deposits: &[(AccountId, u128)]) -> (Location, Xcm<()>) {
	let total: u128 = deposits.iter().map(|(_, amount)| amount).sum();
	let mut message = vec![
		UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
		ReceiveTeleportedAsset((Location::parent(), total).into()),
	];
	message.extend(deposits.iter().map(|(who, amount)| DepositAsset {
		assets: AssetFilter::Definite((Location::parent(), *amount).into()),
		beneficiary: Location::new(
			0,
			[Junction::AccountId32 { network: None, id: who.clone().into() }],
		),
	}));
	(ah_dest(), Xcm(message))
}

/// The expected teleport message to the sweep beneficiary on Asset Hub.
fn teleport_to_beneficiary(amount: u128) -> (Location, Xcm<()>) {
	teleport(&[(SWEEP_BENEFICIARY, amount)])
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

/// Start Sweep Dust with the recorded balance set to the current Relay Chain issuance.
fn at_sweep_dust() {
	RcMigratedBalance::<Test>::put(MigratedBalances {
		kept: total_issuance(),
		..Default::default()
	});
	RcMigrationStage::<Test>::put(Stage::SweepDust { last_key: None });
}

/// `(account, amount, destination)` of every `DustSwept` event so far, sorted: accounts are visited
/// in storage order.
fn dust_events() -> Vec<(AccountId, u128, AccountId)> {
	let mut events: Vec<_> = System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Rc2Migrator(Event::DustSwept { who, amount, to }) =>
				Some((who, amount, to)),
			_ => None,
		})
		.collect();
	events.sort();
	events
}

fn exists(who: &AccountId) -> bool {
	frame_system::Account::<Test>::contains_key(who)
}

#[test]
fn dust_of_at_least_asset_hubs_existential_deposit_goes_to_its_owner() {
	new_test_ext().execute_with(|| {
		// GIVEN an account below the relay's existential deposit but not below Asset Hub's.
		force_below_ed(&ALICE, AH_ED);
		at_sweep_dust();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN the balance is teleported to the same account on Asset Hub, the account is
		// removed, and the stage moves on.
		assert_stage(Stage::TiCorrection);
		assert_eq!(sent(), vec![teleport(&[(ALICE, AH_ED)])]);
		assert_eq!(dust_events(), vec![(ALICE, AH_ED, ALICE)]);
		assert!(!exists(&ALICE));
		assert_eq!(total_issuance(), 0);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: AH_ED, ..Default::default() }
		);
	});
}

#[test]
fn dust_below_asset_hubs_existential_deposit_goes_to_the_beneficiary() {
	new_test_ext().execute_with(|| {
		// GIVEN two accounts below Asset Hub's existential deposit.
		force_below_ed(&ALICE, AH_ED - 1);
		force_below_ed(&BOB, AH_ED - 2);
		at_sweep_dust();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN both balances go to the sweep beneficiary in one deposit, and both accounts are
		// removed.
		assert_stage(Stage::TiCorrection);
		assert_eq!(sent(), vec![teleport_to_beneficiary(2 * AH_ED - 3)]);
		assert_eq!(
			dust_events(),
			vec![(ALICE, AH_ED - 1, SWEEP_BENEFICIARY), (BOB, AH_ED - 2, SWEEP_BENEFICIARY)]
		);
		assert!(!exists(&ALICE) && !exists(&BOB));
		assert_eq!(total_issuance(), 0);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 0, ah_free: 2 * AH_ED - 3, ..Default::default() }
		);
	});
}

#[test]
fn a_pure_proxys_and_a_pots_dust_goes_to_the_beneficiary() {
	new_test_ext().execute_with(|| {
		// GIVEN a pure proxy and a pot, each with dust Asset Hub would accept.
		fund(&IVAN, 1_000);
		let pure = create_pure(&IVAN);
		force_below_ed(&pure, AH_ED);
		let pot = pallet_account();
		force_below_ed(&pot, AH_ED);
		at_sweep_dust();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN both go to the sweep beneficiary.
		assert_stage(Stage::TiCorrection);
		assert_eq!(sent(), vec![teleport_to_beneficiary(2 * AH_ED)]);
		let mut expected =
			vec![(pure.clone(), AH_ED, SWEEP_BENEFICIARY), (pot.clone(), AH_ED, SWEEP_BENEFICIARY)];
		expected.sort();
		assert_eq!(dust_events(), expected);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 1_000, ah_free: 2 * AH_ED, ..Default::default() }
		);
		// AND the pure proxy is removed, but the pot stays, empty, for its pallet.
		assert!(!exists(&pure));
		assert_eq!(balance_and_refs(&pot), ((0, 0, 0), (1, 0)));
		// AND the pure's creator, above the existential deposit, is left alone.
		assert_eq!(frame_system::Account::<Test>::get(&IVAN).data.total(), 1_000);
	});
}

#[test]
fn a_referenced_account_is_emptied_and_kept() {
	new_test_ext().execute_with(|| {
		// GIVEN a validator below the existential deposit, whose session keys reference it.
		force_below_ed(&DAVE, AH_ED);
		add_consumer(&DAVE);
		at_sweep_dust();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN its balance goes to its owner, and the record stays, empty, for the session
		// pallet.
		assert_stage(Stage::TiCorrection);
		assert_eq!(sent(), vec![teleport(&[(DAVE, AH_ED)])]);
		assert_eq!(dust_events(), vec![(DAVE, AH_ED, DAVE)]);
		assert_eq!(balance_and_refs(&DAVE), ((0, 0, 0), (1, 1)));
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn an_empty_account_is_removed_if_one_provider_reference_keeps_it() {
	new_test_ext().execute_with(|| {
		// GIVEN an empty account kept alive by one old provider reference, and one kept alive by
		// two.
		force_below_ed(&EVE, 0);
		force_below_ed(&FRANK, 0);
		frame_system::Pallet::<Test>::inc_providers(&FRANK);
		at_sweep_dust();
		let migrated = RcMigratedBalance::<Test>::get();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN the first is removed and the second stays as it was. Nothing is sent, reported or
		// recorded.
		assert_stage(Stage::TiCorrection);
		assert!(!exists(&EVE));
		assert_eq!(balance_and_refs(&FRANK), ((0, 0, 0), (2, 0)));
		assert!(sent().is_empty());
		assert!(dust_events().is_empty());
		assert_eq!(RcMigratedBalance::<Test>::get(), migrated);
	});
}

#[test]
fn dust_is_swept_whole_whatever_form_it_is_in() {
	new_test_ext().execute_with(|| {
		// GIVEN an account below the existential deposit with free balance, a reserve and a named
		// hold, kept alive by another pallet's provider reference.
		frame_system::Pallet::<Test>::inc_providers(&GRACE);
		fund(&GRACE, 20);
		reserve(&GRACE, 4);
		stray_hold(&GRACE, 3);
		spend(&GRACE, 12);
		assert_eq!(balance_and_refs(&GRACE).0, (1, 7, 0));
		at_sweep_dust();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN all of it goes to its owner, and the account is removed with its balance records.
		assert_stage(Stage::TiCorrection);
		assert_eq!(sent(), vec![teleport(&[(GRACE, 8)])]);
		assert_eq!(dust_events(), vec![(GRACE, 8, GRACE)]);
		assert!(!exists(&GRACE));
		assert!(pallet_balances::Holds::<Test>::get(&GRACE).is_empty());
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn the_manager_and_accounts_at_the_existential_deposit_are_left_alone() {
	new_test_ext().execute_with(|| {
		// GIVEN an account at the existential deposit, and a manager below it.
		fund(&HEIDI, ED);
		force_below_ed(&IVAN, AH_ED);
		Manager::<Test>::put(IVAN);
		at_sweep_dust();

		// WHEN the stage runs. THEN neither is touched, and the stage moves on.
		run_blocks(1);
		assert_stage(Stage::TiCorrection);
		assert!(sent().is_empty());
		assert_eq!((free(&HEIDI), free(&IVAN)), (ED, AH_ED));
	});
}

#[test]
fn sweep_dust_continues_where_the_last_block_stopped() {
	new_test_ext().execute_with(|| {
		// GIVEN three dust accounts and room for one account per block.
		for who in [ALICE, BOB, CHARLIE] {
			force_below_ed(&who, AH_ED);
		}
		Limits::<Test>::put(BlockLimits { items_per_xcm: 1, xcm_per_block: 1 });
		at_sweep_dust();

		// WHEN three blocks run. THEN each sweeps one account, and the stage goes on.
		run_blocks(3);
		assert!(matches!(RcMigrationStage::<Test>::get(), Stage::SweepDust { last_key: Some(_) }));
		assert_eq!(sent().len(), 3);
		assert_eq!(
			dust_events(),
			vec![(ALICE, AH_ED, ALICE), (BOB, AH_ED, BOB), (CHARLIE, AH_ED, CHARLIE)]
		);

		// WHEN the next block runs. THEN it finds nothing left, and the stage moves on.
		run_blocks(1);
		assert_stage(Stage::TiCorrection);
		assert_eq!(sent().len(), 3);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn sweep_dust_retries_when_the_send_fails() {
	new_test_ext().execute_with(|| {
		// GIVEN a dust account and a router that refuses everything.
		force_below_ed(&ALICE, AH_ED);
		at_sweep_dust();
		let migrated = RcMigratedBalance::<Test>::get();
		SendFails::set(true);

		// WHEN the stage runs. THEN the burn is rolled back with the send, and the stage stays.
		run_blocks(1);
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(free(&ALICE), AH_ED);
		assert_eq!(total_issuance(), AH_ED);
		assert_eq!(RcMigratedBalance::<Test>::get(), migrated);
		assert!(dust_events().is_empty());

		// WHEN the router recovers. THEN the next block sweeps it.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::TiCorrection);
		assert_eq!(sent(), vec![teleport(&[(ALICE, AH_ED)])]);
		assert_eq!(total_issuance(), 0);
	});
}

#[test]
fn a_block_that_ends_on_the_manager_does_not_end_the_stage() {
	new_test_ext().execute_with(|| {
		// GIVEN two accounts below the existential deposit, the first in storage order being the
		// manager, and room for one account per block.
		force_below_ed(&ALICE, AH_ED);
		force_below_ed(&BOB, AH_ED);
		let manager = frame_system::Account::<Test>::iter_keys().next().expect("two accounts");
		let other = if manager == ALICE { BOB } else { ALICE };
		Manager::<Test>::put(manager.clone());
		Limits::<Test>::put(BlockLimits { items_per_xcm: 1, xcm_per_block: 1 });
		at_sweep_dust();

		// WHEN a block visits only the manager. THEN the stage continues after it.
		run_blocks(1);
		assert_stage(Stage::SweepDust { last_key: Some(manager.clone()) });

		// WHEN the next blocks run. THEN the other account is swept and the stage moves on.
		run_blocks(2);
		assert_stage(Stage::TiCorrection);
		assert_eq!(dust_events(), vec![(other.clone(), AH_ED, other)]);
		assert_eq!(free(&manager), AH_ED);
	});
}

#[test]
fn a_record_nothing_references_is_removed() {
	new_test_ext().execute_with(|| {
		// GIVEN an empty record with no references at all, as exist on the live chains.
		frame_system::Account::<Test>::insert(&EVE, frame_system::AccountInfo::default());
		assert_eq!(balance_and_refs(&EVE), ((0, 0, 0), (0, 0)));
		at_sweep_dust();
		let migrated = RcMigratedBalance::<Test>::get();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN the record is removed, and nothing is sent, reported or recorded.
		assert_stage(Stage::TiCorrection);
		assert!(!exists(&EVE));
		assert!(sent().is_empty());
		assert!(dust_events().is_empty());
		assert_eq!(RcMigratedBalance::<Test>::get(), migrated);
	});
}

/// Write `who` in the balances pallet's old account format, with no references, and add its
/// balance to the issuance.
fn old_format_account(who: &AccountId, free: u128, reserved: u128) {
	frame_system::Account::<Test>::insert(
		who,
		frame_system::AccountInfo {
			data: pallet_balances::AccountData {
				free,
				reserved,
				flags: pallet_balances::ExtraFlags::old_logic(),
				..Default::default()
			},
			..Default::default()
		},
	);
	pallet_balances::TotalIssuance::<Test>::mutate(|issuance| *issuance += free + reserved);
}

#[test]
fn an_old_format_pot_with_a_reserve_is_swept_for_what_it_holds() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot in the old format with only a reserve and no provider reference. Upgrading
		// it raises its free balance to the existential deposit, which the issuance does not
		// include.
		let pot = pallet_account();
		old_format_account(&pot, 0, 5);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN only what it held is burned and teleported, and the pot is reaped.
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(swept_events(), vec![(pot.clone(), 5)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(5)]);
		assert_eq!(total_issuance(), 0);
		assert!(!frame_system::Account::<Test>::contains_key(&pot), "the pot is reaped");
	});
}

#[test]
fn an_old_format_pot_nothing_references_is_swept_for_what_it_holds() {
	new_test_ext().execute_with(|| {
		// GIVEN a pot in the old format with free balance and no references. Upgrading it removes
		// the record, balance included.
		let pot = pallet_account();
		old_format_account(&pot, 7, 0);
		at_sweep();

		// WHEN the stage runs.
		run_blocks(1);

		// THEN its balance is still burned and teleported, and no record is left.
		assert_stage(Stage::SweepDust { last_key: None });
		assert_eq!(swept_events(), vec![(pot.clone(), 7)]);
		assert_eq!(sent(), vec![teleport_to_beneficiary(7)]);
		assert_eq!(total_issuance(), 0);
		assert!(!frame_system::Account::<Test>::contains_key(&pot), "no record is left");
	});
}
