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
	mock::*, BlockLimits, CtMigratorCall, CtRuntimeCall, Event, ExpectedReserves, RcMigrationStage,
};
use frame_support::traits::ReservableCurrency;

type Migrator = HrmpMigrator<Test>;

const SENDER: u32 = 2001; // the two ends of an open channel
const RECIPIENT: u32 = 2002;
const SENDER_DEPOSIT: u128 = 50;
const RECIPIENT_DEPOSIT: u128 = 40;
const REQUEST_DEPOSIT: u128 = 30;
const SYSTEM_PARA: u32 = 1001;

fn deposit(sender: u32, recipient: u32, side: DepositSide, amount: u128) -> PortableHrmpDeposit {
	PortableHrmpDeposit {
		key: DepositKey { channel: ChannelId { sender, recipient }, side },
		amount,
	}
}

/// The deposits of each `ReceiveHrmpDeposits` sent so far, one entry per message.
fn sent_deposits() -> Vec<Vec<PortableHrmpDeposit>> {
	(0..sent().len())
		.filter_map(|n| match sent_call(n) {
			CtRuntimeCall::CtMigrator(CtMigratorCall::ReceiveHrmpDeposits { deposits }) =>
				Some(deposits),
			_ => None,
		})
		.collect()
}

#[test]
fn of_channel_names_each_side_that_pays() {
	new_test_ext().execute_with(|| {
		// GIVEN a channel between two paras, and one from a system chain that pays nothing.
		open_channel(SENDER, RECIPIENT, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		open_channel(SYSTEM_PARA, RECIPIENT, 0, 0);

		// THEN the first pays on both sides and the system channel on neither.
		let of = |sender: u32, recipient: u32| {
			let id = HrmpChannelId { sender: sender.into(), recipient: recipient.into() };
			let channel = hrmp::HrmpChannels::<Test>::get(&id).unwrap();
			Migrator::of_channel(&id, &channel).collect::<Vec<_>>()
		};
		assert_eq!(
			of(SENDER, RECIPIENT),
			vec![
				deposit(SENDER, RECIPIENT, DepositSide::Sender, SENDER_DEPOSIT),
				deposit(SENDER, RECIPIENT, DepositSide::Recipient, RECIPIENT_DEPOSIT),
			]
		);
		assert_eq!(of(SYSTEM_PARA, RECIPIENT), vec![]);
	});
}

#[test]
fn of_request_adds_the_recipient_once_accepted_between_paras() {
	new_test_ext().execute_with(|| {
		// GIVEN a pending request, an accepted one, and one accepted by a system chain.
		open_request(SENDER, RECIPIENT, REQUEST_DEPOSIT);
		open_request(RECIPIENT, SENDER, REQUEST_DEPOSIT);
		accept_request(RECIPIENT, SENDER);
		open_request(SENDER, SYSTEM_PARA, REQUEST_DEPOSIT);
		hrmp::HrmpOpenChannelRequests::<Test>::mutate(
			HrmpChannelId { sender: SENDER.into(), recipient: SYSTEM_PARA.into() },
			|r| r.as_mut().unwrap().confirmed = true,
		);

		let of = |sender: u32, recipient: u32| {
			let id = HrmpChannelId { sender: sender.into(), recipient: recipient.into() };
			let request = hrmp::HrmpOpenChannelRequests::<Test>::get(&id).unwrap();
			Migrator::of_request(&id, &request, Migrator::recipient_deposit()).collect::<Vec<_>>()
		};

		// THEN the sender always pays, and the recipient once it accepted, unless it is a system
		// chain.
		assert_eq!(
			of(SENDER, RECIPIENT),
			vec![deposit(SENDER, RECIPIENT, DepositSide::Sender, REQUEST_DEPOSIT)]
		);
		assert_eq!(
			of(RECIPIENT, SENDER),
			vec![
				deposit(RECIPIENT, SENDER, DepositSide::Sender, REQUEST_DEPOSIT),
				deposit(RECIPIENT, SENDER, DepositSide::Recipient, HRMP_RECIPIENT_DEPOSIT),
			]
		);
		assert_eq!(
			of(SENDER, SYSTEM_PARA),
			vec![deposit(SENDER, SYSTEM_PARA, DepositSide::Sender, REQUEST_DEPOSIT)]
		);
	});
}

#[test]
fn migrate_many_skips_a_closing_channel_and_resumes_from_cursor() {
	new_test_ext().execute_with(|| {
		// GIVEN 4 channels, one of which is closing, and 2 deposits a block.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 2, xcm_per_block: 1 });
		for recipient in [2010, 2011, 2012, 2013] {
			open_channel(SENDER, recipient, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		}
		request_close(SENDER, 2011);

		// WHEN the blocks run until the channels are exhausted.
		let mut blocks = Vec::new();
		let mut last_key = None;
		loop {
			let out = Migrator::migrate_many(last_key);
			last_key = out.last_key.clone();
			blocks.push(out.deposits);
			if last_key.is_none() {
				break;
			}
		}

		// THEN each block carried one channel's two deposits, the closing channel counted for
		// nothing wherever it fell, and a last block found nothing left.
		assert_eq!(blocks.iter().map(Vec::len).collect::<Vec<_>>(), vec![2, 2, 2, 0]);
		let mut recipients: Vec<u32> =
			blocks.concat().iter().map(|d| d.key.channel.recipient).collect();
		recipients.sort();
		recipients.dedup();
		assert_eq!(recipients, vec![2010, 2012, 2013]);
	});
}

#[test]
fn hrmp_stage_sends_the_requests_then_the_channels_in_chunks() {
	new_test_ext().execute_with(|| {
		// GIVEN a pending request, 3 channels, and limits of 2 deposits a message and 1 message
		// a block.
		Limits::<Test>::put(BlockLimits { items_per_xcm: 2, xcm_per_block: 1 });
		open_request(SENDER, RECIPIENT, REQUEST_DEPOSIT);
		for recipient in [2010, 2011, 2012] {
			open_channel(SENDER, recipient, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		}

		// WHEN the stage initialises. THEN the request's deposit leaves in one message.
		RcMigrationStage::<Test>::put(Stage::HrmpInit);
		run_blocks(1);
		assert_stage(Stage::HrmpOngoing { last_key: None });
		assert_eq!(
			sent_deposits(),
			vec![vec![deposit(SENDER, RECIPIENT, DepositSide::Sender, REQUEST_DEPOSIT)]]
		);

		// WHEN the next block runs. THEN one channel leaves, both sides, and the stage carries
		// a cursor.
		run_blocks(1);
		assert!(matches!(
			RcMigrationStage::<Test>::get(),
			Stage::HrmpOngoing { last_key: Some(_) }
		));
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 2]);

		// WHEN two more blocks run. THEN the other channels leave, and one more block finds
		// nothing left and ends the stage.
		run_blocks(2);
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 2, 2, 2]);
		run_blocks(1);
		assert_stage(Stage::HrmpDone);
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 2, 2, 2]);

		// AND the messages carried every deposit once, and the events count them.
		let mut sent: Vec<_> = sent_deposits().concat();
		sent.sort_by_key(|d| d.key);
		let mut expected = vec![deposit(SENDER, RECIPIENT, DepositSide::Sender, REQUEST_DEPOSIT)];
		for recipient in [2010, 2011, 2012] {
			expected.push(deposit(SENDER, recipient, DepositSide::Sender, SENDER_DEPOSIT));
			expected.push(deposit(SENDER, recipient, DepositSide::Recipient, RECIPIENT_DEPOSIT));
		}
		expected.sort_by_key(|d| d.key);
		assert_eq!(sent, expected);
		let counts: Vec<u32> = System::events()
			.into_iter()
			.filter_map(|r| match r.event {
				RuntimeEvent::Rc2Migrator(Event::HrmpDepositsBatchSent { count }) => Some(count),
				_ => None,
			})
			.collect();
		assert_eq!(counts, vec![1, 2, 2, 2]);

		// AND the records are left as they were.
		assert_eq!(hrmp::HrmpChannels::<Test>::iter().count(), 3);
		assert_eq!(hrmp::HrmpOpenChannelRequests::<Test>::iter().count(), 1);
	});
}

#[test]
fn hrmp_stage_retries_a_block_whose_send_failed() {
	new_test_ext().execute_with(|| {
		// GIVEN a pending request and a channel.
		open_request(SENDER, RECIPIENT, REQUEST_DEPOSIT);
		open_channel(SENDER, 2010, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);

		// WHEN the stage initialises and its send fails. THEN it stays put.
		SendFails::set(true);
		RcMigrationStage::<Test>::put(Stage::HrmpInit);
		run_blocks(1);
		assert_stage(Stage::HrmpInit);

		// WHEN the router works again. THEN the request leaves.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::HrmpOngoing { last_key: None });
		assert_eq!(sent_deposits().len(), 1);

		// WHEN a channel block's send fails. THEN the cursor does not move.
		SendFails::set(true);
		run_blocks(1);
		assert_stage(Stage::HrmpOngoing { last_key: None });

		// WHEN the router works again. THEN the same channel leaves and the stage is done.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::HrmpDone);
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 2]);
	});
}

#[test]
fn the_deposits_are_on_the_coretime_chain_from_hrmp_done() {
	let moved = [
		Stage::HrmpDone,
		Stage::Sweep,
		Stage::SweepDust { last_key: None },
		Stage::TiCorrection,
		Stage::CoolOff { end_at: 10 },
		Stage::MigrationDone,
	];
	let not_yet = [
		Stage::Pending,
		Stage::Scheduled { start: 10 },
		Stage::WaitingForCt,
		Stage::WarmUp { end_at: 10 },
		Stage::AccountsInit,
		Stage::AccountsOngoing { last_key: None },
		Stage::AccountsDone,
		Stage::ProxyInit,
		Stage::ProxyOngoing { last_key: None },
		Stage::ProxyDone,
		Stage::RegistrarInit,
		Stage::RegistrarOngoing { last_key: None },
		Stage::RegistrarDone,
		Stage::HrmpInit,
		Stage::HrmpOngoing { last_key: None },
	];
	for stage in moved {
		assert!(stage.hrmp_deposits_moved(), "{stage:?}");
	}
	for stage in not_yet {
		assert!(!stage.hrmp_deposits_moved(), "{stage:?}");
	}
}

/// The accounts stage sends the reserve of each para under the amounts the HRMP stage names,
/// so the holds on the Coretime chain cover the deposits that arrive.
#[test]
fn the_accounts_stage_holds_what_the_hrmp_stage_sends() {
	new_test_ext().execute_with(|| {
		// GIVEN a channel and a pending request.
		open_channel(SENDER, RECIPIENT, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		open_request(RECIPIENT, SENDER, REQUEST_DEPOSIT);

		// WHEN the deposits are indexed. THEN each para is expected to hold the sum of what the
		// HRMP stage sends for it.
		crate::accounts::AccountsMigrator::<Test>::init();
		let requests = Migrator::request_deposits();
		let channels = Migrator::migrate_many(None).deposits;
		for para in [SENDER, RECIPIENT] {
			let sent: u128 = requests
				.iter()
				.chain(&channels)
				.filter(|d| d.key.para() == para)
				.map(|d| d.amount)
				.sum();
			assert_eq!(ExpectedReserves::<Test>::get(child_sov(para)).hrmp, sent);
			assert_eq!(
				<pallet_balances::Pallet<Test> as ReservableCurrency<_>>::reserved_balance(
					&child_sov(para)
				),
				sent
			);
		}
	});
}
