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
pub use xcm::prelude::*;

// Cumulus
pub use bulletin_polkadot_runtime::ExistentialDeposit as BulletinPolkadotExistentialDeposit;
pub use emulated_integration_tests_common::xcm_emulator::{Chain, TestExt};
pub use parachains_common::{AccountId, Balance};
pub use polkadot_system_emulated_network::{
	asset_hub_polkadot_emulated_chain::genesis::ED as ASSET_HUB_POLKADOT_ED,
	bulletin_polkadot_emulated_chain::{
		self, bulletin_polkadot_runtime, genesis::ED as BULLETIN_POLKADOT_ED,
		BulletinPolkadotParaPallet as BulletinPolkadotPallet,
	},
	penpal_emulated_chain::PenpalAssetOwner,
	polkadot_emulated_chain::genesis::ED as POLKADOT_ED,
	AssetHubPolkadotPara as AssetHubPolkadot, BridgeHubPolkadotPara as BridgeHubPolkadot,
	BulletinPolkadotPara as BulletinPolkadot, CollectivesPolkadotPara as CollectivesPolkadot,
	PenpalAPara as PenpalA, PeoplePolkadotPara as PeoplePolkadot, PolkadotRelay as Polkadot,
};

#[cfg(test)]
mod tests;
