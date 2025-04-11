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

const XCM_FEE: u128 = 40_000_000_000;

/// Tests the registering of a Kusama Asset as a bridged asset on Polkadot Asset Hub.
#[test]
fn register_kusama_asset_on_pah_from_kah() {
	let sa_of_kah_on_pah =
		AssetHubPolkadot::sovereign_account_of_parachain_on_other_global_consensus(
			Kusama,
			AssetHubKusama::para_id(),
		);

	// Kusama Asset Hub asset when bridged to Polkadot Asset Hub.
	let bridged_asset_at_pah = xcm::v4::Location::new(
		2,
		[
			xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Kusama),
			xcm::v4::Junction::Parachain(AssetHubKusama::para_id().into()),
			xcm::v4::Junction::PalletInstance(ASSETS_PALLET_ID),
			xcm::v4::Junction::GeneralIndex(ASSET_ID.into()),
		],
	);

	// Encoded `create_asset` call to be executed in Polkadot Asset Hub ForeignAssets pallet.
	let call = AssetHubPolkadot::create_foreign_asset_call(
		bridged_asset_at_pah.clone(),
		ASSET_MIN_BALANCE,
		sa_of_kah_on_pah.clone(),
	);

	let origin_kind = OriginKind::Xcm;
	let fee_amount = XCM_FEE;
	let fees = (Parent, fee_amount).into();

	let xcm = xcm_transact_paid_execution(call, origin_kind, fees, sa_of_kah_on_pah.clone());

	// SA-of-KAH-on-PAH needs to have balance to pay for fees and asset creation deposit
	AssetHubPolkadot::fund_accounts(vec![(
		sa_of_kah_on_pah.clone(),
		ASSET_HUB_POLKADOT_ED * 10000000000,
	)]);

	let destination = asset_hub_polkadot_location();

	// fund the KAH's SA on KBH for paying bridge transport fees
	BridgeHubKusama::fund_para_sovereign(AssetHubKusama::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubKusama::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubKusama::force_xcm_version(bridge_hub_polkadot_location(), XCM_VERSION);

	let root_origin = <AssetHubKusama as Chain>::RuntimeOrigin::root();
	AssetHubKusama::execute_with(|| {
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PolkadotXcm::send(
			root_origin,
			bx!(destination.into()),
			bx!(xcm),
		));

		AssetHubKusama::assert_xcm_pallet_sent();
	});

	assert_bridge_hub_kusama_message_accepted(true);
	assert_bridge_hub_polkadot_message_received();
	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		AssetHubPolkadot::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				// Burned the fee
				RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
					who: *who == sa_of_kah_on_pah.clone(),
					amount: *amount == fee_amount,
				},
				// Foreign Asset created
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { asset_id, creator, owner }) => {
					asset_id: *asset_id == bridged_asset_at_pah,
					creator: *creator == sa_of_kah_on_pah.clone(),
					owner: *owner == sa_of_kah_on_pah,
				},
				// Unspent fee minted to origin
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == sa_of_kah_on_pah.clone(),
				},
			]
		);
		type ForeignAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
		assert!(ForeignAssets::asset_exists(bridged_asset_at_pah));
	});
}
