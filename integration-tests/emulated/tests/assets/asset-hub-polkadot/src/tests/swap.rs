// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use polkadot_system_emulated_network::penpal_emulated_chain::LocalTeleportableToAssetHub as PenpalLocalTeleportableToAssetHub;
use sp_runtime::ModuleError;
use system_parachains_constants::polkadot::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT;

#[test]
fn swap_locally_on_chain_using_local_assets() {
	use frame_support::traits::fungible::Mutate;

	let asset_native = Box::new(asset_hub_polkadot_runtime::xcm_config::DotLocationV3::get());
	let asset_one = Box::new(v3::Location::new(
		0,
		[
			v3::Junction::PalletInstance(ASSETS_PALLET_ID),
			v3::Junction::GeneralIndex(ASSET_ID.into()),
		],
	));

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::create(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			ASSET_ID.into(),
			AssetHubPolkadotSender::get().into(),
			1000,
		));
		assert!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::mint(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			ASSET_ID.into(),
			AssetHubPolkadotSender::get().into(),
			3_000_000_000_000,
		));

		<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::set_balance(
			&AssetHubPolkadotSender::get(),
			3_000_000_000_000,
		);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::create_pool(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			asset_native.clone(),
			asset_one.clone(),
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			asset_native.clone(),
			asset_one.clone(),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			AssetHubPolkadotSender::get().into()
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		let path = vec![asset_native.clone(), asset_one.clone()];

		assert_ok!(
            <AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::swap_exact_tokens_for_tokens(
                <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
                path,
                100,
                1,
                AssetHubPolkadotSender::get().into(),
                true
            )
        );

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. }) => {
					amount_in: *amount_in == 100,
					amount_out: *amount_out == 199,
				},
			]
		);

		assert_ok!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::remove_liquidity(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
				asset_native,
				asset_one,
				1414213562273 - SYSTEM_PARA_EXISTENTIAL_DEPOSIT * 2, /* all but the 2 EDs can't be
				                                                      * retrieved. */
				0,
				0,
				AssetHubPolkadotSender::get().into(),
			)
		);
	});
}

#[test]
fn swap_locally_on_chain_using_foreign_assets() {
	let asset_native = Box::new(
		v3::Location::try_from(asset_hub_polkadot_runtime::xcm_config::DotLocation::get())
			.expect("conversion works"),
	);

	let ah_as_seen_by_penpal = PenpalB::sibling_location_of(AssetHubPolkadot::para_id());
	let asset_location_on_penpal =
		v3::Location::try_from(PenpalLocalTeleportableToAssetHub::get()).expect("conversion works");
	let asset_id_on_penpal = match asset_location_on_penpal.last() {
		Some(v3::Junction::GeneralIndex(id)) => *id as u32,
		_ => unreachable!(),
	};
	let asset_owner_on_penpal = PenpalBSender::get();
	let foreign_asset_at_asset_hub_polkadot =
		v3::Location::new(1, [v3::Junction::Parachain(PenpalB::para_id().into())])
			.appended_with(asset_location_on_penpal)
			.unwrap();

	// 1. Create asset on penpal and, 2. Create foreign asset on asset_hub_polkadot
	super::penpal_create_foreign_asset_on_asset_hub(
		asset_id_on_penpal,
		foreign_asset_at_asset_hub_polkadot,
		ah_as_seen_by_penpal,
		true,
		asset_owner_on_penpal,
		ASSET_MIN_BALANCE * 1_000_000,
	);

	let penpal_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalB::para_id());
	let sov_penpal_on_ahk = AssetHubPolkadot::sovereign_account_id_of(penpal_as_seen_by_ah);
	AssetHubPolkadot::fund_accounts(vec![
		(AssetHubPolkadotSender::get().into(), 5_000_000 * POLKADOT_ED), /* An account to swap dot
		                                                                  * for something else. */
	]);

	AssetHubPolkadot::execute_with(|| {
		// 3: Mint foreign asset on asset_hub_polkadot:
		//
		// (While it might be nice to use batch,
		// currently that's disabled due to safe call filters.)

		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		// 3. Mint foreign asset (in reality this should be a teleport or some such)
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahk.clone().into()),
			foreign_asset_at_asset_hub_polkadot,
			sov_penpal_on_ahk.clone().into(),
			3_000_000_000_000,
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::ForeignAssets(pallet_assets::Event::Issued { .. }) => {},
			]
		);

		// 4. Create pool:
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::create_pool(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_polkadot),
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		// 5. Add liquidity:
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahk.clone()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_polkadot),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			sov_penpal_on_ahk.clone().into()
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => {
					lp_token_minted: *lp_token_minted == 1414213562273,
				},
			]
		);

		// 6. Swap!
		let path = vec![asset_native.clone(), Box::new(foreign_asset_at_asset_hub_polkadot)];

		assert_ok!(
            <AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::swap_exact_tokens_for_tokens(
                <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
                path,
                100000,
                1000,
                AssetHubPolkadotSender::get().into(),
                true
            )
        );

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. },) => {
					amount_in: *amount_in == 100000,
					amount_out: *amount_out == 199399,
				},
			]
		);

		// 7. Remove liquidity
		assert_ok!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::remove_liquidity(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahk.clone()),
				asset_native.clone(),
				Box::new(foreign_asset_at_asset_hub_polkadot),
				1414213562273 - 2_000_000_000, // all but the 2 EDs can't be retrieved.
				0,
				0,
				sov_penpal_on_ahk.clone().into(),
			)
		);
	});
}

#[test]
fn cannot_create_pool_from_pool_assets() {
	use frame_support::traits::fungibles::Create;
	use frame_support::traits::fungibles::Mutate;

	let asset_native = asset_hub_polkadot_runtime::xcm_config::DotLocation::get()
		.try_into()
		.expect("conversion works");
	let asset_one = asset_hub_polkadot_runtime::xcm_config::PoolAssetsPalletLocation::get()
		.appended_with(GeneralIndex(ASSET_ID.into()))
		.expect("valid location")
		.try_into()
		.expect("conversion works");

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(
			<<AssetHubPolkadot as AssetHubPolkadotPallet>::PoolAssets as Create<_>>::create(
				ASSET_ID.into(),
				AssetHubPolkadotSender::get(),
				false,
				1000,
			)
		);
		assert!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PoolAssets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PoolAssets::mint_into(
			ASSET_ID.into(),
			&AssetHubPolkadotSender::get(),
			3_000_000_000_000,
		));

		assert_matches::assert_matches!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::create_pool(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
				Box::new(asset_native),
				Box::new(asset_one),
			),
			Err(DispatchError::Module(ModuleError{index: _, error: _, message})) => assert_eq!(message, Some("Unknown"))
		);
	});
}
