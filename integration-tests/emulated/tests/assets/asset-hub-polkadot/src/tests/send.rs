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
	AssetHubPolkadot::force_create_asset_from_relay_as_root(
		ASSET_ID,
		ASSET_MIN_BALANCE,
		true,
		AssetHubPolkadotSender::get(),
		None,
	)
}

pub fn penpal_register_foreign_asset_on_asset_hub(asset_location_on_penpal: Location) {
	let penpal_sovereign_account = AssetHubPolkadot::sovereign_account_id_of(
		AssetHubPolkadot::sibling_location_of(PenpalA::para_id()),
	);
	let foreign_asset_at_asset_hub = Location::new(1, [Parachain(PenpalA::para_id().into())])
		.appended_with(asset_location_on_penpal)
		.unwrap();

	// Encoded `create_asset` call to be executed in AssetHub
	let call = AssetHubPolkadot::create_foreign_asset_call(
		foreign_asset_at_asset_hub.clone(),
		ASSET_MIN_BALANCE,
		penpal_sovereign_account.clone(),
	);

	let origin_kind = OriginKind::Xcm;
	let fee_amount = ASSET_HUB_POLKADOT_ED * 1000000;
	let system_asset = (Parent, fee_amount).into();

	let root_origin = <PenpalA as Chain>::RuntimeOrigin::root();
	let system_para_destination = PenpalA::sibling_location_of(AssetHubPolkadot::para_id()).into();
	let xcm = xcm_transact_paid_execution(
		call,
		origin_kind,
		system_asset,
		penpal_sovereign_account.clone(),
	);

	// SA-of-Penpal-on-AHP needs to have balance to pay for fees and asset creation deposit
	AssetHubPolkadot::fund_accounts(vec![(
		penpal_sovereign_account.clone(),
		ASSET_HUB_POLKADOT_ED * 10000000000,
	)]);

	PenpalA::execute_with(|| {
		assert_ok!(<PenpalA as PenpalAPallet>::PolkadotXcm::send(
			root_origin,
			bx!(system_para_destination),
			bx!(xcm),
		));

		PenpalA::assert_xcm_pallet_sent();
	});

	AssetHubPolkadot::execute_with(|| {
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;
		AssetHubPolkadot::assert_xcmp_queue_success(None);
		assert_expected_events!(
			AssetHubPolkadot,
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

		type ForeignAssets = <AssetHubPolkadot as AssetHubPolkadotPallet>::ForeignAssets;
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
