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

/// Relay Chain should be able to execute `Transact` instructions in System Parachain
/// when `OriginKind::Superuser`.
#[test]
#[ignore]
fn send_transact_as_superuser_from_relay_to_asset_hub_works() {
	AssetHubKusama::force_create_asset_from_relay_as_root(
		ASSET_ID,
		ASSET_MIN_BALANCE,
		true,
		AssetHubKusamaSender::get(),
		None,
	)
}

pub fn penpal_register_foreign_asset_on_asset_hub(asset_location_on_penpal: Location) {
	let penpal_sovereign_account = AssetHubKusama::sovereign_account_id_of(
		AssetHubKusama::sibling_location_of(PenpalA::para_id()),
	);
	let foreign_asset_at_asset_hub = Location::new(1, [Parachain(PenpalA::para_id().into())])
		.appended_with(asset_location_on_penpal)
		.unwrap();

	// Encoded `create_asset` call to be executed in AssetHub
	let call = AssetHubKusama::create_foreign_asset_call(
		foreign_asset_at_asset_hub.clone(),
		ASSET_MIN_BALANCE,
		penpal_sovereign_account.clone(),
	);

	let origin_kind = OriginKind::Xcm;
	let fee_amount = ASSET_HUB_KUSAMA_ED * 1000000;
	let system_asset = (Parent, fee_amount).into();

	let root_origin = <PenpalA as Chain>::RuntimeOrigin::root();
	let system_para_destination = PenpalA::sibling_location_of(AssetHubKusama::para_id()).into();
	let xcm = xcm_transact_paid_execution(
		call,
		origin_kind,
		system_asset,
		penpal_sovereign_account.clone(),
	);

	// SA-of-Penpal-on-AHK needs to have balance to pay for fees and asset creation deposit
	AssetHubKusama::fund_accounts(vec![(
		penpal_sovereign_account.clone(),
		ASSET_HUB_KUSAMA_ED * 10000000000,
	)]);

	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as PenpalAPallet>::PolkadotXcm::send(
			root_origin,
			bx!(system_para_destination),
			bx!(xcm),
		));

		PenpalA::assert_xcm_pallet_sent();
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		AssetHubKusama::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubKusama,
			vec![
				// Burned the fee
				RuntimeEvent::Balances(pallet_balances::Event::Burned { who, amount }) => {
					who: *who == penpal_sovereign_account,
					amount: *amount == fee_amount,
				},
				// Foreign Asset created
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Created { asset_id, creator, owner }) => {
					asset_id: *asset_id == foreign_asset_at_asset_hub.clone(),
					creator: *creator == penpal_sovereign_account.clone(),
					owner: *owner == penpal_sovereign_account,
				},
			]
		);

		type ForeignAssets = <AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets;
		assert!(ForeignAssets::asset_exists(foreign_asset_at_asset_hub));
	});
}

/// We test two things here:
/// - Parachain should be able to send XCM paying its fee at Asset Hub using system asset
/// - Parachain should be able to create a new Foreign Asset at Asset Hub
#[test]
fn send_xcm_from_para_to_asset_hub_paying_fee_with_system_asset() {
	let asset_location_on_penpal = Location::new(
		0,
		[Junction::PalletInstance(ASSETS_PALLET_ID), Junction::GeneralIndex(ASSET_ID.into())],
	);
	penpal_register_foreign_asset_on_asset_hub(asset_location_on_penpal);
}

/// We test two things here:
/// - Parachain should be able to send XCM paying its fee at Asset Hub using a pool
/// - Parachain should be able to create a new Asset at Asset Hub
#[test]
#[ignore]
fn send_xcm_from_para_to_asset_hub_paying_fee_from_pool() {
	let asset_native: Location = asset_hub_kusama_runtime::xcm_config::KsmLocation::get();
	let asset_one = Location {
		parents: 0,
		interior: [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())].into(),
	};
	let penpal = AssetHubKusama::sovereign_account_id_of(AssetHubKusama::sibling_location_of(
		PenpalA::para_id(),
	));

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		// set up pool with ASSET_ID <> NATIVE pair
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::create(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			ASSET_ID.into(),
			AssetHubKusamaSender::get().into(),
			ASSET_MIN_BALANCE,
		));
		assert!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			ASSET_ID.into(),
			AssetHubKusamaSender::get().into(),
			3_000_000_000_000,
		));

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::create_pool(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			Box::new(asset_native.clone()),
			Box::new(asset_one.clone()),
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::add_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			Box::new(asset_native),
			Box::new(asset_one),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			AssetHubKusamaSender::get()
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		// ensure `penpal` sovereign account has no native tokens and mint some `ASSET_ID`
		assert_eq!(
			<AssetHubKusama as AssetHubKusamaPallet>::Balances::free_balance(penpal.clone()),
			0
		);

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::touch_other(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			ASSET_ID.into(),
			penpal.clone().into(),
		));

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			ASSET_ID.into(),
			penpal.clone().into(),
			10_000_000_000_000,
		));
	});

	PenpalA::execute_with(|| {
		// send xcm transact from `penpal` account which has only `ASSET_ID` tokens on
		// `AssetHubKusama`
		let call = AssetHubKusama::force_create_asset_call(
			ASSET_ID + 1000,
			penpal.clone(),
			true,
			ASSET_MIN_BALANCE,
		);

		let penpal_root = <PenpalA as Chain>::RuntimeOrigin::root();
		let fee_amount = 4_000_000_000_000u128;
		let asset_one =
			([PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())], fee_amount).into();
		let asset_hub_location = PenpalA::sibling_location_of(AssetHubKusama::para_id()).into();
		let xcm = xcm_transact_paid_execution(
			call,
			OriginKind::SovereignAccount,
			asset_one,
			penpal.clone(),
		);

		assert_ok!(<PenpalA as PenpalAPallet>::PolkadotXcm::send(
			penpal_root,
			bx!(asset_hub_location),
			bx!(xcm),
		));

		PenpalA::assert_xcm_pallet_sent();
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		AssetHubKusama::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapCreditExecuted { .. },) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true,.. }) => {},
			]
		);
	});
}
