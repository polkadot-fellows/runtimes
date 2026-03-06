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

mod aliases;
mod asset_transfers;
mod claim_assets;
mod register_bridged_assets;
mod send_xcm;
mod snowbridge;
mod snowbridge_common;
mod snowbridge_v2_config;
mod snowbridge_v2_inbound;
mod snowbridge_v2_inbound_to_kusama;
mod snowbridge_v2_outbound;
mod snowbridge_v2_outbound_edge_case;
mod snowbridge_v2_outbound_from_kusama;
mod snowbridge_v2_rewards;
mod teleport;

pub(crate) fn asset_hub_kusama_location() -> Location {
	Location::new(2, [GlobalConsensus(Kusama), Parachain(AssetHubKusama::para_id().into())])
}

pub(crate) fn asset_hub_polkadot_global_location() -> Location {
	Location::new(2, [GlobalConsensus(Polkadot), Parachain(AssetHubPolkadot::para_id().into())])
}

pub(crate) fn bridge_hub_kusama_location() -> Location {
	Location::new(2, [GlobalConsensus(Kusama), Parachain(BridgeHubKusama::para_id().into())])
}

// DOT and wDOT
pub(crate) fn dot_at_ah_polkadot() -> Location {
	Parent.into()
}
pub(crate) fn bridged_dot_at_ah_kusama() -> Location {
	Location::new(2, [GlobalConsensus(NetworkId::Polkadot)])
}

// wKSM
pub(crate) fn bridged_ksm_at_ah_polkadot() -> Location {
	Location::new(2, [GlobalConsensus(NetworkId::Kusama)])
}

// USDT and wUSDT
pub(crate) fn usdt_at_ah_polkadot() -> Location {
	Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(USDT_ID.into())])
}
pub(crate) fn bridged_usdt_at_ah_kusama() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(NetworkId::Polkadot),
			Parachain(AssetHubPolkadot::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(USDT_ID.into()),
		],
	)
}

// wETH has same relative location on both Kusama and Polkadot AssetHubs
pub(crate) fn weth_at_asset_hubs() -> Location {
	Location::new(
		2,
		[
			GlobalConsensus(NetworkId::Ethereum { chain_id: snowbridge::CHAIN_ID }),
			AccountKey20 { network: None, key: WETH },
		],
	)
}

pub(crate) fn create_foreign_on_ah_kusama(
	id: Location,
	sufficient: bool,
	reserves: Vec<ForeignAssetReserveData>,
) {
	let owner = AssetHubKusama::account_id_of(ALICE);
	AssetHubKusama::force_create_foreign_asset(
		id.clone(),
		owner.clone(),
		sufficient,
		ASSET_MIN_BALANCE,
		vec![],
	);
	AssetHubKusama::set_foreign_asset_reserves(id, owner, reserves);
}

pub(crate) fn create_foreign_on_ah_polkadot(
	id: Location,
	sufficient: bool,
	reserves: Vec<ForeignAssetReserveData>,
	prefund_accounts: Vec<(AccountId, u128)>,
) {
	let owner = AssetHubPolkadot::account_id_of(ALICE);
	let min = ASSET_MIN_BALANCE;
	AssetHubPolkadot::force_create_foreign_asset(
		id.clone(),
		owner.clone(),
		sufficient,
		min,
		prefund_accounts,
	);
	AssetHubPolkadot::set_foreign_asset_reserves(id, owner, reserves);
}

pub(crate) fn foreign_balance_on_ah_kusama(id: Location, who: &AccountId) -> u128 {
	AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(id, who)
	})
}
pub(crate) fn foreign_balance_on_ah_polkadot(id: Location, who: &AccountId) -> u128 {
	AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(id, who)
	})
}

pub(crate) fn send_assets_from_asset_hub_polkadot(
	destination: Location,
	assets: Assets,
	fee_idx: u32,
	// For knowing what reserve to pick.
	// We only allow using the same transfer type for assets and fees right now.
	// And only `LocalReserve` or `DestinationReserve`.
	transfer_type: TransferType,
) -> DispatchResult {
	let signed_origin =
		<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: AssetHubKusamaReceiver::get().into() }.into();

	type Runtime = <AssetHubPolkadot as Chain>::Runtime;
	let remote_fee_id: AssetId = assets
		.clone()
		.into_inner()
		.get(fee_idx as usize)
		.ok_or(pallet_xcm::Error::<Runtime>::Empty)?
		.clone()
		.id;

	AssetHubPolkadot::execute_with(|| {
		<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::transfer_assets_using_type_and_then(
			signed_origin,
			bx!(destination.into()),
			bx!(assets.into()),
			bx!(transfer_type.clone()),
			bx!(remote_fee_id.into()),
			bx!(transfer_type),
			bx!(VersionedXcm::from(
				Xcm::<()>::builder_unsafe().deposit_asset(AllCounted(1), beneficiary).build()
			)),
			WeightLimit::Unlimited,
		)
	})
}

pub(crate) fn assert_bridge_hub_polkadot_message_accepted(expected_processed: bool) {
	BridgeHubPolkadot::execute_with(|| {
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		if expected_processed {
			assert_expected_events!(
				BridgeHubPolkadot,
				vec![
					// pay for bridge fees
					RuntimeEvent::Balances(pallet_balances::Event::Burned { .. }) => {},
					// message exported
					RuntimeEvent::BridgeKusamaMessages(
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
				BridgeHubPolkadot,
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

pub(crate) fn assert_bridge_hub_kusama_message_received() {
	BridgeHubKusama::execute_with(|| {
		type RuntimeEvent = <BridgeHubKusama as Chain>::RuntimeEvent;
		assert_expected_events!(
			BridgeHubKusama,
			vec![
				// message sent to destination
				RuntimeEvent::XcmpQueue(
					cumulus_pallet_xcmp_queue::Event::XcmpMessageSent { .. }
				) => {},
			]
		);
	})
}
