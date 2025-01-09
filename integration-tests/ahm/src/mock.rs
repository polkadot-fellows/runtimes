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

use asset_hub_polkadot_runtime::Block as AssetHubBlock;
use cumulus_primitives_core::{AggregateMessageOrigin, InboundDownwardMessage};
use frame_support::traits::EnqueueMessage;
use polkadot_runtime::Block as PolkadotBlock;
use remote_externalities::{Builder, Mode, OfflineConfig, RemoteExternalities};
use sp_runtime::BoundedVec;

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
	let now = frame_system::Pallet::<polkadot_runtime::Runtime>::block_number();
	log::info!(target: LOG_RC, "Next block: {:?}", now + 1);
	<polkadot_runtime::RcMigrator as frame_support::traits::OnFinalize<_>>::on_finalize(now);
	frame_system::Pallet::<polkadot_runtime::Runtime>::set_block_number(now + 1);
	frame_system::Pallet::<polkadot_runtime::Runtime>::reset_events();
	<polkadot_runtime::RcMigrator as frame_support::traits::OnInitialize<_>>::on_initialize(
		now + 1,
	);
}

pub fn next_block_ah() {
	let now = frame_system::Pallet::<asset_hub_polkadot_runtime::Runtime>::block_number();
	log::info!(target: LOG_AH, "Next block: {:?}", now + 1);
	<asset_hub_polkadot_runtime::AhMigrator as frame_support::traits::OnFinalize<_>>::on_finalize(
		now,
	);
	frame_system::Pallet::<asset_hub_polkadot_runtime::Runtime>::set_block_number(now + 1);
	<asset_hub_polkadot_runtime::MessageQueue as frame_support::traits::OnInitialize<_>>::on_initialize(now + 1);
	frame_system::Pallet::<polkadot_runtime::Runtime>::reset_events();
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
