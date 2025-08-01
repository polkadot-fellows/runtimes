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

pub use frame_support::{assert_err, assert_ok};

// Polkadot
pub use xcm::{prelude::*, v3};

// Cumulus
pub use emulated_integration_tests_common::{
	accounts::ALICE,
	test_parachain_is_trusted_teleporter,
	xcm_emulator::{assert_expected_events, bx, Chain, Parachain, RelayChain as Relay, TestExt},
};
pub use polkadot_system_emulated_network::{
	asset_hub_polkadot_emulated_chain::{
		genesis::ED as ASSET_HUB_POLKADOT_ED, AssetHubPolkadotParaPallet as AssetHubPolkadotPallet,
	},
	bridge_hub_polkadot_emulated_chain::BridgeHubPolkadotParaPallet as BridgeHubPolkadotPallet,
	collectives_polkadot_emulated_chain::{
		genesis::ED as COLLECTIVES_POLKADOT_ED,
		CollectivesPolkadotParaPallet as CollectivesPolkadotPallet,
	},
	coretime_polkadot_emulated_chain::{
		coretime_polkadot_runtime,
		coretime_polkadot_runtime::ExistentialDeposit as CoretimePolkadotExistentialDeposit,
		CoretimePolkadotParaPallet as CoretimePolkadotPallet,
	},
	penpal_emulated_chain::{PenpalAParaPallet as PenpalAPallet, PenpalAssetOwner},
	people_polkadot_emulated_chain::PeoplePolkadotParaPallet as PeoplePolkadotPallet,
	polkadot_emulated_chain::{genesis::ED as POLKADOT_ED, PolkadotRelayPallet as PolkadotPallet},
	AssetHubPolkadotPara as AssetHubPolkadot,
	AssetHubPolkadotParaReceiver as AssetHubPolkadotReceiver,
	AssetHubPolkadotParaSender as AssetHubPolkadotSender,
	BridgeHubPolkadotPara as BridgeHubPolkadot, CollectivesPolkadotPara as CollectivesPolkadot,
	CollectivesPolkadotParaReceiver as CollectivesPolkadotReceiver,
	CollectivesPolkadotParaSender as CollectivesPolkadotSender,
	CoretimePolkadotPara as CoretimePolkadot, PenpalAPara as PenpalA,
	PeoplePolkadotPara as PeoplePolkadot, PolkadotRelay as Polkadot,
	PolkadotRelayReceiver as PolkadotReceiver, PolkadotRelaySender as PolkadotSender,
};

#[cfg(test)]
mod tests;
