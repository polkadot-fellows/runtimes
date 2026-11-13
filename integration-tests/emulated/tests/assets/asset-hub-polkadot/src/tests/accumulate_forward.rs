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
use emulated_integration_tests_common::dap_helpers::test_accumulate_forward_transfers_to_asset_hub;
use polkadot_system_emulated_network::BulletinPolkadotPara as BulletinPolkadot;

/// Every Polkadot chain with `pallet-accumulate-and-forward` forwards its accumulated funds to the
/// DAP staging account on Asset Hub.
#[test]
fn accumulated_funds_are_forwarded_to_the_dap() {
	Polkadot::execute_with(|| Dmp::make_parachain_reachable(AssetHubPolkadot::para_id()));
	test_accumulate_forward_transfers_to_asset_hub::<Polkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<BridgeHubPolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<CollectivesPolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<PeoplePolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<CoretimePolkadot, AssetHubPolkadot>();
	test_accumulate_forward_transfers_to_asset_hub::<BulletinPolkadot, AssetHubPolkadot>();
}
