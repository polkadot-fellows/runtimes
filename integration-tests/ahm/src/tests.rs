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
use polkadot_runtime_common::crowdloan as pallet_crowdloan;
use polkadot_runtime_common::paras_registrar;

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

/// Check that our function to calculate the unlock time of a crowdloan contribution is correct.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn crowdloan_unlock_times_are_correct_works() {
	let mut rc = remote_ext_test_setup::<PolkadotBlock>("SNAP_RC").await.unwrap() else { return };
	let para_id = ParaId::from(1000);
	let mut wasted = 0;

	rc.execute_with(|| {
		for para in paras_registrar::Paras::<Polkadot>::iter_keys() {
			let id: u32 = para.into();
			let acala_fund_id = pallet_crowdloan::Pallet::<Polkadot>::fund_account_id(id);

			let acc = frame_system::Account::<Polkadot>::get(&acala_fund_id).data;
			if acc.free > 0 && acc.reserved == 0 {
				println!("Para: {}, Fund id: {} with {} free", id, acala_fund_id, acc.free);
				wasted += acc.free;
			}
		}

		println!("Wasted: {}", wasted);
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
