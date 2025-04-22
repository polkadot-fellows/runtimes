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
mod register_bridged_assets;
mod send_xcm;
mod teleport;

mod snowbridge {
	pub const CHAIN_ID: u64 = 1;
	pub const WETH: [u8; 20] = hex_literal::hex!("87d1f7fdfEe7f651FaBc8bFCB6E086C278b77A7d");
}

pub(crate) fn asset_hub_polkadot_location() -> Location {
	Location::new(2, [GlobalConsensus(Polkadot), Parachain(AssetHubPolkadot::para_id().into())])
}

pub(crate) fn bridge_hub_polkadot_location() -> Location {
	Location::new(2, [GlobalConsensus(Polkadot), Parachain(BridgeHubPolkadot::para_id().into())])
}

// KSM and wKSM
pub(crate) fn ksm_at_ah_kusama() -> xcm::v4::Location {
	xcm::v4::Parent.into()
}
pub(crate) fn bridged_ksm_at_ah_polkadot() -> xcm::v4::Location {
	xcm::v4::Location::new(2, [xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Kusama)])
}

// wDOT
pub(crate) fn bridged_dot_at_ah_kusama() -> xcm::v4::Location {
	xcm::v4::Location::new(2, [xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Polkadot)])
}

// USDT and wUSDT
pub(crate) fn usdt_at_ah_polkadot() -> xcm::v4::Location {
	xcm::v4::Location::new(
		0,
		[
			xcm::v4::Junction::PalletInstance(ASSETS_PALLET_ID),
			xcm::v4::Junction::GeneralIndex(USDT_ID.into()),
		],
	)
}
pub(crate) fn bridged_usdt_at_ah_kusama() -> xcm::v4::Location {
	xcm::v4::Location::new(
		2,
		[
			xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Polkadot),
			xcm::v4::Junction::Parachain(AssetHubPolkadot::para_id().into()),
			xcm::v4::Junction::PalletInstance(ASSETS_PALLET_ID),
			xcm::v4::Junction::GeneralIndex(USDT_ID.into()),
		],
	)
}

// wETH has same relative location on both Kusama and Polkadot AssetHubs
pub(crate) fn weth_at_asset_hubs() -> xcm::v4::Location {
	xcm::v4::Location::new(
		2,
		[
			xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Ethereum {
				chain_id: snowbridge::CHAIN_ID,
			}),
			xcm::v4::Junction::AccountKey20 { network: None, key: snowbridge::WETH },
		],
	)
}

pub(crate) fn create_foreign_on_ah_kusama(
	id: xcm::v4::Location,
	sufficient: bool,
	prefund_accounts: Vec<(AccountId, u128)>,
) {
	let owner = AssetHubKusama::account_id_of(ALICE);
	let min = ASSET_MIN_BALANCE;
	AssetHubKusama::force_create_foreign_asset(id, owner, sufficient, min, prefund_accounts);
}

pub(crate) fn create_foreign_on_ah_polkadot(id: xcm::v4::Location, sufficient: bool) {
	let owner = AssetHubPolkadot::account_id_of(ALICE);
	AssetHubPolkadot::force_create_foreign_asset(id, owner, sufficient, ASSET_MIN_BALANCE, vec![]);
}

pub(crate) fn foreign_balance_on_ah_kusama(id: xcm::v4::Location, who: &AccountId) -> u128 {
	AssetHubKusama::execute_with(|| {
		type Assets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(id, who)
	})
}
pub(crate) fn foreign_balance_on_ah_polkadot(id: xcm::v4::Location, who: &AccountId) -> u128 {
	AssetHubPolkadot::execute_with(|| {
		type Assets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		<Assets as Inspect<_>>::balance(id, who)
	})
}

// set up pool
pub(crate) fn set_up_pool_with_dot_on_ah_polkadot(asset: xcm::v4::Location, is_foreign: bool) {
	let dot: xcm::v4::Location = xcm::v4::Parent.into();
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		let owner = AssetHubPolkadotSender::get();
		let signed_owner = <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(owner.clone());

		if is_foreign {
			assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint(
				signed_owner.clone(),
				asset.clone(),
				owner.clone().into(),
				3_000_000_000_000,
			));
		} else {
			let asset_id = match asset.interior.last() {
				Some(xcm::v4::Junction::GeneralIndex(id)) => *id as u32,
				_ => unreachable!(),
			};
			assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::mint(
				signed_owner.clone(),
				asset_id.into(),
				owner.clone().into(),
				3_000_000_000_000,
			));
		}
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::create_pool(
			signed_owner.clone(),
			Box::new(dot.clone()),
			Box::new(asset.clone()),
		));
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			signed_owner.clone(),
			Box::new(dot),
			Box::new(asset),
			1_000_000_000_000,
			2_000_000_000_000,
			1,
			1,
			owner,
		));
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {..}) => {},
			]
		);
	});
}

pub(crate) fn send_assets_from_asset_hub_kusama(
	destination: Location,
	assets: Assets,
	fee_idx: u32,
) -> DispatchResult {
	let signed_origin =
		<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: AssetHubPolkadotReceiver::get().into() }.into();

	AssetHubKusama::execute_with(|| {
		<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::limited_reserve_transfer_assets(
			signed_origin,
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			fee_idx,
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
