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

use crate::*;
use codec::Encode;
use collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::{
	Architects as ArchitectsOrigin, Fellows as FellowsOrigin,
};
use frame_support::{assert_ok, sp_runtime::traits::Dispatchable, traits::BlockNumberProvider};

#[test]
fn fellows_whitelist_call() {
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;
		type PolkadotCall = <Polkadot as Chain>::RuntimeCall;
		type PolkadotRuntime = <Polkadot as Chain>::Runtime;

		let call_hash = [1u8; 32].into();

		let whitelist_call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::parent())),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: PolkadotCall::Whitelist(
						pallet_whitelist::Call::<PolkadotRuntime>::whitelist_call { call_hash }
					)
					.encode()
					.into(),
				}
			]))),
		});

		let fellows_origin: RuntimeOrigin = FellowsOrigin.into();

		assert_ok!(whitelist_call.dispatch(fellows_origin));

		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	Polkadot::execute_with(|| {
		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			Polkadot,
			vec![
				RuntimeEvent::Whitelist(pallet_whitelist::Event::CallWhitelisted { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

/// Verify that the Architects origin can schedule an XCM send via `pallet_scheduler`,
/// and that the scheduled call is dispatched, the message is sent, and it reaches Asset Hub.
///
/// The scheduler stores the caller's origin and dispatches with it later, so the scheduled
/// call executes with the Architects origin — not Root. This means Architects can only
/// schedule calls they are already authorized to make (e.g., `pallet_xcm::send`).
#[test]
fn architects_can_schedule_xcm_send() {
	// Step 1: Schedule the XCM send call for the next block.
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;
		type AssetHubCall = <AssetHubPolkadot as Chain>::RuntimeCall;
		type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;

		// Build an XCM send to Asset Hub with a remark_with_event transact.
		// The Architects origin is recognized by Asset Hub's FellowshipEntities barrier,
		// so unpaid execution is allowed.
		let xcm_send_call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(
				1,
				[Parachain(AssetHubPolkadot::para_id().into())],
			))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: AssetHubCall::System(
						frame_system::Call::<AssetHubRuntime>::remark_with_event {
							remark: b"architects scheduled xcm".to_vec(),
						},
					)
					.encode()
					.into(),
				},
			]))),
		});

		// Schedule for the next block.
		let current_block =
			<Runtime as pallet_scheduler::Config>::BlockNumberProvider::current_block_number();
		let schedule_call = RuntimeCall::Scheduler(pallet_scheduler::Call::<Runtime>::schedule {
			when: current_block + 1,
			maybe_periodic: None,
			priority: 0,
			call: Box::new(xcm_send_call),
		});

		let architects_origin: RuntimeOrigin = ArchitectsOrigin.into();
		assert_ok!(schedule_call.dispatch(architects_origin));

		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::Scheduler(pallet_scheduler::Event::Scheduled { .. }) => {},
			]
		);
	});

	// Step 2: Advance one block on Collectives — the scheduler fires in `on_initialize`,
	// dispatches the scheduled call, and the XCM message is sent.
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::Scheduler(pallet_scheduler::Event::Dispatched { .. }) => {},
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// Step 3: Verify the message was processed successfully on Asset Hub.
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::System(frame_system::Event::Remarked { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

/// Verify that when Architects schedule a Root-only call (like `set_code`), the scheduler
/// accepts the scheduling (Architects have `ScheduleOrigin`) but the dispatch fails because
/// the stored origin is the Architects origin, not Root.
#[test]
fn architects_scheduled_root_call_fails_on_dispatch() {
	// Step 1: Schedule a Root-only call using the Architects origin.
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;

		// `set_code` requires Root origin.
		let root_only_call =
			RuntimeCall::System(frame_system::Call::<Runtime>::set_code { code: vec![] });

		let current_block =
			<Runtime as pallet_scheduler::Config>::BlockNumberProvider::current_block_number();
		let schedule_call = RuntimeCall::Scheduler(pallet_scheduler::Call::<Runtime>::schedule {
			when: current_block + 1,
			maybe_periodic: None,
			priority: 0,
			call: Box::new(root_only_call),
		});

		// Scheduling succeeds — Architects have ScheduleOrigin.
		let architects_origin: RuntimeOrigin = ArchitectsOrigin.into();
		assert_ok!(schedule_call.dispatch(architects_origin));

		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::Scheduler(pallet_scheduler::Event::Scheduled { .. }) => {},
			]
		);
	});

	// Step 2: Advance one block — the scheduler dispatches the call with the Architects
	// origin, which fails because `set_code` requires Root.
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::Scheduler(pallet_scheduler::Event::Dispatched {
					result: Err(_),
					..
				}) => {},
			]
		);
	});
}

/// Verify that a regular signed account cannot schedule calls (only Root and Architects can).
#[test]
fn signed_account_cannot_schedule() {
	CollectivesPolkadot::execute_with(|| {
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;

		let noop_call =
			RuntimeCall::System(frame_system::Call::<Runtime>::remark { remark: vec![] });

		let current_block =
			<Runtime as pallet_scheduler::Config>::BlockNumberProvider::current_block_number();
		let schedule_call = RuntimeCall::Scheduler(pallet_scheduler::Call::<Runtime>::schedule {
			when: current_block + 1,
			maybe_periodic: None,
			priority: 0,
			call: Box::new(noop_call),
		});

		let signed_origin: RuntimeOrigin = RuntimeOrigin::signed([1u8; 32].into());
		assert_err!(schedule_call.dispatch(signed_origin), frame_support::error::BadOrigin,);
	});
}
