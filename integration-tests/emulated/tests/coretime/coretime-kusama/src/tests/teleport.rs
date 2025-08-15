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
use frame_support::{
	dispatch::{GetDispatchInfo, RawOrigin},
	sp_runtime::traits::Dispatchable,
	traits::fungible::Mutate,
};
use integration_tests_helpers::{
	test_parachain_is_trusted_teleporter, test_parachain_is_trusted_teleporter_for_relay,
	test_relay_is_trusted_teleporter,
};
use xcm_runtime_apis::{
	dry_run::runtime_decl_for_dry_run_api::DryRunApiV2,
	fees::runtime_decl_for_xcm_payment_api::XcmPaymentApiV1,
};

#[test]
fn teleport_via_transfer_assets_from_and_to_relay() {
	let amount = CORETIME_KUSAMA_ED * 1000;
	let native_asset: Assets = (Here, amount).into();

	test_relay_is_trusted_teleporter!(
		Kusama,
		vec![CoretimeKusama],
		(native_asset, amount),
		transfer_assets
	);

	let amount = KUSAMA_ED * 1000;

	test_parachain_is_trusted_teleporter_for_relay!(
		CoretimeKusama,
		Kusama,
		amount,
		transfer_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_from_and_to_relay() {
	let amount = CORETIME_KUSAMA_ED * 1000;
	let native_asset: Assets = (Here, amount).into();

	test_relay_is_trusted_teleporter!(
		Kusama,
		vec![CoretimeKusama],
		(native_asset, amount),
		limited_teleport_assets
	);

	let amount = KUSAMA_ED * 1000;

	test_parachain_is_trusted_teleporter_for_relay!(
		CoretimeKusama,
		Kusama,
		amount,
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_limited_teleport_assets_from_and_to_other_system_parachains_works() {
	let amount = ASSET_HUB_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		CoretimeKusama,
		vec![AssetHubKusama],
		(native_asset, amount),
		limited_teleport_assets
	);

	let amount = CORETIME_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		AssetHubKusama,
		vec![CoretimeKusama],
		(native_asset, amount),
		limited_teleport_assets
	);
}

#[test]
fn teleport_via_transfer_assets_from_and_to_other_system_parachains_works() {
	let amount = ASSET_HUB_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		CoretimeKusama,
		vec![AssetHubKusama],
		(native_asset, amount),
		transfer_assets
	);

	let amount = CORETIME_KUSAMA_ED * 1000;
	let native_asset: Assets = (Parent, amount).into();

	test_parachain_is_trusted_teleporter!(
		AssetHubKusama,
		vec![CoretimeKusama],
		(native_asset, amount),
		transfer_assets
	);
}
