// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Three-chain test harness: snapshot loading, manual block production and manual DMP/UMP
//! message shuttling between the Relay Chain and its two migration counterparts (Coretime chain
//! and Asset Hub). Chain-specific wiring lives behind the [`Para`] trait, so adding a chain
//! means adding one impl, not another copy of the plumbing.

use codec::{Decode, Encode};
use cumulus_primitives_core::{
	AggregateMessageOrigin as ParachainMessageOrigin, InboundDownwardMessage, ParaId,
	UpwardMessageSender,
};
use frame_support::{
	dispatch::GetDispatchInfo,
	traits::{EnqueueMessage, Get, OnFinalize, OnInitialize, ProcessMessage},
	weights::Weight,
};
use frame_system::pallet_prelude::BlockNumberFor;
use polkadot_primitives::UpwardMessage;
/// The three runtimes under test, chosen by the `kusama` feature.
///
/// Everything else in this crate goes through these aliases, so the suite is written once and runs
/// against either network. Nothing here may name a network directly.
#[cfg(not(feature = "kusama"))]
pub mod network {
	pub use asset_hub_polkadot_runtime as ah;
	pub use coretime_polkadot_runtime as ct;
	pub use polkadot_runtime as relay;
	pub use polkadot_runtime_constants as constants;

	pub const NAME: &str = "Polkadot";
	pub const RELAY_RPC: &str = "wss://try-runtime.polkadot.io:443";
	pub const AH_RPC: &str = "wss://polkadot-asset-hub-rpc.polkadot.io:443";
	pub const CT_RPC: &str = "wss://polkadot-coretime-rpc.polkadot.io:443";
}

#[cfg(feature = "kusama")]
pub mod network {
	pub use asset_hub_kusama_runtime as ah;
	pub use coretime_kusama_runtime as ct;
	pub use kusama_runtime as relay;
	pub use kusama_runtime_constants as constants;

	pub const NAME: &str = "Kusama";
	pub const RELAY_RPC: &str = "wss://kusama-try-runtime-node.parity-chains.parity.io:443";
	pub const AH_RPC: &str = "wss://kusama-asset-hub-rpc.polkadot.io:443";
	pub const CT_RPC: &str = "wss://kusama-coretime-rpc.polkadot.io:443";
}

use network::relay::{Block as RelayBlock, Runtime as Polkadot};
use network::constants::system_parachain;
use remote_externalities::{Builder, Mode, OfflineConfig};
use runtime_parachains::{
	configuration::ActiveConfig,
	dmp::{self, DownwardMessageQueues},
	inclusion::{AggregateMessageOrigin as RcMessageOrigin, UmpQueueId},
};
use sp_core::H256;
use sp_io::TestExternalities;
use sp_runtime::{traits::One, BoundedVec};
use tokio::sync::OnceCell;
use xcm::{
	latest::prelude::{Instruction, Xcm},
	VersionedXcm,
};

const LOG_RC: &str = "runtime::relay";

pub type RuntimeCallFor<P> = <<P as Para>::Runtime as frame_system::Config>::RuntimeCall;
type MqPallet<P> = pallet_message_queue::Pallet<<P as Para>::Runtime>;

/// A parachain that takes part in the migration.
///
/// Everything chain-specific that the harness needs lives here; the block production and message
/// shuttling functions are generic over it.
/// The event bounds are satisfied by the `TryInto<pallet::Event>` impls that `construct_runtime`
/// generates for every runtime; no per-chain event-matching code is needed.
pub trait Para {
	type Runtime: frame_system::Config<
			RuntimeEvent: TryInto<pallet_message_queue::Event<Self::Runtime>>
			                  + TryInto<frame_system::Event<Self::Runtime>>,
			RuntimeCall: GetDispatchInfo,
		> + pallet_message_queue::Config<
			MessageProcessor: ProcessMessage<Origin = ParachainMessageOrigin>,
		> + cumulus_pallet_parachain_system::Config;
	const PARA_ID: u32;
	/// Used as log target and in assertion messages.
	const NAME: &'static str;
	/// Which snapshot this chain loads from.
	const CHAIN: Chain;
}

pub struct AssetHubPara;
impl Para for AssetHubPara {
	type Runtime = network::ah::Runtime;
	const PARA_ID: u32 = system_parachain::ASSET_HUB_ID;
	const NAME: &'static str = "runtime::asset-hub";
	const CHAIN: Chain = Chain::AssetHub;
}

pub struct CoretimePara;
impl Para for CoretimePara {
	type Runtime = network::ct::Runtime;
	const PARA_ID: u32 = system_parachain::BROKER_ID;
	const NAME: &'static str = "runtime::coretime";
	const CHAIN: Chain = Chain::Coretime;
}

// ---------------------------------------------------------------------------
// Snapshot loading
// ---------------------------------------------------------------------------

/// Raw key-value snapshot plus state root, as produced by `try-runtime create-snapshot`.
///
/// Cached in this form because `TestExternalities` is not `Clone`; each test re-hydrates its own
/// externalities from the cached raw snapshot.
pub type RawSnapshot = (Vec<(Vec<u8>, (Vec<u8>, i32))>, H256);

static RC_CACHE: OnceCell<RawSnapshot> = OnceCell::const_new();
static AH_CACHE: OnceCell<RawSnapshot> = OnceCell::const_new();
static CT_CACHE: OnceCell<RawSnapshot> = OnceCell::const_new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chain {
	Relay,
	AssetHub,
	Coretime,
}

impl Chain {
	pub const fn name(self) -> &'static str {
		match self {
			Chain::Relay => network::NAME,
			Chain::AssetHub => "Asset Hub",
			Chain::Coretime => "Coretime",
		}
	}

	pub const fn snap_env(self) -> &'static str {
		match self {
			Chain::Relay => "SNAP_RC",
			Chain::AssetHub => "SNAP_AH",
			Chain::Coretime => "SNAP_CT",
		}
	}

	/// Public RPC endpoint, used in error messages to tell the developer how to create a missing
	/// snapshot. Must stay in sync with the `chains` table in the justfile.
	pub const fn rpc(self) -> &'static str {
		match self {
			Chain::Relay => network::RELAY_RPC,
			Chain::AssetHub => network::AH_RPC,
			Chain::Coretime => network::CT_RPC,
		}
	}

	fn cache(self) -> &'static OnceCell<RawSnapshot> {
		match self {
			Chain::Relay => &RC_CACHE,
			Chain::AssetHub => &AH_CACHE,
			Chain::Coretime => &CT_CACHE,
		}
	}

	fn missing_snapshot_help(self) -> String {
		format!(
			"\n\nSnapshot for the {} chain is missing or unreadable.\n\
			Run `just snapshots` in integration-tests/minimal-relay to download all three chains\n\
			from the fellowship CI (or `just test`, which fetches them automatically).\n\
			Alternatively create this one from an RPC node and point the {} env var at it:\n\n    \
			try-runtime create-snapshot --uri={} {}.snap\n",
			self.name(),
			self.snap_env(),
			self.rpc(),
			self.snap_env().to_lowercase(),
		)
	}
}

/// Load the externalities of one chain from its snapshot.
///
/// Panics with instructions if the snapshot is not available: a missing snapshot must fail the
/// test loudly, never skip it.
pub async fn remote_ext(chain: Chain) -> TestExternalities {
	sp_tracing::try_init_simple();
	let snapshot = chain
		.cache()
		.get_or_init(|| async move { load_snapshot_uncached(chain).await })
		.await;
	TestExternalities::from_raw_snapshot(
		snapshot.0.clone(),
		snapshot.1,
		sp_storage::StateVersion::V1,
	)
}

async fn load_snapshot_uncached(chain: Chain) -> RawSnapshot {
	let path = std::env::var(chain.snap_env())
		.unwrap_or_else(|_| panic!("{}", chain.missing_snapshot_help()));
	let abs = std::path::absolute(&path).expect("Could not get absolute path");
	assert!(abs.exists(), "No file at {}.{}", abs.display(), chain.missing_snapshot_help());

	log::info!("Loading {} snapshot from {}", chain.name(), abs.display());
	// The `Block` type is only used for header decoding in online mode; `RelayBlock` works for
	// all three chains when loading offline snapshots.
	let ext = Builder::<RelayBlock>::default()
		.mode(Mode::Offline(OfflineConfig { state_snapshot: abs.display().to_string().into() }))
		.build()
		.await
		.unwrap_or_else(|e| {
			panic!("Corrupt snapshot at {}: {e:?}{}", abs.display(), chain.missing_snapshot_help())
		});

	ext.inner_ext.into_raw_snapshot()
}

/// Load one chain on a worker thread, so `tokio::join!`-ed loads actually run in parallel
/// (snapshot hydration is CPU-bound).
pub async fn load(chain: Chain) -> TestExternalities {
	tokio::spawn(remote_ext(chain))
		.await
		.unwrap_or_else(|e| panic!("failed to load the {} snapshot: {e}", chain.name()))
}

/// Load all three chains in parallel.
pub async fn load_externalities() -> (TestExternalities, TestExternalities, TestExternalities) {
	tokio::join!(load(Chain::Relay), load(Chain::AssetHub), load(Chain::Coretime))
}

// ---------------------------------------------------------------------------
// Block production
// ---------------------------------------------------------------------------

/// Execute the next Relay Chain block.
///
/// Only runs the hooks the tests rely on — `MessageQueue`, so inbound messages are processed.
/// The migration stage PRs extend this with their pallet's hooks as they land.
pub fn next_block_rc() {
	next_block::<Polkadot>(LOG_RC, |now| {
		let weight = <network::relay::MessageQueue as OnInitialize<_>>::on_initialize(now);
		<network::relay::MessageQueue as OnFinalize<_>>::on_finalize(now);
		weight
	});
}

/// Execute the next block on parachain `P`. Same hooks and assertions as [`next_block_rc`].
pub fn next_block_para<P: Para>() {
	next_block::<P::Runtime>(P::NAME, |now| {
		let weight = <MqPallet<P> as OnInitialize<_>>::on_initialize(now);
		<MqPallet<P> as OnFinalize<_>>::on_finalize(now);
		weight
	});
}

/// Shared block-execution skeleton: bump the block number, reset events, run the chain's hooks,
/// then assert that no message failed processing and that the consumed weight stays below 80% of
/// the block limit. The per-block assertions live here, in one place, so they cannot drift apart
/// between the chains.
fn next_block<T>(name: &str, hooks: impl FnOnce(BlockNumberFor<T>) -> Weight)
where
	T: frame_system::Config + pallet_message_queue::Config,
	<T as frame_system::Config>::RuntimeEvent: TryInto<pallet_message_queue::Event<T>>,
{
	let now = frame_system::Pallet::<T>::block_number() + One::one();
	log::debug!(target: name, "Executing block: {now:?}");
	frame_system::Pallet::<T>::set_block_number(now);
	frame_system::Pallet::<T>::reset_events();
	let weight = hooks(now);

	for record in frame_system::Pallet::<T>::events() {
		if let Ok(pallet_message_queue::Event::Processed { success: false, .. }) =
			record.event.clone().try_into()
		{
			panic!("{name}: message processing failure: {:?}", record.event);
		}
	}

	let limit = <T as frame_system::Config>::BlockWeights::get().max_block;
	assert!(
		weight.all_lte(weight_threshold(limit)),
		"{name}: weight exceeded 80% of limit: {weight:?}, limit: {limit:?}"
	);
}

/// 80% of the given weight limit; blocks that consume more than this are considered overweight.
fn weight_threshold(limit: Weight) -> Weight {
	Weight::from_parts(limit.ref_time() / 5 * 4, limit.proof_size() / 5 * 4)
}

// ---------------------------------------------------------------------------
// Message shuttling
// ---------------------------------------------------------------------------

/// Queue a DMP message on the Relay Chain destined for `para`.
///
/// This is the same code path that the RC-side XCM router uses, so anything queued here is
/// indistinguishable from a message sent by a pallet on the RC.
pub fn send_dmp(para: ParaId, xcm: Xcm<()>) {
	let config = ActiveConfig::<Polkadot>::get();
	dmp::Pallet::<Polkadot>::queue_downward_message(
		&config,
		para,
		VersionedXcm::from(xcm).encode(),
	)
	.expect("can queue DMP message");
}

/// Send an UMP message from parachain `P` to the Relay Chain.
pub fn send_ump<P: Para>(xcm: Xcm<()>) {
	<cumulus_pallet_parachain_system::Pallet<P::Runtime> as UpwardMessageSender>::send_upward_message(
		VersionedXcm::from(xcm).encode(),
	)
	.expect("can send UMP message");
}

/// Take all DMP messages that the Relay Chain has queued for `para`.
pub fn take_dmp(para: ParaId) -> Vec<InboundDownwardMessage> {
	DownwardMessageQueues::<Polkadot>::take(para)
}

/// Take all UMP messages that parachain `P` has queued for the Relay Chain.
pub fn take_ump<P: Para>() -> Vec<UpwardMessage> {
	cumulus_pallet_parachain_system::PendingUpwardMessages::<P::Runtime>::take()
}

/// Enqueue DMP messages on the message queue of parachain `P`.
///
/// Bypasses `set_validation_data` and `enqueue_inbound_downward_messages` by enqueuing directly.
pub fn enqueue_dmp<P: Para>(msgs: Vec<InboundDownwardMessage>) {
	log::info!(target: P::NAME, "Received {} DMP messages from RC", msgs.len());
	for msg in msgs {
		sanity_check_xcm::<RuntimeCallFor<P>>(&msg.msg);

		let bounded: BoundedVec<
			u8,
			<MqPallet<P> as EnqueueMessage<ParachainMessageOrigin>>::MaxMessageLen,
		> = msg.msg.try_into().expect("DMP message too big");
		MqPallet::<P>::enqueue_message(bounded.as_bounded_slice(), ParachainMessageOrigin::Parent);
	}
}

/// Enqueue UMP messages from `para` on the Relay Chain message queue.
pub fn enqueue_ump(para: ParaId, msgs: Vec<UpwardMessage>) {
	log::info!(target: LOG_RC, "Received {} UMP messages from para {}", msgs.len(), u32::from(para));
	for msg in msgs {
		sanity_check_xcm::<network::relay::RuntimeCall>(&msg);

		let bounded: BoundedVec<u8, _> = msg.try_into().expect("UMP message too big");
		network::relay::MessageQueue::enqueue_message(
			bounded.as_bounded_slice(),
			RcMessageOrigin::Ump(UmpQueueId::Para(para)),
		);
	}
}

/// Decode a forwarded XCM and, for every `Transact` in it, check that the receiving runtime can
/// decode the inner call. This is what catches encode/decode drift between the chains.
fn sanity_check_xcm<Call: Decode + GetDispatchInfo>(msg: &[u8]) {
	let versioned = VersionedXcm::<Call>::decode(&mut &msg[..]).expect("Must decode forwarded XCM");
	let xcm: Xcm<Call> =
		versioned.try_into().expect("Must convert forwarded XCM to latest version");
	for instruction in xcm.0 {
		if let Instruction::Transact { call, .. } = instruction {
			let _call: Call = Decode::decode(&mut &call.into_encoded()[..])
				.expect("Receiving runtime must decode the Transact call");
		}
	}
}
