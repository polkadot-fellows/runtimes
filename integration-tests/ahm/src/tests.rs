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
use frame_support::{pallet_prelude::*, traits::*, weights::WeightMeter};
use pallet_rc_migrator::RcMigrationStage;
use polkadot_runtime::RcMigrator;
use sp_core::H256;
use sp_io::TestExternalities;
use sp_storage::StateVersion;
use std::cell::OnceCell;

use asset_hub_polkadot_runtime::{Block as AssetHubBlock, Runtime as AssetHub};
use polkadot_runtime::{Block as PolkadotBlock, Runtime as Polkadot};

use super::mock::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn account_migration_works() {
	let Some((mut rc, mut ah)) = load_externalities().await else { return };

	// Simulate 10 relay blocks and grab the DMP messages
	let dmp_messages = rc.execute_with(|| {
		for _ in 0..10 {
			next_block_rc();
		}

		// DMP:
		let para_id = ParaId::from(1000);
		runtime_parachains::dmp::DownwardMessageQueues::<Polkadot>::take(para_id)
	});
	rc.commit_all().unwrap();
	log::info!("Num of RC->AH DMP messages: {}", dmp_messages.len());

	// Inject the DMP messages into the Asset Hub
	ah.execute_with(|| {
		// We bypass `set_validation_data` and `enqueue_inbound_downward_messages` by just directly
		// enqueuing them.
		for msg in dmp_messages {
			let bounded_msg: BoundedVec<u8, _> = msg.msg.try_into().expect("DMP message too big");
			asset_hub_polkadot_runtime::MessageQueue::enqueue_message(
				bounded_msg.as_bounded_slice(),
				AggregateMessageOrigin::Parent,
			);
		}

		for _ in 0..10 {
			next_block_ah();
		}
	});
}
