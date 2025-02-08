// Copyright (C) Parity Technologies (UK) Ltd.
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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Rust integration for the Asset Hub Migration.
//!
//! This test calls `on_initialize` on the RC and on AH alternately and forwards DMP messages.
//!
//! Create snapshots in the root dir:
//!
//! ```
//! try-runtime create-snapshot --uri wss://sys.ibp.network:443/statemint ah-polkadot.snap
//! try-runtime create-snapshot --uri wss://try-runtime.polkadot.io:443 polkadot.snap
//! ```
//!
//! Run with:
//!
//! ```
//! SNAP_RC="../../polkadot.snap" SNAP_AH="../../ah-polkadot.snap" RUST_LOG="info" ct polkadot-integration-tests-ahm -r on_initialize_works -- --nocapture
//! ```

use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::traits::*;
use pallet_rc_migrator::{types::PalletMigrationChecks, MigrationStage, RcMigrationStage};
use std::str::FromStr;
use std::io::Write;
use polkadot_runtime_common::crowdloan as pallet_crowdloan;
use polkadot_runtime_common::paras_registrar;

use polkadot_runtime_common::slots as pallet_slots;
use polkadot_runtime::Block as PolkadotBlock;
use asset_hub_polkadot_runtime::Runtime as AssetHub;
use polkadot_runtime::Runtime as Polkadot;

use super::mock::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_migration_works() {
	let Some((mut rc, mut ah)) = load_externalities().await else { return };
	let para_id = ParaId::from(1000);

	// Simulate relay blocks and grab the DMP messages
	let (dmp_messages, pre_check_payload) = rc.execute_with(|| {
		let mut dmps = Vec::new();

		if let Ok(stage) = std::env::var("START_STAGE") {
			let stage = MigrationStage::from_str(&stage).expect("Invalid start stage");
			RcMigrationStage::<Polkadot>::put(stage);
		}

		let pre_check_payload =
			pallet_rc_migrator::preimage::PreimageChunkMigrator::<Polkadot>::pre_check();

		// Loop until no more DMPs are added and we had at least 1
		loop {
			next_block_rc();

			let new_dmps =
				runtime_parachains::dmp::DownwardMessageQueues::<Polkadot>::take(para_id);
			dmps.extend(new_dmps);

			if RcMigrationStage::<Polkadot>::get() ==
				pallet_rc_migrator::MigrationStage::MigrationDone
			{
				log::info!("Migration done");
				break (dmps, pre_check_payload);
			}
		}
	});
	rc.commit_all().unwrap();
	// TODO: for some reason this prints some small value (2947), but logs on XCM send and receive
	// show more iteration.
	log::info!("Num of RC->AH DMP messages: {}", dmp_messages.len());

	// Inject the DMP messages into the Asset Hub
	ah.execute_with(|| {
		pallet_ah_migrator::preimage::PreimageMigrationCheck::<AssetHub>::pre_check();
		let mut fp =
			asset_hub_polkadot_runtime::MessageQueue::footprint(AggregateMessageOrigin::Parent);
		enqueue_dmp(dmp_messages);

		// Loop until no more DMPs are queued
		loop {
			let new_fp =
				asset_hub_polkadot_runtime::MessageQueue::footprint(AggregateMessageOrigin::Parent);
			if fp == new_fp {
				log::info!("AH DMP messages left: {}", fp.storage.count);
				break;
			}
			fp = new_fp;

			log::debug!("AH DMP messages left: {}", fp.storage.count);
			next_block_ah();

			if RcMigrationStage::<Polkadot>::get() ==
				pallet_rc_migrator::MigrationStage::PreimageMigrationDone
			{
				pallet_rc_migrator::preimage::PreimageChunkMigrator::<Polkadot>::post_check(
					pre_check_payload.clone(),
				);
			}
		}

		pallet_ah_migrator::preimage::PreimageMigrationCheck::<AssetHub>::post_check(());
		// NOTE that the DMP queue is probably not empty because the snapshot that we use contains
		// some overweight ones.
	});
}

use sp_runtime::AccountId32;
use sp_runtime::traits::Dispatchable;

/// Check that our function to calculate the unlock time of a crowdloan contribution is correct.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crowdloan_unlock_times_are_correct_works() {
	let mut rc = remote_ext_test_setup::<PolkadotBlock>("SNAP_RC").await.unwrap();

	rc.execute_with(|| {
		let now = frame_system::Pallet::<Polkadot>::block_number();
		let mut para_ids = pallet_crowdloan::Funds::<Polkadot>::iter_keys().collect::<Vec<_>>();
		para_ids.sort();

		for para_id in para_ids.clone() {
			let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
			// they all ended
			assert!(fund.end < now);

			let id: u32 = para_id.into();
			let fund_id = pallet_crowdloan::Pallet::<Polkadot>::fund_account_id(fund.fund_index);

			let acc = frame_system::Account::<Polkadot>::get(&fund_id);
			let total = acc.data.free + acc.data.reserved;

			if acc.data.reserved == 0 {
				println!("$ Para: {}, Fund index: {}, id: {} Can be dissolved", para_id, fund.fund_index, fund_id);

				assert!(fund.raised == 0);
				assert!(fund.deposit != 0, "we must have a deposit"); // TODO refund
				ensure_can_dissolve(para_id);
				
				continue;
			}
			
			println!("  Para: {}, Fund index: {}, id: {}, with {} total", para_id, fund.fund_index, fund_id, total / 10_000_000_000);
		}

		println!("#### Done ####");
		let mut refunds: String = "para_id,fund_id,account,amount,refund_date\n".into();

		for para_id in para_ids {
			let Some(fund) = pallet_crowdloan::Funds::<Polkadot>::get(para_id) else {
				continue;
			};

			let id: u32 = para_id.into();
			let fund_id = pallet_crowdloan::Pallet::<Polkadot>::fund_account_id(fund.fund_index);

			let acc = frame_system::Account::<Polkadot>::get(&fund_id);
			let total = acc.data.free + acc.data.reserved;
			let refund_time = block_number_to_date(calculate_refund_time(para_id));

			let mut contrib_iter = pallet_crowdloan::Pallet::<Polkadot>::contribution_iterator(fund.fund_index);
			let mut total_contrib = 0;
			let mut num_contribs = 0;

			while let Some((contributor, (contrib, _memo))) = contrib_iter.next() {
				total_contrib += contrib;
				num_contribs += 1;
				let amount = format!("{}", contrib / 10_000_000_000).replace(",", "_");
				refunds.push_str(&format!("{},{},{},{},{}\n", para_id, fund.fund_index, contributor, amount, refund_time));
			}
			assert_eq!(total_contrib, fund.raised);

			println!("  Para: {}, Fund index: {}, id: {}, with {} total, {} contribs", para_id, fund.fund_index, fund_id, total / 10_000_000_000, num_contribs);
			if acc.data.free + acc.data.reserved > fund.raised {
				println!("! Over funded by {}", (acc.data.free + acc.data.reserved - fund.raised) / 10_000_000_000);
			}
		}

		// write to file
		let mut file = std::fs::File::create("/Users/vados/Documents/work/runtimes/refunds.csv").unwrap();
		file.write_all(refunds.as_bytes()).unwrap();
	});
}

/// Calculate when a crowdloan will be able to dissolve.
fn calculate_refund_time(para_id: ParaId) -> u32 {
	let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
	let dissolve_time = fund.last_period * <Polkadot as pallet_slots::Config>::LeasePeriod::get() + <Polkadot as pallet_slots::Config>::LeaseOffset::get();
	//ensure_can_refund(para_id, dissolve_time);
	dissolve_time
}

fn block_number_to_date(block_number: u32) -> String {
    let block_now = frame_system::Pallet::<Polkadot>::block_number();
    let unix_now = pallet_timestamp::Now::<Polkadot>::get();
    let date = unix_now as i128 + (block_number as i128 - block_now as i128) as i128 * 6_000;
	chrono::NaiveDateTime::from_timestamp_millis(date as i64).unwrap().to_string()
}

fn ensure_can_refund(para_id: ParaId, at: u32) {
	frame_support::hypothetically!({
		let alice = AccountId32::new([0u8; 32]);
		pallet_balances::Pallet::<Polkadot>::make_free_balance_be(&alice, 100_000_000_000_000_000);
		run_to_block(at + 10);

		let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
		let origin = polkadot_runtime::RuntimeOrigin::signed(alice);
		let call: polkadot_runtime::RuntimeCall = pallet_crowdloan::Call::<Polkadot>::refund { index: para_id }.into();
		call.dispatch_bypass_filter(origin).unwrap(); // Why bypass?
	});
}

fn run_to_block(at: u32) {
	let mut bn = frame_system::Pallet::<Polkadot>::block_number();

	frame_system::Pallet::<Polkadot>::set_block_number(bn);
	polkadot_runtime::AllPalletsWithSystem::on_initialize(bn);
}

fn ensure_can_dissolve(para_id: ParaId) {
	frame_support::hypothetically!({
		let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
		let sender = fund.depositor;
		println!("fund index: {}, sender: {}, deposit: {}", fund.fund_index, sender, fund.deposit);
		let data_before = frame_system::Account::<Polkadot>::get(&sender).data;
		let origin = polkadot_runtime::RuntimeOrigin::signed(sender.clone());

		let call: polkadot_runtime::RuntimeCall = pallet_crowdloan::Call::<Polkadot>::dissolve { index: para_id }.into();
		call.dispatch(origin).unwrap();
		let data_after = frame_system::Account::<Polkadot>::get(&sender).data;

		if data_after.reserved >= data_before.reserved || data_after.free <= data_before.free {
			println!("! Could not unreserve");
		}
	});
}

// The block after which a crowdloan contribution will be able to redeem their contribution.
/*fn crowdloan_unlock_block<T: Config>(para_id: ParaId) -> u64 {
	let lease_period = T::LeasePeriod::get();
	let fund_index = T::FundIndex::get();
	let fund_period = fund_index / lease_period;
	lease_period * fund_period + fund_index
}
*/
