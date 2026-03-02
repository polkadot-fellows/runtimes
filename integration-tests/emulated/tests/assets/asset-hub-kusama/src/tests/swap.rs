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
use kusama_system_emulated_network::penpal_emulated_chain::LocalTeleportableToAssetHub as PenpalLocalTeleportableToAssetHub;
use sp_runtime::ModuleError;
use system_parachains_constants::kusama::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT;

#[test]
fn swap_locally_on_chain_using_local_assets() {
	let asset_native = Box::new(asset_hub_kusama_runtime::xcm_config::KsmLocation::get());
	let asset_one = Box::new(Location::new(
		0,
		[PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())],
	));

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
			AssetHubKusamaSender::get()
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
				AssetHubKusamaSender::get(),
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
			1414213562273 - SYSTEM_PARA_EXISTENTIAL_DEPOSIT * 2, /* all but the 2 EDs can't be
			                                                      * retrieved. */
			0,
			0,
			AssetHubKusamaSender::get(),
		));
	});
}

#[test]
fn swap_locally_on_chain_using_foreign_assets() {
	let asset_native = Box::new(asset_hub_kusama_runtime::xcm_config::KsmLocation::get());
	let asset_location_on_penpal: Location =
		PenpalA::execute_with(PenpalLocalTeleportableToAssetHub::get);
	let foreign_asset_at_asset_hub_kusama =
		Location::new(1, [Parachain(PenpalA::para_id().into())])
			.appended_with(asset_location_on_penpal)
			.unwrap();

	let penpal_as_seen_by_ah = AssetHubKusama::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahk = AssetHubKusama::sovereign_account_id_of(penpal_as_seen_by_ah);
	AssetHubKusama::fund_accounts(vec![
		// An account to swap ksmfor something else.
		(AssetHubKusamaSender::get(), 5_000_000 * ASSET_HUB_KUSAMA_ED),
		// Penpal's sovereign account in AH should have some balance
		(sov_penpal_on_ahk.clone(), 100_000_000 * ASSET_HUB_KUSAMA_ED),
	]);

	AssetHubKusama::execute_with(|| {
		// 0: No need to create foreign asset as it exists in genesis.
		//
		// 1:: Mint foreign asset on asset_hub_kusama:
		//
		// (While it might be nice to use batch,
		// currently that's disabled due to safe call filters.)

		type RuntimeEvent = <AssetHubKusama as Chain>::RuntimeEvent;
		// 3. Mint foreign asset (in reality this should be a teleport or some such)
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::ForeignAssets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahk.clone()),
			foreign_asset_at_asset_hub_kusama.clone(),
			sov_penpal_on_ahk.clone().into(),
			ASSET_HUB_KUSAMA_ED * 3_000_000_000_000,
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
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_kusama.clone()),
		));

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		// 5. Add liquidity:
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::add_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahk.clone()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_kusama.clone()),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			sov_penpal_on_ahk.clone()
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
		let path = vec![asset_native.clone(), Box::new(foreign_asset_at_asset_hub_kusama.clone())];

		assert_ok!(
			<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::swap_exact_tokens_for_tokens(
				<AssetHubKusama as Chain>::RuntimeOrigin::signed(AssetHubKusamaSender::get()),
				path,
				100000 * ASSET_HUB_KUSAMA_ED,
				1000 * ASSET_HUB_KUSAMA_ED,
				AssetHubKusamaSender::get(),
				true
			)
		);

		assert_expected_events!(
			AssetHubKusama,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. },) => {
					amount_in: *amount_in == 333333300000,
					amount_out: *amount_out == 498874118173,
				},
			]
		);

		// 7. Remove liquidity
		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::AssetConversion::remove_liquidity(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahk.clone()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_kusama),
			1414213562273 / 2, // remove only half
			0,
			0,
			sov_penpal_on_ahk.clone(),
		));
	});
}

#[test]
fn cannot_create_pool_from_pool_assets() {
	let asset_native = asset_hub_kusama_runtime::xcm_config::KsmLocation::get();
	let asset_one = asset_hub_kusama_runtime::xcm_config::PoolAssetsPalletLocation::get()
		.appended_with(GeneralIndex(ASSET_ID.into()))
		.expect("valid location");

	AssetHubKusama::execute_with(|| {
		let pool_owner_account_id = asset_hub_kusama_runtime::AssetConversionOrigin::get();

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PoolAssets::create(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(pool_owner_account_id.clone()),
			ASSET_ID,
			pool_owner_account_id.clone().into(),
			1000,
		));
		assert!(<AssetHubKusama as AssetHubKusamaPallet>::PoolAssets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubKusama as AssetHubKusamaPallet>::PoolAssets::mint(
			<AssetHubKusama as Chain>::RuntimeOrigin::signed(pool_owner_account_id),
			ASSET_ID,
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

#[test]
#[ignore]
fn pay_xcm_fee_with_some_asset_swapped_for_native() {
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
		// send xcm transact from `penpal` account
		let call = <AssetHubKusama as Chain>::RuntimeCall::System(frame_system::Call::<
			<AssetHubKusama as Chain>::Runtime,
		>::remark {
			remark: vec![],
		})
		.encode()
		.into();

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
