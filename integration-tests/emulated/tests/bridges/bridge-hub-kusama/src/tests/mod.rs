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

mod asset_transfers;
mod claim_assets;
mod send_xcm;
mod snowbridge;
mod teleport;

pub(crate) fn asset_hub_polkadot_location() -> Location {
	Location::new(
		2,
		[GlobalConsensus(NetworkId::Polkadot), Parachain(AssetHubPolkadot::para_id().into())],
	)
}

pub(crate) fn bridge_hub_polkadot_location() -> Location {
	Location::new(
		2,
		[GlobalConsensus(NetworkId::Polkadot), Parachain(BridgeHubPolkadot::para_id().into())],
	)
}

pub(crate) fn send_asset_from_asset_hub_kusama(
	destination: Location,
	(id, amount): (Location, u128),
) -> DispatchResult {
	let signed_origin =
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get());

	let beneficiary: Location =
		AccountId32Junction { network: None, id: AssetHubPolkadotReceiver::get().into() }.into();

	let assets: Assets = (id, amount).into();
	let fee_asset_item = 0;

	AssetHubKusama::execute_with(|| {
		<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::limited_reserve_transfer_assets(
			signed_origin,
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			fee_asset_item,
			WeightLimit::Unlimited,
		)
	})
}

pub(crate) fn assert_bridge_hub_kusama_message_accepted(expected_processed: bool) {
	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;

		if expected_processed {
			assert_expected_events!(
				BridgeHubKusama,
				vec![
					// pay for bridge fees
					RuntimeEvent::Balances(pallet_balances::Event::Burned { .. }) => {},
					// message exported
					RuntimeEvent::BridgePolkadotMessages(
						pallet_bridge_messages::Event::MessageAccepted { .. }
					) => {},
					// message processed successfully
					RuntimeEvent::MessageQueue(
						pallet_message_queue::Event::Processed { success: true, .. }
					) => {},
				]
			);
		} else {
			assert_expected_events!(
				BridgeHubKusama,
				vec![
					RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed {
						success: false,
						..
					}) => {},
				]
			);
		}
	});
}

pub(crate) fn assert_bridge_hub_polkadot_message_received() {
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				// message sent to destination
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	})
}
