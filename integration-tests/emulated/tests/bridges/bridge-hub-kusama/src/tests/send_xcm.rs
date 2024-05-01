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

use crate::tests::*;

#[test]
fn send_xcm_from_kusama_relay_to_polkadot_asset_hub_should_fail_on_not_applicable() {
	// Init tests variables
	// XcmPallet send arguments
	let sudo_origin = <Kusama as Chain>::RuntimeOrigin::root();
	let destination = Kusama::child_location_of(BridgeHubKusama::para_id()).into();
	let weight_limit = WeightLimit::Unlimited;
	let check_origin = None;

	let remote_xcm = Xcm(vec![ClearOrigin]);

	let xcm = VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit, check_origin },
		ExportMessage {
			network: PolkadotId.into(),
			destination: [Parachain(AssetHubPolkadot::para_id().into())].into(),
			xcm: remote_xcm,
		},
	]));

	// Kusama Global Consensus
	// Send XCM message from Relay Chain to Bridge Hub source Parachain
	Kusama::execute_with(|| {
		assert_ok!(<Kusama as KusamaPallet>::XcmPallet::send(
			sudo_origin,
			bx!(destination),
			bx!(xcm),
		));

		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});
	// Receive XCM message in Bridge Hub source Parachain, it should fail, because we don't have
	// opened bridge/lane.
	assert_bridge_hub_kusama_message_accepted(false);
}

#[test]
fn send_xcm_through_opened_lane_with_different_xcm_version_on_hops_works() {
	// Initially set only default version on all runtimes
	AssetHubKusama::force_default_xcm_version(Some(xcm::v2::prelude::XCM_VERSION));
	BridgeHubKusama::force_default_xcm_version(Some(xcm::v2::prelude::XCM_VERSION));
	BridgeHubPolkadot::force_default_xcm_version(Some(xcm::v2::prelude::XCM_VERSION));
	AssetHubPolkadot::force_default_xcm_version(Some(xcm::v2::prelude::XCM_VERSION));

	// prepare data
	let destination = asset_hub_polkadot_location();
	let native_token = Location::parent();
	let amount = ASSET_HUB_KUSAMA_ED * 1_000;

	// fund the AHK's SA on BHK for paying bridge transport fees
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 10_000_000_000_000u128);
	// fund sender
	AssetHubKusama::fund_accounts(vec![(AssetHubKusamaSender::get(), amount * 10)]);

	// send XCM from AssetHubKusama - fails - destination version not known
	assert_err!(
		send_asset_from_asset_hub_kusama(destination.clone(), (native_token.clone(), amount)),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 31,
			error: [1, 0, 0, 0],
			message: Some("SendFailure")
		})
	);

	// set destination version
	AssetHubKusama::force_xcm_version(destination.clone(), xcm::v3::prelude::XCM_VERSION);

	// TODO: remove this block, when removing `xcm:v2`
	{
		// send XCM from AssetHubKusama - fails - AssetHubKusama is set to the default/safe `2`
		// version, which does not have the `ExportMessage` instruction. If the default `2` is
		// changed to `3`, then this assert can go away"
		assert_err!(
			send_asset_from_asset_hub_kusama(destination.clone(), (native_token.clone(), amount)),
			DispatchError::Module(sp_runtime::ModuleError {
				index: 31,
				error: [1, 0, 0, 0],
				message: Some("SendFailure")
			})
		);

		// set exact version for BridgeHubPolkadot to `2` without `ExportMessage` instruction
		AssetHubKusama::force_xcm_version(
			ParentThen(Parachain(BridgeHubKusama::para_id().into()).into()).into(),
			xcm::v2::prelude::XCM_VERSION,
		);
		// send XCM from AssetHubKusama - fails - `ExportMessage` is not in `2`
		assert_err!(
			send_asset_from_asset_hub_kusama(destination.clone(), (native_token.clone(), amount)),
			DispatchError::Module(sp_runtime::ModuleError {
				index: 31,
				error: [1, 0, 0, 0],
				message: Some("SendFailure")
			})
		);
	}

	// set version with `ExportMessage` for BridgeHubKusama
	AssetHubKusama::force_xcm_version(
		ParentThen(Parachain(BridgeHubKusama::para_id().into()).into()).into(),
		xcm::v3::prelude::XCM_VERSION,
	);
	// send XCM from AssetHubKusama - ok
	assert_ok!(send_asset_from_asset_hub_kusama(
		destination.clone(),
		(native_token.clone(), amount)
	));

	// `ExportMessage` on local BridgeHub - fails - remote BridgeHub version not known
	assert_bridge_hub_kusama_message_accepted(false);

	// set version for remote BridgeHub on BridgeHubKusama
	BridgeHubKusama::force_xcm_version(
		bridge_hub_polkadot_location(),
		xcm::v3::prelude::XCM_VERSION,
	);
	// set version for AssetHubPolkadot on BridgeHubPolkadot
	BridgeHubPolkadot::force_xcm_version(
		ParentThen(Parachain(AssetHubPolkadot::para_id().into()).into()).into(),
		xcm::v3::prelude::XCM_VERSION,
	);

	// send XCM from AssetHubKusama - ok
	assert_ok!(send_asset_from_asset_hub_kusama(
		destination.clone(),
		(native_token.clone(), amount)
	));
	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();
	// message delivered and processed at destination
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// message processed with failure, but for this scenario it is ok, important is that was delivered
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: false, .. }
				) => {},
			]
		);
	});

	// TODO: remove this block, when removing `xcm:v2`
	{
		// set `2` version for remote BridgeHub on BridgeHubKusama, which does not have
		// `UniversalOrigin` and `DescendOrigin`
		BridgeHubKusama::force_xcm_version(
			bridge_hub_polkadot_location(),
			xcm::v2::prelude::XCM_VERSION,
		);

		// send XCM from AssetHubKusama - ok
		assert_ok!(send_asset_from_asset_hub_kusama(destination, (native_token, amount)));
		// message is not accepted on the local BridgeHub (`DestinationUnsupported`) because we
		// cannot add `UniversalOrigin` and `DescendOrigin`
		assert_bridge_hub_kusama_message_accepted(false);
	}
}
