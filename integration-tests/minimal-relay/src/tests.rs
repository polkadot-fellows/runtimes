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

//! Tests for the Minimal Relay migration.
//!
//! Every test that needs the Asset Hub snapshot (by far the largest, ~4 GB on Polkadot) carries
//! `asset_hub` in its name, so `--skip asset_hub` removes that dependency entirely.
//!
//! Tests use the multi-thread tokio runtime because [`load`] spawns snapshot hydration onto a
//! worker; on the default single-thread runtime, `tokio::join!`-ed loads would run one after the
//! other.

use crate::mock::*;
use codec::Encode;
use xcm::latest::prelude::*;

/// An XCM program that executes `call` on the destination with the sender's sovereign-account
/// origin. Both the RC and the system parachains grant each other unpaid execution, so no fee
/// payment is needed.
fn unpaid_transact<Call: Encode>(call: Call) -> Xcm<()> {
	Xcm(vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		Transact {
			origin_kind: OriginKind::SovereignAccount,
			fallback_max_weight: None,
			call: call.encode().into(),
		},
	])
}

// One block-production test per chain, so a lane that skips a chain skips exactly its tests.
// 10 blocks is enough for the message queues to drain whatever the live snapshot carries;
// `next_block_*` asserts on every block that nothing fails processing and that the weight stays
// under 80% of the block limit.
#[tokio::test(flavor = "multi_thread")]
async fn relay_chain_produces_blocks() {
	load(Chain::Relay).await.execute_with(|| {
		for _ in 0..10 {
			next_block_rc();
		}
	});
}

#[tokio::test(flavor = "multi_thread")]
async fn coretime_produces_blocks() {
	load(Chain::Coretime).await.execute_with(|| {
		for _ in 0..10 {
			next_block_para::<CoretimePara>();
		}
	});
}

#[tokio::test(flavor = "multi_thread")]
async fn asset_hub_produces_blocks() {
	load(Chain::AssetHub).await.execute_with(|| {
		for _ in 0..10 {
			next_block_para::<AssetHubPara>();
		}
	});
}

#[tokio::test(flavor = "multi_thread")]
async fn rc_and_asset_hub_exchange_messages() {
	message_round_trip::<AssetHubPara>().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn rc_and_coretime_exchange_messages() {
	message_round_trip::<CoretimePara>().await;
}

/// Assert that a `System::Remarked` event was emitted on runtime `T`.
fn assert_remarked<T: frame_system::Config>(chain: Chain)
where
	T::RuntimeEvent: TryInto<frame_system::Event<T>>,
{
	assert!(
		frame_system::Pallet::<T>::events().into_iter().any(|record| matches!(
			record.event.try_into(),
			Ok(frame_system::Event::<T>::Remarked { .. })
		)),
		"remark did not execute on {}",
		chain.name()
	);
}

/// Sends a `System::remark_with_event` from the RC to `P` and back, asserting on the destination
/// that the remark actually executed.
async fn message_round_trip<P: Para>()
where
	RuntimeCallFor<P>: From<frame_system::Call<P::Runtime>>,
{
	let (mut rc, mut para) = tokio::join!(load(Chain::Relay), load(P::CHAIN));

	// RC -> para.
	let dmp = rc.execute_with(|| {
		let call: RuntimeCallFor<P> = frame_system::Call::<P::Runtime>::remark_with_event {
			remark: b"minimal-relay dmp".to_vec(),
		}
		.into();
		send_dmp(P::PARA_ID.into(), unpaid_transact(call));
		next_block_rc();
		take_dmp(P::PARA_ID.into())
	});
	rc.commit_all().unwrap();
	// The live snapshot may have queued unrelated messages for this para, so only assert that
	// ours is among them.
	assert!(!dmp.is_empty(), "RC queued no DMP message for {}", P::CHAIN.name());

	para.execute_with(|| {
		enqueue_dmp::<P>(dmp);
		next_block_para::<P>();
		assert_remarked::<P::Runtime>(P::CHAIN);
	});
	para.commit_all().unwrap();

	// para -> RC.
	let ump = para.execute_with(|| {
		let call: network::relay::RuntimeCall =
			frame_system::Call::remark_with_event { remark: b"minimal-relay ump".to_vec() }.into();
		send_ump::<P>(unpaid_transact(call));
		take_ump::<P>()
	});
	para.commit_all().unwrap();
	assert!(!ump.is_empty(), "{} queued no UMP message for the RC", P::CHAIN.name());

	rc.execute_with(|| {
		enqueue_ump(P::PARA_ID.into(), ump);
		next_block_rc();
		assert_remarked::<network::relay::Runtime>(Chain::Relay);
	});
}
