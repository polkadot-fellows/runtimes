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
use sp_runtime::ModuleError;

#[test]
fn swap_locally_on_chain_using_local_assets() {
	let asset_native = Box::new(asset_hub_kusama_runtime::xcm_config::KsmLocation::get());
	let asset_one = Box::new(MultiLocation {
		parents: 0,
		interior: X2(PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())),
	});

	AssetHubKusama::execute_with(|| {
		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::create(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			ASSET_ID.into(),
			AssetHubKusamaSender::get().into(),
			1000,
		));
		assert!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::Assets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			ASSET_ID.into(),
			AssetHubKusamaSender::get().into(),
			100_000_000_000_000,
		));

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::create_pool(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			asset_native.clone(),
			asset_one.clone(),
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::add_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			asset_native.clone(),
			asset_one.clone(),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			AssetHubKusamaSender::get().into()
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		let path = vec![asset_native.clone(), asset_one.clone()];

		assert_ok!(
			<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::swap_exact_tokens_for_tokens(
				<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
				path,
				100,
				1,
				AssetHubKusamaSender::get().into(),
				true
			)
		);

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. }) => {
					amount_in: *amount_in == 100,
					amount_out: *amount_out == 199,
				},
			]
		);

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::remove_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			asset_native,
			asset_one,
			1414213562273 - ASSET_HUB_KUSAMA_ED * 2, // all but the 2 EDs can't be retrieved.
			0,
			0,
			AssetHubKusamaSender::get().into(),
		));
	});
}

#[test]
fn swap_locally_on_chain_using_foreign_assets() {
	let asset_native = asset_hub_kusama_runtime::xcm_config::KsmLocation::get();
	let ah_as_seen_by_penpal = PenpalKusamaA::sibling_location_of(AssetHubKusama::para_id());
	let asset_location_on_penpal = PenpalLocalTeleportableToAssetHub::get();
	let asset_id_on_penpal = match asset_location_on_penpal.last() {
		Some(GeneralIndex(id)) => *id as u32,
		_ => unreachable!(),
	};
	let asset_owner_on_penpal = PenpalKusamaASender::get();
	let foreign_asset_at_asset_hub_kusama =
		MultiLocation { parents: 1, interior: X1(Parachain(PenpalKusamaA::para_id().into())) }
			.appended_with(asset_location_on_penpal)
			.unwrap();

	// 1. Create asset on penpal and, 2. Create foreign asset on asset_hub_kusama
	super::penpal_create_foreign_asset_on_asset_hub(
		asset_id_on_penpal,
		foreign_asset_at_asset_hub_kusama,
		ah_as_seen_by_penpal,
		true,
		asset_owner_on_penpal,
		ASSET_MIN_BALANCE * 1_000_000,
	);

	let penpal_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalKusamaA::para_id());
	let sov_penpal_on_ahr = AssetHubKusama::sovereign_account_id_of(penpal_as_seen_by_ah);
	AssetHubKusama::fund_accounts(vec![
		(AssetHubKusamaSender::get().into(), 5_000_000 * KUSAMA_ED), /* An account to swap dot
		                                                              * for something else. */
	]);

	AssetHubKusama::execute_with(|| {
		// 3: Mint foreign asset on asset_hub_kusama:
		//
		// (While it might be nice to use batch,
		// currently that's disabled due to safe call filters.)

		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		// 3. Mint foreign asset (in reality this should be a teleport or some such)
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone().into()),
			foreign_asset_at_asset_hub_kusama,
			sov_penpal_on_ahr.clone().into(),
			3_000_000_000_000,
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);

		// 4. Create pool:
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::create_pool(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
			Box::new(asset_native),
			Box::new(foreign_asset_at_asset_hub_kusama),
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		// 5. Add liquidity:
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::add_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone()),
			Box::new(asset_native),
			Box::new(foreign_asset_at_asset_hub_kusama),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			sov_penpal_on_ahr.clone().into()
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => {
					lp_token_minted: *lp_token_minted == 1414213562273,
				},
			]
		);

		// 6. Swap!
		let path = vec![Box::new(asset_native), Box::new(foreign_asset_at_asset_hub_kusama)];

		assert_ok!(
			<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::swap_exact_tokens_for_tokens(
				<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
				path,
				100000,
				1000,
				AssetHubKusamaSender::get().into(),
				true
			)
		);

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. },) => {
					amount_in: *amount_in == 100000,
					amount_out: *amount_out == 199399,
				},
			]
		);

		// 7. Remove liquidity
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::remove_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahr.clone()),
			Box::new(asset_native),
			Box::new(foreign_asset_at_asset_hub_kusama),
			1414213562273 - 2_000_000_000, // all but the 2 EDs can't be retrieved.
			0,
			0,
			sov_penpal_on_ahr.clone().into(),
		));
	});
}

#[test]
fn cannot_create_pool_from_pool_assets() {
	let asset_native = asset_hub_kusama_runtime::xcm_config::KsmLocation::get();
	let mut asset_one = asset_hub_kusama_runtime::xcm_config::PoolAssetsPalletLocation::get();
	asset_one.append_with(GeneralIndex(ASSET_ID.into())).expect("pool assets");

	AssetHubKusama::execute_with(|| {
		let pool_owner_account_id = asset_hub_kusama_runtime::AssetConversionOrigin::get();

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PoolAssets::create(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(pool_owner_account_id.clone()),
			ASSET_ID.into(),
			pool_owner_account_id.clone().into(),
			1000,
		));
		assert!(<AssetHubKusama as AssetHubKusamaPallet>::PoolAssets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PoolAssets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(pool_owner_account_id),
			ASSET_ID.into(),
			AssetHubKusamaSender::get().into(),
			3_000_000_000_000,
		));

		assert_matches::assert_matches!(
			<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::create_pool(
				<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
				Box::new(asset_native),
				Box::new(asset_one),
			),
			Err(DispatchError::Module(ModuleError{index: _, error: _, message})) => assert_eq!(message, Some("Unknown"))
		);
	});
}
