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

//! Tests for the validation of `pallet_xcm::Pallet::<T>::transfer_assets`.
//! See the `pallet_xcm::transfer_assets_validation` module for more information.

use crate::*;
use emulated_integration_tests_common::USDT_ID;
use frame_support::{assert_err, assert_ok};
use sp_runtime::DispatchError;

// ==================================================================================
// ============================== PenpalA <-> Kusama ===============================
// ==================================================================================

/// Test that `transfer_assets` fails when doing reserve transfer of KSM from PenpalA to Kusama.
/// This fails because PenpalA's IsReserve config considers Kusama as the reserve for KSM,
/// so transfer_assets automatically chooses reserve transfer, which we block.
#[test]
fn transfer_assets_ksm_reserve_transfer_para_to_relay_fails() {
	let destination = PenpalA::parent_location();
	let beneficiary: Location =
		AccountId32Junction { network: None, id: KusamaReceiver::get().into() }.into();
	let amount_to_send: Balance = KUSAMA_ED * 1000;
	let assets: Assets = (Parent, amount_to_send).into();

	// Mint KSM on PenpalA for testing.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		KsmLocation::get(),
		PenpalASender::get(),
		amount_to_send * 2,
	);

	// Fund PenpalA's sovereign account on Kusama with the reserve KSM.
	let penpal_location_as_seen_by_relay = Kusama::child_location_of(PenpalA::para_id());
	let sov_penpal_on_relay = Kusama::sovereign_account_id_of(penpal_location_as_seen_by_relay);
	Kusama::fund_accounts(vec![(sov_penpal_on_relay, amount_to_send * 2)]);

	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			0,
			WeightLimit::Unlimited,
		);

		// This should fail because KSM reserve transfer is blocked.
		assert_err!(
			result,
			DispatchError::Module(sp_runtime::ModuleError {
				index: 31,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve.
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

/// Test that `transfer_assets` fails when doing reserve transfer of KSM from Kusama to PenpalA
/// This fails because Kusama's configuration would make this a reserve transfer, which we block.
#[test]
fn transfer_assets_ksm_reserve_transfer_relay_to_para_fails() {
	let destination = Kusama::child_location_of(PenpalA::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalAReceiver::get().into() }.into();
	let amount_to_send: Balance = KUSAMA_ED * 1000;
	let assets: Assets = (Here, amount_to_send).into();

	Kusama::execute_with(|| {
		let result = <Kusama as KusamaPallet>::XcmPallet::transfer_assets(
			<Kusama as Chain>::RuntimeOrigin::signed(KusamaSender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			0,
			WeightLimit::Unlimited,
		);

		// This should fail because KSM reserve transfer is blocked.
		assert_err!(
			result,
			DispatchError::Module(sp_runtime::ModuleError {
				index: 99,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve.
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

// ==================================================================================
// ============================== PenpalA <-> PenpalB ===============================
// ==================================================================================

/// Test that `transfer_assets` fails when doing reserve transfer of KSM from PenpalA to PenpalB
#[test]
fn transfer_assets_ksm_reserve_transfer_para_to_para_fails() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalBReceiver::get().into() }.into();
	let amount_to_send: Balance = KUSAMA_ED * 1000;
	let assets: Assets = (Parent, amount_to_send).into();

	// Mint KSM on PenpalA for testing
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		KsmLocation::get(),
		PenpalASender::get(),
		amount_to_send * 2,
	);

	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			0,
			WeightLimit::Unlimited,
		);

		// This should fail because KSM reserve transfer is blocked
		assert_err!(
			result,
			DispatchError::Module(sp_runtime::ModuleError {
				index: 31,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

// ==================================================================================
// ============================== Mixed Assets and Fees =============================
// ==================================================================================

/// Test that `transfer_assets` fails when KSM is used as fee asset in reserve transfer
#[test]
fn transfer_assets_ksm_as_fee_in_reserve_transfer_fails() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalBReceiver::get().into() }.into();
	let asset_amount: Balance = 1_000_000_000_000; // A million USDT.
	let fee_amount: Balance = KUSAMA_ED * 100;

	// Create a foreign asset location (representing another asset).
	let foreign_asset_location = Location::new(
		1,
		[
			Parachain(AssetHubKusama::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(USDT_ID.into()), // USDT.
		],
	);

	// Mint both assets on PenpalA for testing.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		foreign_asset_location.clone(),
		PenpalASender::get(),
		asset_amount * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		KsmLocation::get(),
		PenpalASender::get(),
		fee_amount * 2,
	);

	// Transfer foreign asset, pay fees with KSM.
	let assets: Assets = vec![
		(foreign_asset_location, asset_amount).into(),
		(Parent, fee_amount).into(), // KSM as fee.
	]
	.into();
	let fee_asset_item = 1; // KSM is the fee asset.

	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			fee_asset_item,
			WeightLimit::Unlimited,
		);

		// This should fail because KSM fee would be reserve transferred.
		assert_err!(
			result,
			DispatchError::Module(sp_runtime::ModuleError {
				index: 31,
				error: [21, 0, 0, 0], // InvalidAssetUnknownReserve.
				message: Some("InvalidAssetUnknownReserve")
			})
		);
	});
}

/// Test that `transfer_assets` works when neither asset nor fee is KSM.
#[test]
fn transfer_assets_non_native_assets_work() {
	let destination = PenpalA::sibling_location_of(PenpalB::para_id());
	let beneficiary: Location =
		AccountId32Junction { network: None, id: PenpalBReceiver::get().into() }.into();
	let amount: Balance = 1_000_000_000_000; // A million USDT.

	// Create foreign asset locations (both non-native).
	let asset_location = Location::new(
		1,
		[
			Parachain(AssetHubKusama::para_id().into()),
			PalletInstance(ASSETS_PALLET_ID),
			GeneralIndex(USDT_ID.into()), // USDT.
		],
	);

	// Mint both USDT and KSM on PenpalA, one for sending, the other for paying delivery fees.
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		KsmLocation::get(),
		PenpalASender::get(),
		amount * 2,
	);
	PenpalA::mint_foreign_asset(
		<PenpalA as Chain>::RuntimeOrigin::signed(PenpalAssetOwner::get()),
		asset_location.clone(),
		PenpalASender::get(),
		amount * 2,
	);

	// Transfer non-native assets.
	let assets: Assets = (asset_location, amount).into();
	let fee_asset_item = 0;

	PenpalA::execute_with(|| {
		let result = <PenpalA as PenpalAPallet>::PolkadotXcm::transfer_assets(
			<PenpalA as Chain>::RuntimeOrigin::signed(PenpalASender::get()),
			bx!(destination.into()),
			bx!(beneficiary.into()),
			bx!(assets.into()),
			fee_asset_item,
			WeightLimit::Unlimited,
		);

		// This should succeed because neither asset is KSM.
		assert_ok!(result);
	});
}
