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
use asset_hub_polkadot_runtime::xcm_config::XcmConfig as AssetHubPolkadotXcmConfig;
use collectives_polkadot_runtime::xcm_config::XcmConfig as CollectivesPolkadotXcmConfig;
use frame_support::assert_ok;
use integration_tests_helpers::{
	test_parachain_is_trusted_teleporter, test_relay_is_trusted_teleporter,
	test_sibling_is_trusted_teleporter,
};
use polkadot_runtime::xcm_config::XcmConfig as PolkadotXcmConfig;

#[test]
fn teleport_from_and_to_relay() {
	let amount = POLKADOT_ED * 10;
	let native_asset: Assets = (Here, amount).into();

	test_relay_is_trusted_teleporter!(
		Polkadot,                  // Origin
		PolkadotXcmConfig,         // XCM Configuration
		vec![CollectivesPolkadot], // Destinations
		(native_asset, amount)
	);

	test_parachain_is_trusted_teleporter!(
		CollectivesPolkadot,          // Origin
		CollectivesPolkadotXcmConfig, // XCM Configuration
		Polkadot,                     // Destination
		amount
	);
}

#[test]
fn teleport_from_collectives_to_asset_hub() {
	let amount = ASSET_HUB_POLKADOT_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	test_sibling_is_trusted_teleporter!(
		CollectivesPolkadot,          // Origin
		CollectivesPolkadotXcmConfig, // XCM Configuration
		vec![AssetHubPolkadot],       // Destinations
		(native_asset, amount)
	);
}

#[test]
fn teleport_from_asset_hub_to_collectives() {
	let amount = COLLECTIVES_POLKADOT_ED * 100;
	let native_asset: Assets = (Parent, amount).into();

	test_sibling_is_trusted_teleporter!(
		AssetHubPolkadot,          // Origin
		AssetHubPolkadotXcmConfig, // XCM Configuration
		vec![CollectivesPolkadot], // Destinations
		(native_asset, amount)
	);
}
