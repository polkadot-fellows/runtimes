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

#[cfg(feature = "kusama-ahm")]
use crate::porting_prelude::*;

use asset_hub_polkadot_runtime::{AhMigrator, Runtime as AssetHub, RuntimeEvent as AhRuntimeEvent};
use codec::Decode;
use cumulus_primitives_core::{
	AggregateMessageOrigin as ParachainMessageOrigin, InboundDownwardMessage, ParaId,
};
use frame_support::traits::{EnqueueMessage, OnFinalize, OnInitialize, QueueFootprintQuery};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_rc_migrator::{
	MigrationStage as RcMigrationStage, MigrationStageOf as RcMigrationStageOf,
	RcMigrationStage as RcMigrationStageStorage,
};
use polkadot_primitives::UpwardMessage;
use polkadot_runtime::{
	Block as PolkadotBlock, RcMigrator, Runtime as Polkadot, RuntimeEvent as RcRuntimeEvent,
};
use remote_externalities::{Builder, Mode, OfflineConfig};
use runtime_parachains::{
	dmp::DownwardMessageQueues,
	inclusion::{AggregateMessageOrigin as RcMessageOrigin, UmpQueueId},
};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{BoundedVec, BuildStorage, Perbill};
use std::str::FromStr;
use tokio::sync::OnceCell;
use xcm::prelude::*;

pub const AH_PARA_ID: ParaId = ParaId::new(1000);
const LOG_RC: &str = "runtime::relay";
const LOG_AH: &str = "runtime::asset-hub";

pub enum Chain {
	Relay,
	AssetHub,
}

impl std::fmt::Display for Chain {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match self {
				Chain::Relay => "SNAP_RC",
				Chain::AssetHub => "SNAP_AH",
			}
		)
	}
}

pub type Snapshot = (Vec<(Vec<u8>, (Vec<u8>, i32))>, sp_core::H256);

static RC_CACHE: OnceCell<Snapshot> = OnceCell::const_new();
static AH_CACHE: OnceCell<Snapshot> = OnceCell::const_new();

/// Load Relay and AH externalities in parallel.
pub async fn load_externalities() -> Option<(TestExternalities, TestExternalities)> {
	let (rc, ah) = tokio::try_join!(
		tokio::spawn(async { remote_ext_test_setup(Chain::Relay).await }),
		tokio::spawn(async { remote_ext_test_setup(Chain::AssetHub).await })
	)
	.ok()?;
	Some((rc?, ah?))
}

pub type RawSnapshot = (Vec<(Vec<u8>, (Vec<u8>, i32))>, H256);

pub async fn remote_ext_test_setup(chain: Chain) -> Option<TestExternalities> {
	sp_tracing::try_init_simple();
	log::info!("Checking {chain} snapshot cache");

	let cache = match chain {
		Chain::Relay => &RC_CACHE,
		Chain::AssetHub => &AH_CACHE,
	};

	let snapshot = cache
		.get_or_init(|| async {
			let path = std::env::var(chain.to_string()).expect("Env var not set");
			load_snapshot_uncached(&path).await
		})
		.await;
	let ext = snapshot_to_externalities(snapshot);

	Some(ext)
}

pub async fn load_snapshot_uncached(path: &str) -> RawSnapshot {
	log::info!("Loading snapshot from {path}");
	let abs = std::path::absolute(path).expect("Could not get absolute path");

	let ext = Builder::<PolkadotBlock>::default()
		.mode(Mode::Offline(OfflineConfig { state_snapshot: abs.display().to_string().into() }))
		.build()
		.await
		.map_err(|e| {
			eprintln!("Could not load from snapshot: {abs:?}: {e:?}");
		})
		.unwrap();

	// `RemoteExternalities` and `TestExternalities` types cannot be cloned so we need to
	// convert them to raw snapshot and store it in the cache.
	ext.inner_ext.into_raw_snapshot()
}

pub fn snapshot_to_externalities(snapshot: &RawSnapshot) -> TestExternalities {
	TestExternalities::from_raw_snapshot(
		snapshot.0.clone(),
		snapshot.1,
		sp_storage::StateVersion::V1,
	)
}

pub async fn load_externalities_uncached(env_var: &str) -> Option<TestExternalities> {
	let path = std::env::var(env_var).expect("Env var not set");
	let snapshot = load_snapshot_uncached(&path).await;
	Some(snapshot_to_externalities(&snapshot))
}

pub fn next_block_rc() {
	let past = frame_system::Pallet::<Polkadot>::block_number();
	let now = past + 1;
	log::debug!(target: LOG_RC, "Executing RC block: {now:?}");
	frame_system::Pallet::<Polkadot>::set_block_number(now);
	frame_system::Pallet::<Polkadot>::reset_events();
	let weight = <polkadot_runtime::MessageQueue as OnInitialize<_>>::on_initialize(now);
	let weight = <RcMigrator as OnInitialize<_>>::on_initialize(now).saturating_add(weight);
	<polkadot_runtime::MessageQueue as OnFinalize<_>>::on_finalize(now);
	<RcMigrator as OnFinalize<_>>::on_finalize(now);

	let events = frame_system::Pallet::<Polkadot>::events();
	assert!(
		!events.iter().any(|record| {
			if matches!(
				record.event,
				RcRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed {
					success: false,
					..
				})
			) {
				log::error!(target: LOG_RC, "Message processing error: {events:?}");
				true
			} else {
				false
			}
		}),
		"unexpected xcm message processing failure",
	);

	let limit = <Polkadot as frame_system::Config>::BlockWeights::get().max_block;
	assert!(
		weight.all_lte(Perbill::from_percent(80) * limit),
		"Weight exceeded 80% of limit: {weight:?}, limit: {limit:?}"
	);
}

pub fn next_block_ah() {
	let past = frame_system::Pallet::<AssetHub>::block_number();
	let now = past + 1;
	log::debug!(target: LOG_AH, "Executing AH block: {now:?}");
	frame_system::Pallet::<AssetHub>::set_block_number(now);
	frame_system::Pallet::<AssetHub>::reset_events();
	let weight = <asset_hub_polkadot_runtime::MessageQueue as OnInitialize<_>>::on_initialize(now);
	let weight = <AhMigrator as OnInitialize<_>>::on_initialize(now).saturating_add(weight);
	<asset_hub_polkadot_runtime::MessageQueue as OnFinalize<_>>::on_finalize(now);
	<AhMigrator as OnFinalize<_>>::on_finalize(now);

	let events = frame_system::Pallet::<AssetHub>::events();
	assert!(
		!events.iter().any(|record| {
			if matches!(
				record.event,
				AhRuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed {
					success: false,
					..
				})
			) {
				log::error!(target: LOG_AH, "Message processing error: {events:?}");
				true
			} else {
				false
			}
		}),
		"unexpected xcm message processing failure",
	);

	let limit = <AssetHub as frame_system::Config>::BlockWeights::get().max_block;
	assert!(
		weight.all_lte(Perbill::from_percent(80) * limit),
		"Weight exceeded 80% of limit: {weight:?}, limit: {limit:?}"
	);
}

/// Enqueue DMP messages on the parachain side.
///
/// This bypasses `set_validation_data` and `enqueue_inbound_downward_messages` by just directly
/// enqueuing them.
///
/// The block number parameter indicates the block at which these messages were sent. It may be set
/// to zero when the block number information is not important for a test.
pub fn enqueue_dmp(msgs: (Vec<InboundDownwardMessage>, BlockNumberFor<Polkadot>)) {
	log::info!(target: LOG_AH, "Received {} DMP messages from RC block {}", msgs.0.len(), msgs.1);
	for msg in msgs.0 {
		sanity_check_xcm::<asset_hub_polkadot_runtime::RuntimeCall>(&msg.msg);

		let bounded_msg: BoundedVec<u8, _> = msg.msg.try_into().expect("DMP message too big");
		asset_hub_polkadot_runtime::MessageQueue::enqueue_message(
			bounded_msg.as_bounded_slice(),
			ParachainMessageOrigin::Parent,
		);
	}
}

/// Enqueue UMP messages on the Relay Chain side.
///
/// The block number parameter indicates the block at which these messages were sent. It may be set
/// to zero when the block number information is not important for a test.
pub fn enqueue_ump(msgs: (Vec<UpwardMessage>, BlockNumberFor<AssetHub>)) {
	log::info!(target: LOG_RC, "Received {} UMP messages from AH block {}", msgs.0.len(), msgs.1);
	for msg in msgs.0 {
		sanity_check_xcm::<polkadot_runtime::RuntimeCall>(&msg);

		let bounded_msg: BoundedVec<u8, _> = msg.try_into().expect("UMP message too big");
		polkadot_runtime::MessageQueue::enqueue_message(
			bounded_msg.as_bounded_slice(),
			RcMessageOrigin::Ump(UmpQueueId::Para(AH_PARA_ID)),
		);
	}
}

fn sanity_check_xcm<Call: Decode>(msg: &[u8]) {
	let xcm = xcm::VersionedXcm::<Call>::decode(&mut &msg[..]).expect("Must decode DMP XCM");
	match xcm {
		VersionedXcm::V3(inner) =>
			for instruction in inner.0 {
				if let xcm::v3::Instruction::Transact { call, .. } = instruction {
					// Interesting part here: ensure that the receiving runtime can decode the
					// call
					let _call: Call = Decode::decode(&mut &call.into_encoded()[..])
						.expect("Must decode DMP XCM call");
				}
			},
		VersionedXcm::V4(inner) =>
			for instruction in inner.0 {
				if let xcm::v4::Instruction::Transact { call, .. } = instruction {
					// Interesting part here: ensure that the receiving runtime can decode the
					// call
					let _call: Call = Decode::decode(&mut &call.into_encoded()[..])
						.expect("Must decode DMP XCM call");
				}
			},
		VersionedXcm::V5(inner) =>
			for instruction in inner.0 {
				if let xcm::v5::Instruction::Transact { call, .. } = instruction {
					// Interesting part here: ensure that the receiving runtime can decode the
					// call
					let _call: Call = Decode::decode(&mut &call.into_encoded()[..])
						.expect("Must decode DMP XCM call");
				}
			},
	};
}

// Sets the initial migration stage on the Relay Chain.
//
// If the `START_STAGE` environment variable is set, it will be used to set the initial migration
// stage. Otherwise, the `AccountsMigrationInit` stage will be set bypassing the `Scheduled` stage.
// The `Scheduled` stage is tested separately by the `scheduled_migration_works` test.
pub fn set_initial_migration_stage(
	relay_chain: &mut TestExternalities,
	maybe_stage: Option<RcMigrationStageOf<Polkadot>>,
) -> RcMigrationStageOf<Polkadot> {
	let stage = relay_chain.execute_with(|| {
		let stage = if let Some(stage) = maybe_stage {
			stage
		} else if let Ok(stage) = std::env::var("START_STAGE") {
			log::info!("Setting start stage: {:?}", &stage);
			RcMigrationStage::from_str(&stage).expect("Invalid start stage")
		} else {
			RcMigrationStage::Scheduled { start: 0u32 }
		};
		RcMigrationStageStorage::<Polkadot>::put(stage.clone());
		stage
	});
	relay_chain.commit_all().unwrap();
	stage
}

// Migrates the pallet out of the Relay Chain and returns the corresponding Payload.
//
// Sends DMP messages with pallet migration data from relay chain to asset hub. The output includes
// both the DMP messages sent from the relay chain to asset hub, which will be used to perform the
// migration, and the relay chain payload, which will be used to check the correctness of the
// migration process.
pub fn rc_migrate(relay_chain: &mut TestExternalities) -> Vec<InboundDownwardMessage> {
	// AH parachain ID
	let para_id = ParaId::from(1000);

	// Simulate relay blocks and grab the DMP messages
	let dmp_messages = relay_chain.execute_with(|| {
		let mut dmps = Vec::new();

		// Loop until no more DMPs are added and we had at least 1
		loop {
			next_block_rc();

			let new_dmps = DownwardMessageQueues::<Polkadot>::take(para_id);
			dmps.extend(new_dmps);

			match RcMigrationStageStorage::<Polkadot>::get() {
				RcMigrationStage::WaitingForAh => {
					log::info!("Migration waiting for AH signal");
					break dmps;
				},
				RcMigrationStage::MigrationDone => {
					log::info!("Migration done");
					break dmps;
				},
				_ => (),
			}
		}
	});
	relay_chain.commit_all().unwrap();
	log::info!("Num of RC->AH DMP messages: {}", dmp_messages.len());
	dmp_messages
}

// Migrates the pallet into Asset Hub.
//
// Processes all the pending DMP messages in the AH message queue to complete the pallet
// migration. Uses the relay chain pre-migration payload to check the correctness of the
// migration once completed.
pub fn ah_migrate(asset_hub: &mut TestExternalities, dmp_messages: Vec<InboundDownwardMessage>) {
	// Inject the DMP messages into the Asset Hub
	asset_hub.execute_with(|| {
		let mut fp =
			asset_hub_polkadot_runtime::MessageQueue::footprint(ParachainMessageOrigin::Parent);
		enqueue_dmp((dmp_messages, 0u32));

		// Loop until no more DMPs are queued
		loop {
			let new_fp =
				asset_hub_polkadot_runtime::MessageQueue::footprint(ParachainMessageOrigin::Parent);
			if fp == new_fp {
				log::info!("AH DMP messages left: {}", fp.storage.count);
				break;
			}
			fp = new_fp;

			log::debug!("AH DMP messages left: {}", fp.storage.count);
			next_block_ah();
		}

		// NOTE that the DMP queue is probably not empty because the snapshot that we use
		// contains some overweight ones.
		// TODO: @re-gius compare with the number of messages before the migration
	});
	asset_hub.commit_all().unwrap();
}

pub fn new_test_rc_ext() -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();

	let mut t = frame_system::GenesisConfig::<Polkadot>::default().build_storage().unwrap();

	pallet_xcm::GenesisConfig::<Polkadot> {
		safe_xcm_version: Some(xcm::latest::VERSION),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| frame_system::Pallet::<Polkadot>::set_block_number(1));
	ext
}

pub fn new_test_ah_ext() -> sp_io::TestExternalities {
	sp_tracing::try_init_simple();

	let mut t = frame_system::GenesisConfig::<AssetHub>::default().build_storage().unwrap();

	pallet_xcm::GenesisConfig::<AssetHub> {
		safe_xcm_version: Some(xcm::latest::VERSION),
		..Default::default()
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| frame_system::Pallet::<AssetHub>::set_block_number(1));
	ext
}
