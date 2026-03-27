// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// Substrate
pub use frame_support::{assert_ok, traits::fungibles::Inspect};

// Polkadot
pub use xcm::prelude::{AccountId32 as AccountId32Junction, *};

// Cumulus
pub use bulletin_polkadot_runtime::ExistentialDeposit as BulletinPolkadotExistentialDeposit;
pub use emulated_integration_tests_common::{
	impls::Parachain,
	xcm_emulator::{Chain, TestExt},
};
pub use parachains_common::{AccountId, Balance};
pub use polkadot_system_emulated_network::{
	asset_hub_polkadot_emulated_chain::{
		genesis::ED as ASSET_HUB_POLKADOT_ED, AssetHubPolkadotParaPallet as AssetHubPolkadotPallet,
	},
	bridge_hub_polkadot_emulated_chain::BridgeHubPolkadotParaPallet as BridgeHubPolkadotPallet,
	bulletin_polkadot_emulated_chain::{
		self, bulletin_polkadot_runtime, genesis::ED as BULLETIN_POLKADOT_ED,
		BulletinPolkadotParaPallet as BulletinPolkadotPallet,
	},
	collectives_polkadot_emulated_chain::CollectivesPolkadotParaPallet as CollectivesPolkadotPallet,
	penpal_emulated_chain::{PenpalAParaPallet as PenpalAPallet, PenpalAssetOwner},
	people_polkadot_emulated_chain::PeoplePolkadotParaPallet as PeoplePolkadotPallet,
	polkadot_emulated_chain::{genesis::ED as POLKADOT_ED, PolkadotRelayPallet as PolkadotPallet},
	AssetHubPolkadotPara as AssetHubPolkadot,
	AssetHubPolkadotParaReceiver as AssetHubPolkadotReceiver,
	AssetHubPolkadotParaSender as AssetHubPolkadotSender,
	BridgeHubPolkadotPara as BridgeHubPolkadot,
	BulletinPolkadotPara as BulletinPolkadot,
	BulletinPolkadotParaReceiver as BulletinPolkadotReceiver,
	BulletinPolkadotParaSender as BulletinPolkadotSender,
	CollectivesPolkadotPara as CollectivesPolkadot, PenpalAPara as PenpalA,
	PenpalAParaReceiver as PenpalAReceiver, PenpalAParaSender as PenpalASender,
	PeoplePolkadotPara as PeoplePolkadot, PeoplePolkadotParaReceiver as PeoplePolkadotReceiver,
	PeoplePolkadotParaSender as PeoplePolkadotSender, PolkadotRelay as Polkadot,
	PolkadotRelayReceiver as PolkadotReceiver, PolkadotRelaySender as PolkadotSender,
};

#[cfg(test)]
mod tests;
