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

//! Tests for the AHM v2 migration.
//!
//! Tests use the multi-thread tokio runtime because [`load`] spawns snapshot hydration onto a
//! worker; on the default single-thread runtime, `tokio::join!`-ed loads would run one after the
//! other.

use crate::mock::*;
use codec::Encode;
use frame_support::assert_ok;
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

// One block-production test per chain, so a failure names the chain that broke.
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
		let call: RuntimeCallFor<P> =
			frame_system::Call::<P::Runtime>::remark_with_event { remark: b"ahmv2 dmp".to_vec() }
				.into();
		send_dmp(P::PARA_ID.into(), unpaid_transact(call));
		next_block_rc();
		take_dmp(P::PARA_ID.into())
	});
	// The live snapshot may have queued unrelated messages for this para, so only assert that
	// ours is among them.
	assert!(!dmp.is_empty(), "RC queued no DMP message for {}", P::CHAIN.name());

	para.execute_with(|| {
		enqueue_dmp::<P>(dmp);
		next_block_para::<P>();
		assert_remarked::<P::Runtime>(P::CHAIN);
	});

	// para -> RC.
	let ump = para.execute_with(|| {
		let call: network::relay::RuntimeCall =
			frame_system::Call::remark_with_event { remark: b"ahmv2 ump".to_vec() }.into();
		send_ump::<P>(unpaid_transact(call));
		take_ump::<P>()
	});
	assert!(!ump.is_empty(), "{} queued no UMP message for the RC", P::CHAIN.name());

	rc.execute_with(|| {
		enqueue_ump(P::PARA_ID.into(), ump);
		next_block_rc();
		assert_remarked::<network::relay::Runtime>(Chain::Relay);
	});
}

/// The migration's stage machine, driven end to end over live relay-chain and Coretime state.
///
/// This is what the unit tests cannot prove: that the two chains' hand-encoded calls decode
/// against each other's real `RuntimeCall`, that the relay chain's XCM router and the Coretime
/// chain's barrier actually carry the handshake, and that each side's origin converter grants the
/// authority the receiving call checks for — `Superuser` downwards, the parachain origin upwards.
///
/// It also asserts that nothing moves: the machine has no data stages, so a full run must leave
/// both chains' issuance exactly as the snapshot had it.
#[tokio::test(flavor = "multi_thread")]
async fn the_migration_runs_to_completion_and_moves_nothing() {
	use pallet_rc2_migrator::MigrationStage as RcStage;

	let (mut rc, mut ct) = tokio::join!(load(Chain::Relay), load(CoretimePara::CHAIN));

	let rc_issuance_before =
		rc.execute_with(pallet_balances::Pallet::<network::relay::Runtime>::total_issuance);
	let ct_issuance_before =
		ct.execute_with(pallet_balances::Pallet::<network::ct::Runtime>::total_issuance);

	// The relay chain is inert until governance schedules the migration, and stays inert until
	// the start block.
	let dmp = rc.execute_with(|| {
		assert_eq!(rc_stage(), RcStage::Pending);
		next_block_rc();
		assert_eq!(rc_stage(), RcStage::Pending, "an unscheduled migration must not start");

		// Two blocks' worth of time ahead: the block whose hooks first see a clock at or past
		// `start` is the third from here, since hooks run before the timestamp inherent.
		let start = now_ms_rc() + 2 * RC_BLOCK_TIME_MS;
		assert_ok!(pallet_rc2_migrator::Pallet::<network::relay::Runtime>::schedule_migration(
			network::relay::RuntimeOrigin::root(),
			start,
		));

		next_block_rc();
		next_block_rc();
		assert_eq!(rc_stage(), RcStage::Scheduled { start }, "must not start before its time");

		next_block_rc();
		assert_eq!(rc_stage(), RcStage::WaitingForCt);
		take_dmp(CoretimePara::PARA_ID.into())
	});
	assert!(!dmp.is_empty(), "the relay chain queued no start signal for the Coretime chain");

	// The Coretime chain opens the migration and answers upwards. `enqueue_dmp` decodes the
	// message with the real Coretime `RuntimeCall`, so a stale pallet or call index fails here.
	let ump = ct.execute_with(|| {
		assert_eq!(ct_stage(), pallet_ct_migrator::MigrationStage::Pending);
		enqueue_dmp::<CoretimePara>(dmp);
		next_block_para::<CoretimePara>();

		assert_eq!(ct_stage(), pallet_ct_migrator::MigrationStage::DataMigrationOngoing);
		take_ump::<CoretimePara>()
	});
	assert!(!ump.is_empty(), "the Coretime chain queued no answer for the relay chain");

	// The answer admits the machine to the verification window and, with no data stages
	// implemented, straight on to the finish.
	let dmp = rc.execute_with(|| {
		enqueue_ump(CoretimePara::PARA_ID.into(), ump);
		next_block_rc();

		let RcStage::CoolOff { end_at } = rc_stage() else {
			panic!("readiness did not admit the machine to the cool-off: {:?}", rc_stage())
		};
		let now = frame_system::Pallet::<network::relay::Runtime>::block_number();
		assert!(end_at > now, "the verification window must not be already over");

		// The window's length is a runtime constant, not what this test is measuring.
		set_block_number_rc(end_at - 1);
		next_block_rc();
		assert_eq!(rc_stage(), RcStage::MigrationDone);
		take_dmp(CoretimePara::PARA_ID.into())
	});
	assert!(!dmp.is_empty(), "the relay chain queued no finish signal");

	ct.execute_with(|| {
		enqueue_dmp::<CoretimePara>(dmp);
		next_block_para::<CoretimePara>();
		assert_eq!(ct_stage(), pallet_ct_migrator::MigrationStage::MigrationDone);

		// Nothing was migrated, so nothing was minted here.
		assert_eq!(
			pallet_balances::Pallet::<network::ct::Runtime>::total_issuance(),
			ct_issuance_before,
			"a migration with no data stages must not change Coretime issuance"
		);
	});

	rc.execute_with(|| {
		assert_eq!(
			pallet_balances::Pallet::<network::relay::Runtime>::total_issuance(),
			rc_issuance_before,
			"a migration with no data stages must not change relay-chain issuance"
		);
	});
}

fn rc_stage() -> pallet_rc2_migrator::MigrationStageOf<network::relay::Runtime> {
	pallet_rc2_migrator::RcMigrationStage::<network::relay::Runtime>::get()
}

fn ct_stage() -> pallet_ct_migrator::MigrationStage {
	pallet_ct_migrator::CtMigrationStage::<network::ct::Runtime>::get()
}
