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

use std::str::FromStr;

use asset_hub_polkadot_runtime::Block as AssetHubBlock;
use cumulus_primitives_core::{AggregateMessageOrigin, InboundDownwardMessage, ParaId};
use frame_support::traits::EnqueueMessage;
use pallet_rc_migrator::{
	types::{AhPalletMigrationChecks, RcPalletMigrationChecks},
	MigrationStage, RcMigrationStage,
};
use remote_externalities::{Builder, Mode, OfflineConfig, RemoteExternalities};
use sp_runtime::BoundedVec;

use asset_hub_polkadot_runtime::Runtime as AssetHub;
use polkadot_runtime::{Block as PolkadotBlock, Runtime as Polkadot};

const LOG_RC: &str = "runtime::relay";
const LOG_AH: &str = "runtime::asset-hub";

/// Load Relay and AH externalities in parallel.
pub async fn load_externalities(
) -> Option<(RemoteExternalities<PolkadotBlock>, RemoteExternalities<AssetHubBlock>)> {
	let (rc, ah) = tokio::try_join!(
		tokio::spawn(async { remote_ext_test_setup::<PolkadotBlock>("SNAP_RC").await }),
		tokio::spawn(async { remote_ext_test_setup::<AssetHubBlock>("SNAP_AH").await })
	)
	.ok()?;
	Some((rc?, ah?))
}

pub async fn remote_ext_test_setup<Block: sp_runtime::traits::Block>(
	env: &str,
) -> Option<RemoteExternalities<Block>> {
	sp_tracing::try_init_simple();
	let snap = std::env::var(env).ok()?;
	let abs = std::path::absolute(snap.clone());

	let ext = Builder::<Block>::default()
		.mode(Mode::Offline(OfflineConfig { state_snapshot: snap.clone().into() }))
		.build()
		.await
		.map_err(|e| {
			eprintln!("Could not load from snapshot: {:?}: {:?}", abs, e);
		})
		.unwrap();

	Some(ext)
}

pub fn next_block_rc() {
	let now = frame_system::Pallet::<Polkadot>::block_number();
	log::debug!(target: LOG_RC, "Next block: {:?}", now + 1);
	<polkadot_runtime::RcMigrator as frame_support::traits::OnFinalize<_>>::on_finalize(now);
	frame_system::Pallet::<Polkadot>::set_block_number(now + 1);
	frame_system::Pallet::<Polkadot>::reset_events();
	<polkadot_runtime::RcMigrator as frame_support::traits::OnInitialize<_>>::on_initialize(
		now + 1,
	);
}

pub fn next_block_ah() {
	let now = frame_system::Pallet::<AssetHub>::block_number();
	log::debug!(target: LOG_AH, "Next block: {:?}", now + 1);
	<asset_hub_polkadot_runtime::AhMigrator as frame_support::traits::OnFinalize<_>>::on_finalize(
		now,
	);
	frame_system::Pallet::<AssetHub>::set_block_number(now + 1);
	<asset_hub_polkadot_runtime::MessageQueue as frame_support::traits::OnInitialize<_>>::on_initialize(now + 1);
	frame_system::Pallet::<Polkadot>::reset_events();
	<asset_hub_polkadot_runtime::AhMigrator as frame_support::traits::OnInitialize<_>>::on_initialize(now + 1);
}

/// Enqueue DMP messages on the parachain side.
///
/// This bypasses `set_validation_data` and `enqueue_inbound_downward_messages` by just directly
/// enqueuing them.
pub fn enqueue_dmp(msgs: Vec<InboundDownwardMessage>) {
	for msg in msgs {
		let bounded_msg: BoundedVec<u8, _> = msg.msg.try_into().expect("DMP message too big");
		asset_hub_polkadot_runtime::MessageQueue::enqueue_message(
			bounded_msg.as_bounded_slice(),
			AggregateMessageOrigin::Parent,
		);
	}
}

// Migrates the pallet out of the Relay Chain and returns the corresponding Payload.
//
// Sends DMP messages with pallet migration data from relay chain to asset hub. The output includes
// both the DMP messages sent from the relay chain to asset hub, which will be used to perform the
// migration, and the relay chain payload, which will be used to check the correctness of the
// migration process.
pub fn rc_migrate<RcMigrator: RcPalletMigrationChecks>(
	mut relay_chain: RemoteExternalities<PolkadotBlock>,
) -> (Vec<InboundDownwardMessage>, RcMigrator::RcPayload) {
	// AH parachain ID
	let para_id = ParaId::from(1000);

	// Simulate relay blocks and grab the DMP messages
	let (dmp_messages, rc_payload) = relay_chain.execute_with(|| {
		let mut dmps = Vec::new();

		if let Ok(stage) = std::env::var("START_STAGE") {
			let stage = MigrationStage::from_str(&stage).expect("Invalid start stage");
			RcMigrationStage::<Polkadot>::put(stage);
		}

		let rc_payload = RcMigrator::pre_check();

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
				break (dmps, rc_payload);
			}
		}
	});
	relay_chain.commit_all().unwrap();
	// TODO: for some reason this prints some small value (2947), but logs on XCM send and
	// receive show more iteration.
	log::info!("Num of RC->AH DMP messages: {}", dmp_messages.len());
	(dmp_messages, rc_payload)
}

// Migrates the pallet into Asset Hub.
//
// Processes all the pending DMP messages in the AH message queue to complete the pallet
// migration. Uses the relay chain pre-migration payload to check the correctness of the
// migration once completed.
pub fn ah_migrate<
	RcMigrator: RcPalletMigrationChecks,
	AhMigrator: AhPalletMigrationChecks<RcPayload = RcMigrator::RcPayload>,
>(
	mut asset_hub: RemoteExternalities<AssetHubBlock>,
	rc_payload: RcMigrator::RcPayload,
	dmp_messages: Vec<InboundDownwardMessage>,
) {
	// Inject the DMP messages into the Asset Hub
	asset_hub.execute_with(|| {
		let ah_payload = AhMigrator::pre_check();
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
				RcMigrator::post_check(rc_payload.clone());
			}
		}

		AhMigrator::post_check(ah_payload, rc_payload);
		// NOTE that the DMP queue is probably not empty because the snapshot that we use
		// contains some overweight ones.
		// TODO compare with the number of messages before the migration
	});
}
