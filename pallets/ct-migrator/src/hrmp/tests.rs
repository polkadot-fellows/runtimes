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
use frame_support::{assert_noop, assert_ok};
use migrator_types::{ChannelId, DepositSide, PortableHoldReason};
use sp_runtime::DispatchError::BadOrigin;

const PARA: u32 = 2001;
const OTHER: u32 = 2002;

fn record(
	sender: u32,
	recipient: u32,
	sender_deposit: u128,
	recipient_deposit: u128,
) -> PortableHrmpChannelRecord {
	PortableHrmpChannelRecord {
		channel: ChannelId { sender, recipient },
		sender_deposit,
		recipient_deposit,
	}
}

fn sov(para: u32) -> AccountId {
	sibling_account(para)
}

/// What the accounts stage leaves for a para: `free` and an `HrmpDeposit` hold of `held`.
fn migrate_account(para: u32, free: u128, held: u128) {
	CtMigrator::do_receive_accounts(vec![portable_account(
		&sov(para),
		free,
		vec![(PortableHoldReason::HrmpDeposit, held)],
	)]);
	migrator_events();
}

/// What `pallet-hrmp-para` holds for `para`.
fn channel_held(para: u32) -> u128 {
	<Balances as InspectHold<AccountId>>::balance_on_hold(
		&RuntimeHoldReason::HrmpPara(pallet_hrmp_para::HoldReason::ChannelDeposit),
		&sov(para),
	)
}

#[test]
fn receive_moves_the_migrated_hold_to_the_channel_sides() {
	new_test_ext().execute_with(|| {
		// GIVEN a para that pays as sender on one channel and as recipient on another, and
		// arrived with an `HrmpDeposit` hold of both.
		migrate_account(PARA, 10, 50 + 40);
		let issuance = total_issuance();

		// WHEN the records arrive.
		CtMigrator::do_receive_hrmp_deposits(vec![
			record(PARA, OTHER, 50, 0),
			record(OTHER, PARA, 0, 40),
		]);

		// THEN the HRMP pallet holds each side, out of the migrated hold, and nothing else
		// moved.
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 0);
		assert_eq!(channel_held(PARA), 90);
		assert_eq!(
			pallet_hrmp_para::Deposits::<Test>::get((
				ChannelId { sender: PARA, recipient: OTHER },
				DepositSide::Sender
			)),
			Some(50)
		);
		assert_eq!(
			pallet_hrmp_para::Deposits::<Test>::get((
				ChannelId { sender: OTHER, recipient: PARA },
				DepositSide::Recipient
			)),
			Some(40)
		);
		assert_eq!(free(&sov(PARA)), 10);
		assert_eq!(total_issuance(), issuance);
		assert_eq!(
			migrator_events(),
			vec![Event::HrmpDepositsReceived { count_good: 2, count_bad: 0 }]
		);
	});
}

#[test]
fn a_short_hold_is_made_up_from_free_balance() {
	new_test_ext().execute_with(|| {
		// GIVEN a para whose sovereign had reserved less than its records say, but has free
		// balance here.
		migrate_account(PARA, 100, 30);

		// WHEN its deposit arrives.
		CtMigrator::do_receive_hrmp_deposits(vec![record(PARA, OTHER, 50, 0)]);

		// THEN the whole deposit is held, the shortfall taken from free balance.
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 0);
		assert_eq!(channel_held(PARA), 50);
		assert_eq!(free(&sov(PARA)), 80);
	});
}

#[test]
fn a_hold_larger_than_the_deposits_keeps_the_rest() {
	new_test_ext().execute_with(|| {
		// GIVEN a para with a larger `HrmpDeposit` hold than the deposit that arrives.
		migrate_account(PARA, 10, 70);

		// WHEN its deposit arrives.
		CtMigrator::do_receive_hrmp_deposits(vec![record(PARA, OTHER, 50, 0)]);

		// THEN only the deposit leaves the migrated hold.
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 20);
		assert_eq!(channel_held(PARA), 50);
		assert_eq!(free(&sov(PARA)), 10);
	});
}

#[test]
fn a_refused_deposit_leaves_its_hold_in_place() {
	new_test_ext().execute_with(|| {
		// GIVEN two paras with their migrated holds.
		migrate_account(PARA, 10, 50);
		migrate_account(OTHER, 10, 40);

		// WHEN a record is refused, and the next one is not.
		HrmpDepositsFail::set(true);
		CtMigrator::do_receive_hrmp_deposits(vec![record(PARA, OTHER, 50, 0)]);
		HrmpDepositsFail::set(false);
		CtMigrator::do_receive_hrmp_deposits(vec![record(OTHER, PARA, 40, 0)]);

		// THEN the refused one's hold is untouched.
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 50);
		assert_eq!(channel_held(PARA), 0);

		// AND the other one went through.
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(OTHER)), 0);
		assert_eq!(channel_held(OTHER), 40);
		assert_eq!(
			migrator_events(),
			vec![
				Event::HrmpDepositsReceived { count_good: 0, count_bad: 1 },
				Event::HrmpDepositsReceived { count_good: 1, count_bad: 0 },
			]
		);
	});
}

/// A rewind of the relay stage delivers a deposit again.
#[test]
fn a_deposit_delivered_twice_is_held_once() {
	new_test_ext().execute_with(|| {
		// GIVEN a para whose deposit was held.
		migrate_account(PARA, 10, 50);
		let deposits = vec![record(PARA, OTHER, 50, 0)];
		CtMigrator::do_receive_hrmp_deposits(deposits.clone());
		assert_eq!(channel_held(PARA), 50);
		migrator_events();

		// WHEN the same deposit arrives again. THEN nothing moves, and it still counts as good.
		CtMigrator::do_receive_hrmp_deposits(deposits);
		assert_eq!(channel_held(PARA), 50);
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 0);
		assert_eq!(free(&sov(PARA)), 10);
		assert_eq!(
			migrator_events(),
			vec![Event::HrmpDepositsReceived { count_good: 1, count_bad: 0 }]
		);
	});
}

#[test]
fn a_deposit_delivered_twice_leaves_the_other_deposits_held() {
	new_test_ext().execute_with(|| {
		// GIVEN a para whose hold covers two deposits, one of which was delivered.
		migrate_account(PARA, 10, 90);
		let first = vec![record(PARA, OTHER, 50, 0)];
		CtMigrator::do_receive_hrmp_deposits(first.clone());
		assert_eq!(channel_held(PARA), 50);
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 40);

		// WHEN it arrives again. THEN the other deposit stays held, nothing is freed.
		CtMigrator::do_receive_hrmp_deposits(first);
		assert_eq!(channel_held(PARA), 50);
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 40);
		assert_eq!(free(&sov(PARA)), 10);

		// WHEN the other deposit arrives. THEN it is held out of what was left.
		CtMigrator::do_receive_hrmp_deposits(vec![record(OTHER, PARA, 0, 40)]);
		assert_eq!(channel_held(PARA), 90);
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 0);
		assert_eq!(free(&sov(PARA)), 10);
	});
}

#[test]
fn a_record_holds_both_sides_on_their_paras() {
	new_test_ext().execute_with(|| {
		// GIVEN the two ends of a channel, each with its migrated hold.
		migrate_account(PARA, 10, 50);
		migrate_account(OTHER, 10, 40);

		// WHEN the channel's record arrives. THEN each para holds its side.
		CtMigrator::do_receive_hrmp_deposits(vec![record(PARA, OTHER, 50, 40)]);
		assert_eq!(channel_held(PARA), 50);
		assert_eq!(channel_held(OTHER), 40);
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(PARA)), 0);
		assert_eq!(held(HoldReason::HrmpDeposit, &sov(OTHER)), 0);
		assert_eq!(
			migrator_events(),
			vec![Event::HrmpDepositsReceived { count_good: 1, count_bad: 0 }]
		);
	});
}

#[test]
fn only_root_receives_hrmp_deposits() {
	new_test_ext().execute_with(|| {
		let deposits = vec![record(PARA, OTHER, 50, 0)];
		assert_noop!(
			CtMigrator::receive_hrmp_deposits(RuntimeOrigin::signed(ADMIN), deposits.clone()),
			BadOrigin
		);
		assert_ok!(CtMigrator::receive_hrmp_deposits(RuntimeOrigin::root(), deposits));
	});
}
