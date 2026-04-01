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
use frame_support::{assert_ok, sp_runtime::traits::Dispatchable, traits::fungible::Inspect};
use parachains_common::{AccountId, Balance};
use sp_runtime::traits::BlockNumberProvider;
use xcm_executor::traits::ConvertLocation;

/// Helper: wrap `inner_call` in a `pallet_scheduler::schedule` call targeting the next block.
/// Must be called inside a `CollectivesPolkadot::execute_with` closure.
fn schedule_on_next_block(
	inner_call: <CollectivesPolkadot as Chain>::RuntimeCall,
) -> <CollectivesPolkadot as Chain>::RuntimeCall {
	type Runtime = <CollectivesPolkadot as Chain>::Runtime;
	type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
	let current_block =
		<Runtime as pallet_scheduler::Config>::BlockNumberProvider::current_block_number();
	RuntimeCall::Scheduler(pallet_scheduler::Call::<Runtime>::schedule {
		when: current_block + 1,
		maybe_periodic: None,
		priority: 0,
		call: Box::new(inner_call),
	})
}

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
/// The scheduled XCM aliases into the Fellowship Treasury on Asset Hub, withdraws DOT from the
/// treasury's sovereign account, and deposits it to a beneficiary — the same pattern as the
/// direct-send test in `aliases.rs`, but routed through the scheduler.
///
/// The scheduler stores the caller's origin and dispatches with it later, so the scheduled
/// call executes with the Architects origin — not Root. This means Architects can only
/// schedule calls they are already authorized to make (e.g., `pallet_xcm::send`).
#[test]
fn architects_can_schedule_xcm_send() {
	let collectives_para_id: u32 = CollectivesPolkadot::para_id().into();
	let amount: Balance = ASSET_HUB_POLKADOT_ED * 100;

	// Fellowship Treasury pallet location on Collectives, as seen from AH.
	let pallet_location = Location::new(
		1,
		[
			Parachain(collectives_para_id),
			PalletInstance(
				collectives_polkadot_runtime_constants::FELLOWSHIP_TREASURY_PALLET_INDEX,
			),
		],
	);

	// Compute the sovereign account for this pallet location on AH.
	let pallet_sovereign =
		asset_hub_polkadot_runtime::xcm_config::LocationToAccountId::convert_location(
			&pallet_location,
		)
		.expect("Failed to convert pallet location to account");

	let beneficiary: AccountId = [42u8; 32].into();

	// Fund the pallet's sovereign account on AH.
	AssetHubPolkadot::fund_accounts(vec![(pallet_sovereign.clone(), amount * 2)]);

	// Record pre-balances on AH.
	let (pre_sovereign_balance, pre_beneficiary_balance) = AssetHubPolkadot::execute_with(|| {
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;
		(
			<Balances as Inspect<_>>::balance(&pallet_sovereign),
			<Balances as Inspect<_>>::balance(&beneficiary),
		)
	});

	// Step 1: Schedule the XCM send call for the next block.
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;

		let xcm_send_call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(
				1,
				[Parachain(AssetHubPolkadot::para_id().into())],
			))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				AliasOrigin(pallet_location.clone()),
				WithdrawAsset((Parent, amount).into()),
				DepositAsset {
					assets: Wild(All),
					beneficiary: Location::new(
						0,
						[AccountId32 { network: None, id: beneficiary.clone().into() }],
					),
				},
			]))),
		});

		// Schedule for the next block.
		let schedule_call = schedule_on_next_block(xcm_send_call);

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

	// Step 3: Verify balance changes on Asset Hub — funds moved from sovereign to beneficiary.
	AssetHubPolkadot::execute_with(|| {
		type Balances = <AssetHubPolkadot as AssetHubPolkadotPallet>::Balances;

		let post_sovereign_balance = <Balances as Inspect<_>>::balance(&pallet_sovereign);
		let post_beneficiary_balance = <Balances as Inspect<_>>::balance(&beneficiary);

		assert!(
			post_sovereign_balance < pre_sovereign_balance,
			"Sovereign account balance should have decreased: pre={pre_sovereign_balance}, post={post_sovereign_balance}",
		);
		assert!(
			post_beneficiary_balance > pre_beneficiary_balance,
			"Beneficiary balance should have increased: pre={pre_beneficiary_balance}, post={post_beneficiary_balance}",
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

		let schedule_call = schedule_on_next_block(root_only_call);

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

		let schedule_call = schedule_on_next_block(noop_call);

		let signed_origin: RuntimeOrigin = RuntimeOrigin::signed([1u8; 32].into());
		assert_err!(schedule_call.dispatch(signed_origin), frame_support::error::BadOrigin,);
	});
}
