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

// Substrate
pub use codec::Encode;
pub use frame_support::{assert_err, assert_ok, pallet_prelude::DispatchResult};
pub use sp_runtime::{traits::Dispatchable, DispatchError};

// Polkadot
pub use xcm::{
	latest::ParentThen,
	prelude::{AccountId32 as AccountId32Junction, *},
	v5::{
		self, Error,
		NetworkId::{Kusama as KusamaId, Polkadot as PolkadotId},
	},
};
pub use xcm_executor::traits::TransferType;

// Bridges
pub use bp_messages::LegacyLaneId;

// Cumulus
pub use emulated_integration_tests_common::{
	accounts::{ALICE, BOB},
	impls::Inspect,
	test_parachain_is_trusted_teleporter,
	xcm_emulator::{
		assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
		RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
	},
	xcm_helpers::{xcm_transact_paid_execution, xcm_transact_unpaid_execution},
	ASSETS_PALLET_ID, PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, XCM_V4,
};
pub use kusama_polkadot_system_emulated_network::{
	asset_hub_kusama_emulated_chain::{
		genesis::ED as ASSET_HUB_KUSAMA_ED, AssetHubKusamaParaPallet as AssetHubKusamaPallet,
	},
	asset_hub_polkadot_emulated_chain::{
		genesis::{AssetHubPolkadotAssetOwner, ED as ASSET_HUB_POLKADOT_ED},
		AssetHubPolkadotParaPallet as AssetHubPolkadotPallet,
	},
	bridge_hub_kusama_emulated_chain::{
		genesis::ED as BRIDGE_HUB_KUSAMA_ED, BridgeHubKusamaParaPallet as BridgeHubKusamaPallet,
	},
	kusama_emulated_chain::{
		genesis::ED as KUSAMA_ED,
		kusama_runtime::{xcm_config::UniversalLocation as KusamaRelayUniversalLocation, Dmp},
		KusamaRelayPallet as KusamaPallet,
	},
	penpal_emulated_chain::{
		penpal_runtime::xcm_config::{
			CustomizableAssetFromSystemAssetHub as PenpalCustomizableAssetFromSystemAssetHub,
			UniversalLocation as PenpalUniversalLocation,
		},
		PenpalAParaPallet as PenpalAPallet, PenpalAssetOwner,
	},
	AssetHubKusamaPara as AssetHubKusama, AssetHubKusamaParaReceiver as AssetHubKusamaReceiver,
	AssetHubKusamaParaSender as AssetHubKusamaSender, AssetHubPolkadotPara as AssetHubPolkadot,
	AssetHubPolkadotParaReceiver as AssetHubPolkadotReceiver,
	AssetHubPolkadotParaSender as AssetHubPolkadotSender, BridgeHubKusamaPara as BridgeHubKusama,
	BridgeHubKusamaParaReceiver as BridgeHubKusamaReceiver,
	BridgeHubKusamaParaSender as BridgeHubKusamaSender, BridgeHubPolkadotPara as BridgeHubPolkadot,
	KusamaRelay as Kusama, KusamaRelayReceiver as KusamaReceiver,
	KusamaRelaySender as KusamaSender, PenpalAPara as PenpalA,
	PenpalAParaReceiver as PenpalAReceiver, PenpalAParaSender as PenpalASender,
};
pub use parachains_common::{AccountId, Balance};

pub const ASSET_ID: u32 = 1;
pub const ASSET_MIN_BALANCE: u128 = 1000;
pub const USDT_ID: u32 = 1984;

#[cfg(test)]
mod tests;
