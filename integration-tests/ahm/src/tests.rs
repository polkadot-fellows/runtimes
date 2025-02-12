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

use asset_hub_polkadot_runtime::Runtime as AssetHub;
use codec::Encode;
use cumulus_primitives_core::{AggregateMessageOrigin, ParaId};
use frame_support::traits::*;
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_rc_migrator::{types::PalletMigrationChecks, MigrationStage, RcMigrationStage};
use parachains_common::impls::BalanceOf;
use polkadot_runtime::{Block as PolkadotBlock, Runtime as Polkadot};
use polkadot_runtime_common::{
	crowdloan as pallet_crowdloan, paras_registrar, paras_registrar as pallet_registrar,
	slots as pallet_slots,
};
use std::{collections::BTreeMap, io::Write, str::FromStr};

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

use sp_runtime::{traits::Dispatchable, AccountId32};

/// Check that our function to calculate the unlock time of a crowdloan contribution is correct.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crowdloan_unlock_times_are_correct_works() {
	std::env::set_var("SNAP_RC", "/Users/vados/Documents/work/runtimes/polkadot.snap");
	std::env::set_var("START_STAGE", "preimage");

	let mut rc = remote_ext_test_setup::<PolkadotBlock>("SNAP_RC").await.unwrap();

	rc.execute_with(|| {
		let now = frame_system::Pallet::<Polkadot>::block_number();
		let mut para_ids = pallet_crowdloan::Funds::<Polkadot>::iter_keys().collect::<Vec<_>>();
		para_ids.sort();

		for para_id in para_ids.clone() {
			let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
			assert!(fund.end < now);

			let id: u32 = para_id.into();
			let fund_id = pallet_crowdloan::Pallet::<Polkadot>::fund_account_id(fund.fund_index);

			let acc = frame_system::Account::<Polkadot>::get(&fund_id);
			let total = acc.data.free + acc.data.reserved;

			// All crowdloans without reserved amount can be dissolved. We will do this as part of
			// the migration
			if acc.data.reserved == 0 {
				println!(
					"$ Para: {}, Fund index: {}, id: {} Can be dissolved",
					para_id, fund.fund_index, fund_id
				);

				assert!(fund.raised == 0);
				assert!(fund.deposit != 0, "we must have a deposit"); // TODO refund
				ensure_can_dissolve(para_id);

				continue;
			}

			println!(
				"  Para: {}, Fund index: {}, id: {}, with {} total",
				para_id,
				fund.fund_index,
				fund_id,
				total / 10_000_000_000
			);
		}

		println!("#### Done ####");
		let mut refunds: String = "para_id,fund_id,account,amount,refund_date\n".into();

		/*for para_id in para_ids {
		let Some(fund) = pallet_crowdloan::Funds::<Polkadot>::get(para_id) else {
			continue;
		};

		let id: u32 = para_id.into();
		let fund_id = pallet_crowdloan::Pallet::<Polkadot>::fund_account_id(fund.fund_index);

		let acc = frame_system::Account::<Polkadot>::get(&fund_id);
		let total = acc.data.free + acc.data.reserved;*/
		let refund_time = calculate_refund_time(para_ids);

		/*let mut contrib_iter = pallet_crowdloan::Pallet::<Polkadot>::contribution_iterator(fund.fund_index);
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
		}*/

		// write to file
		//let mut file =
		// std::fs::File::create("/Users/vados/Documents/work/runtimes/refunds.csv").unwrap();
		// file.write_all(refunds.as_bytes()).unwrap();
	});
}

/// Calculate when a crowdloan will be able to dissolve.
fn calculate_refund_time(mut para_ids: Vec<ParaId>) -> BTreeMap<ParaId, BlockNumberFor<Polkadot>> {
	let mut cutoff = 10_000_000; // some high number to make the test timeout if there is an error
	let mut original_reserved: BTreeMap<AccountId32, BalanceOf<Polkadot>> = BTreeMap::new();
	let orig_len = para_ids.len();
	let mut refund_times: BTreeMap<ParaId, BlockNumberFor<Polkadot>> = BTreeMap::new();

	frame_support::hypothetically!({
		while !para_ids.is_empty() && cutoff > 0 {
			let now = frame_system::Pallet::<Polkadot>::block_number();

			para_ids = para_ids
				.into_iter()
				.filter(|para_id| {
					let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
					let slots = pallet_slots::Leases::<Polkadot>::get(para_id);
					// Lease pot account can either be a crowdloan account or a solo bidder
					log::info!("[{now}] Para {} has {} slots", para_id, slots.len());
					let Some(last_lease) = slots.last().cloned() else {
						// TODO check this
						// TODO additional check with crowdloan account has no reserve
						log::info!(
							"[{now}] Para {} has no slots and already had its funds unreserved",
							para_id
						);
						// TODO https://polkadot.subsquare.io/referenda/524
						if *para_id == ParaId::from(3356) {
							return false;
						}
						refund_times.insert(*para_id, now);
						// TODO some additional checks i forgot about
						// Account must have at least `rased` in free funds
						let pot =
							pallet_crowdloan::Pallet::<Polkadot>::fund_account_id(fund.fund_index);
						let pot_acc = frame_system::Account::<Polkadot>::get(&pot);
						if pot_acc.data.free < fund.raised {
							panic!(
								"Para {} has {} raised but only {} free",
								para_id, fund.raised, pot_acc.data.free
							);
						}
						return false;
					};

					let Some((lease_pot_account, deposit_amount)) = last_lease else {
						frame_support::defensive!("Last lease should never be None");
						return false;
					};

					let reserved =
						pallet_balances::Pallet::<Polkadot>::reserved_balance(&lease_pot_account);
					let original_res =
						original_reserved.entry(lease_pot_account).or_insert(reserved);
					if reserved < *original_res {
						log::info!(
							"[{}] Lease funds of para {} can be withdrawn, reserved: {} -> {}",
							now,
							para_id,
							*original_res,
							reserved
						);
						assert_eq(*original_res - reserved, deposit_amount);
						assert_eq(fund.raised, 0);
						// TODO additional checks if there is a crowdloan, then it should be zero
						return false;
					}

					true
				})
				.collect();

			// Go to the start of the next Lease period
			let offset = <Polkadot as pallet_slots::Config>::LeaseOffset::get();
			let period = <Polkadot as pallet_slots::Config>::LeasePeriod::get();
			let next_period_start = ((now - offset) / period) * period + period + offset;

			run_to_block(next_period_start);
			cutoff -= 1;
		}

		if !para_ids.is_empty() {
			panic!("Some crowdloans could not be dissolved: {:?}", para_ids);
		}
	});
	// TODO -1 for the Bifrost lease swap 3356 mentioned above
	assert_eq!(orig_len - 1, refund_times.len());
	refund_times
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
		run_to_block(at);

		let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
		let origin = polkadot_runtime::RuntimeOrigin::signed(alice);
		let call: polkadot_runtime::RuntimeCall =
			pallet_crowdloan::Call::<Polkadot>::refund { index: para_id }.into();
		call.dispatch_bypass_filter(origin).unwrap(); // Why bypass?
	});
}

fn run_to_block(at: u32) {
	//let mut bn = frame_system::Pallet::<Polkadot>::block_number();

	frame_system::Pallet::<Polkadot>::set_block_number(at);
	//polkadot_runtime::AllPalletsWithSystem::on_initialize(bn);
	<pallet_registrar::Pallet<Polkadot> as frame_support::traits::OnInitialize<
		BlockNumberFor<Polkadot>,
	>>::on_initialize(at);
	<pallet_slots::Pallet<Polkadot> as frame_support::traits::OnInitialize<
		BlockNumberFor<Polkadot>,
	>>::on_initialize(at);
	<pallet_crowdloan::Pallet<Polkadot> as frame_support::traits::OnInitialize<
		BlockNumberFor<Polkadot>,
	>>::on_initialize(at);
	<frame_system::Pallet<Polkadot> as frame_support::traits::OnFinalize<
		BlockNumberFor<Polkadot>,
	>>::on_finalize(at);
}

fn ensure_can_dissolve(para_id: ParaId) {
	frame_support::hypothetically!({
		let fund = pallet_crowdloan::Funds::<Polkadot>::get(para_id).unwrap();
		let sender = fund.depositor;
		println!("fund index: {}, sender: {}, deposit: {}", fund.fund_index, sender, fund.deposit);
		let data_before = frame_system::Account::<Polkadot>::get(&sender).data;
		let origin = polkadot_runtime::RuntimeOrigin::signed(sender.clone());

		let call: polkadot_runtime::RuntimeCall =
			pallet_crowdloan::Call::<Polkadot>::dissolve { index: para_id }.into();
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

/// Assert that also works without debug_assert
fn assert_eq<R: PartialEq + core::fmt::Debug>(a: R, b: R) {
	if a != b {
		panic!("{a:?} != {b:?}");
	}
}
