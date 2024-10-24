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

use crate::tests::{
	snowbridge::{CHAIN_ID, WETH},
	*,
};

const XCM_FEE: u128 = 40_000_000_000;

/// Tests the registering of a Polkadot Asset as a bridged asset on Kusama Asset Hub.
#[test]
fn register_polkadot_asset_on_kah_from_pah() {
	// Polkadot Asset Hub asset when bridged to Kusama Asset Hub.
	let bridged_asset_at_kah = v4::Location::new(
		2,
		[
			v4::Junction::GlobalConsensus(v4::NetworkId::Polkadot),
			v4::Junction::Parachain(AssetHubPolkadot::para_id().into()),
			v4::Junction::PalletInstance(ASSETS_PALLET_ID),
			v4::Junction::GeneralIndex(ASSET_ID.into()),
		],
	);
	// Register above asset on Kusama AH from Polkadot AH.
	register_asset_on_kah_from_pah(bridged_asset_at_kah);
}

/// Tests the registering of an Ethereum Asset as a bridged asset on Kusama Asset Hub.
#[test]
fn register_ethereum_asset_on_kah_from_pah() {
	// Ethereum asset when bridged to Kusama Asset Hub.
	let bridged_asset_at_kah = v4::Location::new(
		2,
		[
			v4::Junction::GlobalConsensus(v4::NetworkId::Ethereum { chain_id: CHAIN_ID }),
			v4::Junction::AccountKey20 { network: None, key: WETH },
		],
	);
	// Register above asset on Kusama AH from Polkadot AH.
	register_asset_on_kah_from_pah(bridged_asset_at_kah);
}

fn register_asset_on_kah_from_pah(bridged_asset_at_kah: v4::Location) {
	let sa_of_pah_on_kah = AssetHubKusama::sovereign_account_of_parachain_on_other_global_consensus(
		Polkadot,
		AssetHubPolkadot::para_id(),
	);

	// Encoded `create_asset` call to be executed in Kusama Asset Hub ForeignAssets pallet.
	let call = AssetHubKusama::create_foreign_asset_call(
		bridged_asset_at_kah.clone(),
		ASSET_MIN_BALANCE,
		sa_of_pah_on_kah.clone(),
	);

	let origin_kind = OriginKind::Xcm;
	let fee_amount = XCM_FEE;
	let fees = (Parent, fee_amount).into();

	let xcm = xcm_transact_paid_execution(call, origin_kind, fees, sa_of_pah_on_kah.clone());

	// SA-of-PAH-on-KAH needs to have balance to pay for fees and asset creation deposit
	AssetHubKusama::fund_accounts(vec![(
		sa_of_pah_on_kah.clone(),
		ASSET_HUB_KUSAMA_ED * 10000000000,
	)]);

	let destination = asset_hub_kusama_location();

	// fund the PAH's SA on PBH for paying bridge transport fees
	BridgeHubPolkadot::fund_para_sovereign(AssetHubPolkadot::para_id(), 10_000_000_000_000u128);

	// set XCM versions
	AssetHubPolkadot::force_xcm_version(destination.clone(), XCM_VERSION);
	BridgeHubPolkadot::force_xcm_version(bridge_hub_kusama_location(), XCM_VERSION);

	let root_origin = <AssetHubPolkadot as Chain>::RuntimeOrigin::root();
	AssetHubPolkadot::execute_with(|| {
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PolkadotXcm::send(
			root_origin,
			bx!(destination.into()),
			bx!(xcm),
		));

		AssetHubPolkadot::assert_xcm_pallet_sent();
	});

	assert_bridge_hub_polkadot_message_accepted(true);
	assert_bridge_hub_kusama_message_received();
	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		AssetHubKusama::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// Burned the fee
				RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
					who: *who == sa_of_pah_on_kah.clone(),
					amount: *amount == fee_amount,
				},
				// Foreign Asset created
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { asset_id, creator, owner }) => {
					asset_id: *asset_id == bridged_asset_at_kah,
					creator: *creator == sa_of_pah_on_kah.clone(),
					owner: *owner == sa_of_pah_on_kah,
				},
				// Unspent fee minted to origin
				RuntimeEvent::Balances(pallet_balances::Event::Minted { who, .. }) => {
					who: *who == sa_of_pah_on_kah.clone(),
				},
			]
		);
		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		assert!(ForeignAssets::asset_exists(bridged_asset_at_kah));
	});
}
