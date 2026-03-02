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
use system_parachains_constants::polkadot::currency::SYSTEM_PARA_EXISTENTIAL_DEPOSIT;

#[test]
fn swap_locally_on_chain_using_local_assets() {
	use frame_support::traits::fungible::Mutate;

	let asset_native = asset_hub_polkadot_runtime::xcm_config::DotLocation::get();
	let asset_one =
		Location::new(0, [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())]);

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
			bx!(asset_native.clone()),
			bx!(asset_one.clone()),
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			bx!(asset_native.clone()),
			bx!(asset_one.clone()),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			AssetHubPolkadotSender::get()
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		let path = vec![bx!(asset_native.clone()), bx!(asset_one.clone())];

		assert_ok!(
            <AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::swap_exact_tokens_for_tokens(
                <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
                path,
                100,
                1,
                AssetHubPolkadotSender::get(),
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
				bx!(asset_native),
				bx!(asset_one),
				1414213562273 - SYSTEM_PARA_EXISTENTIAL_DEPOSIT * 2, /* all but the 2 EDs can't
				                                                      * be
				                                                      * retrieved. */
				0,
				0,
				AssetHubPolkadotSender::get(),
			)
		);
	});
}

#[test]
fn swap_locally_on_chain_using_foreign_assets() {
	let asset_native = Box::new(asset_hub_polkadot_runtime::xcm_config::DotLocation::get());
	let asset_location_on_penpal: Location =
		PenpalA::execute_with(PenpalLocalTeleportableToAssetHub::get);
	let foreign_asset_at_asset_hub_polkadot =
		Location::new(1, [Parachain(PenpalA::para_id().into())])
			.appended_with(asset_location_on_penpal)
			.unwrap();

	let penpal_as_seen_by_ah = AssetHubPolkadot::sibling_location_of(PenpalA::para_id());
	let sov_penpal_on_ahp = AssetHubPolkadot::sovereign_account_id_of(penpal_as_seen_by_ah);
	AssetHubPolkadot::fund_accounts(vec![
		// An account to swap dot for something else.
		(AssetHubPolkadotSender::get(), 5_000_000 * ASSET_HUB_POLKADOT_ED),
		// Penpal's sovereign account in AH should have some balance
		(sov_penpal_on_ahp.clone(), 100_000_000 * ASSET_HUB_POLKADOT_ED),
	]);

	AssetHubPolkadot::execute_with(|| {
		// 0: No need to create foreign asset as it exists in genesis.
		//
		// 1:: Mint foreign asset on asset_hub_polkadot:
		//
		// (While it might be nice to use batch,
		// currently that's disabled due to safe call filters.)

		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		// 3. Mint foreign asset (in reality this should be a teleport or some such)
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets::mint(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahp.clone()),
			foreign_asset_at_asset_hub_polkadot.clone(),
			sov_penpal_on_ahp.clone().into(),
			ASSET_HUB_POLKADOT_ED * 3_000_000_000_000,
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
			Box::new(foreign_asset_at_asset_hub_polkadot.clone()),
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		// 5. Add liquidity:
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahp.clone()),
			asset_native.clone(),
			Box::new(foreign_asset_at_asset_hub_polkadot.clone()),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			sov_penpal_on_ahp.clone()
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
		let path =
			vec![asset_native.clone(), Box::new(foreign_asset_at_asset_hub_polkadot.clone())];

		assert_ok!(
            <AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::swap_exact_tokens_for_tokens(
                <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
                path,
                100000 * ASSET_HUB_POLKADOT_ED,
                1000 * ASSET_HUB_POLKADOT_ED,
                AssetHubPolkadotSender::get(),
                true
            )
        );

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapExecuted { amount_in, amount_out, .. },) => {
					amount_in: *amount_in == 10000000000000,
					amount_out: *amount_out == 1817684594348,
				},
			]
		);

		// 7. Remove liquidity
		assert_ok!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::remove_liquidity(
				<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(sov_penpal_on_ahp.clone()),
				asset_native.clone(),
				Box::new(foreign_asset_at_asset_hub_polkadot),
				1414213562273 / 2, // remove only half
				0,
				0,
				sov_penpal_on_ahp.clone(),
			)
		);
	});
}

#[test]
fn cannot_create_pool_from_pool_assets() {
	use frame_support::traits::fungibles::{Create, Mutate};

	let asset_native = asset_hub_polkadot_runtime::xcm_config::DotLocation::get();
	let asset_one = asset_hub_polkadot_runtime::xcm_config::PoolAssetsPalletLocation::get()
		.appended_with(GeneralIndex(ASSET_ID.into()))
		.expect("valid location");

	AssetHubPolkadot::execute_with(|| {
		assert_ok!(
			<<AssetHubPolkadot as AssetHubPolkadotPallet>::PoolAssets as Create<_>>::create(
				ASSET_ID,
				AssetHubPolkadotSender::get(),
				false,
				1000,
			)
		);
		assert!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PoolAssets::asset_exists(ASSET_ID));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::PoolAssets::mint_into(
			ASSET_ID,
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

#[test]
#[ignore]
fn pay_xcm_fee_with_some_asset_swapped_for_native() {
	use frame_support::traits::fungible::Mutate;

	let asset_native: Location = asset_hub_polkadot_runtime::xcm_config::DotLocation::get();
	let asset_one = Location {
		parents: 0,
		interior: [PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())].into(),
	};
	let penpal = AssetHubPolkadot::sovereign_account_id_of(AssetHubPolkadot::sibling_location_of(
		PenpalB::para_id(),
	));

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		// set up pool with ASSET_ID <> NATIVE pair
		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::create(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			ASSET_ID.into(),
			AssetHubPolkadotSender::get().into(),
			ASSET_MIN_BALANCE,
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
			Box::new(asset_native.clone()),
			Box::new(asset_one.clone()),
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::PoolCreated { .. }) => {},
			]
		);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::AssetConversion::add_liquidity(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			Box::new(asset_native),
			Box::new(asset_one),
			1_000_000_000_000,
			2_000_000_000_000,
			0,
			0,
			AssetHubPolkadotSender::get()
		));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::LiquidityAdded {lp_token_minted, .. }) => { lp_token_minted: *lp_token_minted == 1414213562273, },
			]
		);

		// ensure `penpal` sovereign account has no native tokens and mint some `ASSET_ID`
		assert_eq!(
			<AssetHubPolkadot as AssetHubPolkadotPallet>::Balances::free_balance(penpal.clone()),
			0
		);

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::touch_other(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			ASSET_ID.into(),
			penpal.clone().into(),
		));

		assert_ok!(<AssetHubPolkadot as AssetHubPolkadotPallet>::Assets::mint(
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(AssetHubPolkadotSender::get()),
			ASSET_ID.into(),
			penpal.clone().into(),
			10_000_000_000_000,
		));
	});

	PenpalB::execute_with(|| {
		// send xcm transact from `penpal` account while paying with `ASSET_ID` tokens on
		// `AssetHubPolkadot`
		let call = <AssetHubPolkadot as Chain>::RuntimeCall::System(frame_system::Call::<
			<AssetHubPolkadot as Chain>::Runtime,
		>::remark {
			remark: vec![],
		})
		.encode()
		.into();

		let penpal_root = <PenpalB as Chain>::RuntimeOrigin::root();
		let fee_amount = 4_000_000_000_000u128;
		let asset_one =
			([PalletInstance(ASSETS_PALLET_ID), GeneralIndex(ASSET_ID.into())], fee_amount).into();
		let asset_hub_location = PenpalB::sibling_location_of(AssetHubPolkadot::para_id()).into();
		let xcm = xcm_transact_paid_execution(
			call,
			OriginKind::SovereignAccount,
			asset_one,
			penpal.clone(),
		);

		assert_ok!(<PenpalB as PenpalBPallet>::PolkadotXcm::send(
			penpal_root,
			bx!(asset_hub_location),
			bx!(xcm),
		));

		PenpalB::assert_xcm_pallet_sent();
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		AssetHubPolkadot::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::AssetConversion(pallet_asset_conversion::Event::SwapCreditExecuted { .. },) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true,.. }) => {},
			]
		);
	});
}
