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
	mock::*, BlockLimits, CtMigratorCall, CtRuntimeCall, Event, ExpectedReserves, Limits,
	RcMigrationStage,
};
use frame_support::traits::ReservableCurrency;

type Migrator = HrmpMigrator<Test>;

const SENDER: u32 = 2001; // the two ends of an open channel
const RECIPIENT: u32 = 2002;
const SENDER_DEPOSIT: u128 = 50;
const RECIPIENT_DEPOSIT: u128 = 40;
const REQUEST_DEPOSIT: u128 = 30;
const SYSTEM_PARA: u32 = 1001;

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

/// The deposits of each `ReceiveHrmpDeposits` sent so far, one entry per message.
fn sent_deposits() -> Vec<Vec<PortableHrmpChannelRecord>> {
	(0..sent().len())
		.filter_map(|n| match sent_call(n) {
			CtRuntimeCall::CtMigrator(CtMigratorCall::ReceiveHrmpDeposits { deposits }) =>
				Some(deposits),
			_ => None,
		})
		.collect()
}

/// The records `migrate_many` finds in one pass, sorted by channel.
fn all_deposits() -> Vec<PortableHrmpChannelRecord> {
	let mut deposits = Migrator::migrate_many(None).channels;
	deposits.sort_by_key(|d| d.channel);
	deposits
}

/// `deposits` sorted by key, for comparing with [`all_deposits`].
fn sorted(mut deposits: Vec<PortableHrmpChannelRecord>) -> Vec<PortableHrmpChannelRecord> {
	deposits.sort_by_key(|d| d.channel);
	deposits
}

#[test]
fn a_channel_pays_on_each_side_that_has_a_deposit() {
	new_test_ext().execute_with(|| {
		// GIVEN a channel between two paras, and one from a system chain that pays nothing.
		open_channel(SENDER, RECIPIENT, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		open_channel(SYSTEM_PARA, RECIPIENT, 0, 0);

		// THEN the first pays on both sides and the system channel on neither.
		assert_eq!(
			all_deposits(),
			vec![record(SENDER, RECIPIENT, SENDER_DEPOSIT, RECIPIENT_DEPOSIT)]
		);
	});
}

#[test]
fn a_request_adds_the_recipient_once_confirmed_between_paras() {
	new_test_ext().execute_with(|| {
		// GIVEN a pending request, a confirmed one, and one confirmed by a system chain.
		open_request(SENDER, RECIPIENT, REQUEST_DEPOSIT);
		open_request(RECIPIENT, SENDER, REQUEST_DEPOSIT);
		accept_request(RECIPIENT, SENDER);
		open_request(SENDER, SYSTEM_PARA, REQUEST_DEPOSIT);
		hrmp::HrmpOpenChannelRequests::<Test>::mutate(
			HrmpChannelId { sender: SENDER.into(), recipient: SYSTEM_PARA.into() },
			|r| r.as_mut().unwrap().confirmed = true,
		);

		// THEN the sender always pays, and the recipient once it confirmed, unless it is a system
		// chain.
		assert_eq!(
			all_deposits(),
			sorted(vec![
				record(SENDER, RECIPIENT, REQUEST_DEPOSIT, 0),
				record(RECIPIENT, SENDER, REQUEST_DEPOSIT, HRMP_RECIPIENT_DEPOSIT),
				record(SENDER, SYSTEM_PARA, REQUEST_DEPOSIT, 0),
			])
		);
	});
}

#[test]
fn migrate_many_skips_a_closing_channel_and_resumes_from_cursor() {
	new_test_ext().execute_with(|| {
		// GIVEN 4 channels, one of which is closing, and 1 record a block.
		HrmpRecordsPerBlock::set(1);
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
			blocks.push(out.channels);
			if last_key.is_none() {
				break;
			}
		}

		// THEN each block read one channel, the closing one counting for nothing wherever it
		// fell, and a last block found nothing left.
		assert_eq!(blocks.len(), 5);
		assert_eq!(blocks.iter().filter(|b| b.len() == 1).count(), 3);
		assert_eq!(blocks.iter().filter(|b| b.is_empty()).count(), 2);
		let mut recipients: Vec<u32> =
			blocks.concat().iter().map(|d| d.channel.recipient).collect();
		recipients.sort();
		recipients.dedup();
		assert_eq!(recipients, vec![2010, 2012, 2013]);
	});
}

#[test]
fn migrate_many_bounds_the_channels_read_when_none_pays() {
	new_test_ext().execute_with(|| {
		// GIVEN 3 system channels, which pay nothing, and 2 records a block.
		HrmpRecordsPerBlock::set(2);
		for recipient in [2010, 2011, 2012] {
			open_channel(SYSTEM_PARA, recipient, 0, 0);
		}

		// WHEN a block runs. THEN it reads 2 channels, finds no deposit, and leaves a cursor.
		let out = Migrator::migrate_many(None);
		assert_eq!(out.records, 2);
		assert_eq!(out.channels, vec![]);
		assert!(out.last_key.is_some());

		// WHEN the next block runs. THEN it reads the last channel and ends the stage.
		let out = Migrator::migrate_many(out.last_key);
		assert_eq!(out.records, 1);
		assert_eq!(out.channels, vec![]);
		assert_eq!(out.last_key, None);
	});
}

#[test]
fn migrate_many_walks_the_requests_then_the_channels() {
	new_test_ext().execute_with(|| {
		// GIVEN 2 requests, a channel, and 1 record a block.
		HrmpRecordsPerBlock::set(1);
		open_request(SENDER, 2010, REQUEST_DEPOSIT);
		open_request(SENDER, 2011, REQUEST_DEPOSIT);
		open_channel(SENDER, 2012, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);

		// WHEN the blocks run until the records are exhausted.
		let mut lens = Vec::new();
		let mut keys = Vec::new();
		let mut last_key = None;
		loop {
			let out = Migrator::migrate_many(last_key);
			lens.push(out.channels.len());
			last_key = out.last_key;
			match &last_key {
				Some(key) => keys.push(key.clone()),
				None => break,
			}
		}

		// THEN the requests went first, one a block, then the channel, and a last block found
		// nothing left.
		assert_eq!(lens, vec![1, 1, 1, 0]);
		assert!(matches!(
			keys[..],
			[HrmpKey::Request(_), HrmpKey::Request(_), HrmpKey::Channel(_)]
		));
	});
}

#[test]
fn hrmp_stage_sends_the_requests_then_the_channels_in_chunks() {
	new_test_ext().execute_with(|| {
		// GIVEN a pending request, 3 channels, 2 records a block, and 1 item a message and a
		// block.
		HrmpRecordsPerBlock::set(2);
		Limits::<Test>::put(BlockLimits { items_per_xcm: 1, xcm_per_block: 1 });
		open_request(SENDER, RECIPIENT, REQUEST_DEPOSIT);
		for recipient in [2010, 2011, 2012] {
			open_channel(SENDER, recipient, SENDER_DEPOSIT, RECIPIENT_DEPOSIT);
		}

		// WHEN the stage initialises. THEN nothing leaves yet.
		RcMigrationStage::<Test>::put(Stage::HrmpInit);
		run_blocks(1);
		assert_stage(Stage::HrmpOngoing { last_key: None });
		assert_eq!(sent_deposits(), Vec::<Vec<_>>::new());

		// WHEN the next block runs. THEN the request leaves alone, a channel would not have fit
		// the block's items, and the stage carries a cursor at the request.
		run_blocks(1);
		assert!(matches!(
			RcMigrationStage::<Test>::get(),
			Stage::HrmpOngoing { last_key: Some(HrmpKey::Request(_)) }
		));
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![1]);

		// WHEN three more blocks run. THEN the channels leave one a block, and one more block
		// finds nothing left and ends the stage.
		run_blocks(3);
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1, 1, 1]);
		run_blocks(1);
		assert_stage(Stage::HrmpDone);
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![1, 1, 1, 1]);

		// AND the messages carried every record once, and the events count them.
		let mut sent: Vec<_> = sent_deposits().concat();
		sent.sort_by_key(|d| d.channel);
		let mut expected = vec![record(SENDER, RECIPIENT, REQUEST_DEPOSIT, 0)];
		for recipient in [2010, 2011, 2012] {
			expected.push(record(SENDER, recipient, SENDER_DEPOSIT, RECIPIENT_DEPOSIT));
		}
		expected.sort_by_key(|d| d.channel);
		assert_eq!(sent, expected);
		let counts: Vec<u32> = System::events()
			.into_iter()
			.filter_map(|r| match r.event {
				RuntimeEvent::Rc2Migrator(Event::HrmpDepositsBatchSent { count }) => Some(count),
				_ => None,
			})
			.collect();
		assert_eq!(counts, vec![1, 1, 1, 1]);

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

		// WHEN the stage initialises. THEN it moves on, sending nothing.
		RcMigrationStage::<Test>::put(Stage::HrmpInit);
		run_blocks(1);
		assert_stage(Stage::HrmpOngoing { last_key: None });

		// WHEN a block's send fails. THEN the cursor does not move.
		SendFails::set(true);
		run_blocks(1);
		assert_stage(Stage::HrmpOngoing { last_key: None });
		assert_eq!(sent_deposits().len(), 0);

		// WHEN the router works again. THEN the request and the channel leave, and the stage is
		// done.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::HrmpDone);
		assert_eq!(sent_deposits().iter().map(Vec::len).collect::<Vec<_>>(), vec![2]);
	});
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
		let deposits = Migrator::migrate_many(None).channels;
		for para in [SENDER, RECIPIENT] {
			let mut sent = 0u128;
			for d in &deposits {
				if d.channel.sender == para {
					sent += d.sender_deposit;
				}
				if d.channel.recipient == para {
					sent += d.recipient_deposit;
				}
			}
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
