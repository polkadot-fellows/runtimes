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

pub use codec::Encode;

// Substrate
pub use frame_support::{
	assert_err, assert_ok,
	instances::Instance2,
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchError, DispatchResult, ModuleError},
	traits::fungibles::Inspect,
	BoundedVec,
};

// Polkadot
pub use xcm::{
	prelude::{AccountId32 as AccountId32Junction, *},
	v3::{self, Error, NetworkId::Polkadot as PolkadotId},
};
pub use xcm_executor::traits::TransferType;

// Cumulus
pub use asset_test_utils::xcm_helpers;
pub use emulated_integration_tests_common::{
	test_parachain_is_trusted_teleporter,
	xcm_emulator::{
		assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
		RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
	},
	xcm_helpers::{xcm_transact_paid_execution, xcm_transact_unpaid_execution},
	PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, XCM_V3,
};
pub use parachains_common::{AccountId, Balance};
pub use polkadot_runtime::xcm_config::UniversalLocation as PolkadotUniversalLocation;
pub use polkadot_system_emulated_network::{
	asset_hub_polkadot_emulated_chain::{
		genesis::{AssetHubPolkadotAssetOwner, ED as ASSET_HUB_POLKADOT_ED},
		AssetHubPolkadotParaPallet as AssetHubPolkadotPallet,
	},
	collectives_polkadot_emulated_chain::{
		genesis::ED as COLLECTIVES_POLKADOT_ED,
		CollectivesPolkadotParaPallet as CollectivesPolkadotPallet,
	},
	penpal_emulated_chain::{
		CustomizableAssetFromSystemAssetHub, PenpalAParaPallet as PenpalAPallet, PenpalAssetOwner,
		PenpalBParaPallet as PenpalBPallet, ED as PENPAL_ED,
	},
	polkadot_emulated_chain::{genesis::ED as POLKADOT_ED, PolkadotRelayPallet as PolkadotPallet},
	AssetHubPolkadotPara as AssetHubPolkadot,
	AssetHubPolkadotParaReceiver as AssetHubPolkadotReceiver,
	AssetHubPolkadotParaSender as AssetHubPolkadotSender,
	BridgeHubPolkadotPara as BridgeHubPolkadot,
	BridgeHubPolkadotParaReceiver as BridgeHubPolkadotReceiver,
	CollectivesPolkadotPara as CollectivesPolkadot, PenpalAPara as PenpalA,
	PenpalAParaReceiver as PenpalAReceiver, PenpalAParaSender as PenpalASender,
	PenpalBPara as PenpalB, PenpalBParaReceiver as PenpalBReceiver,
	PenpalBParaSender as PenpalBSender, PolkadotRelay as Polkadot,
	PolkadotRelayReceiver as PolkadotReceiver, PolkadotRelaySender as PolkadotSender,
};

pub const ASSET_ID: u32 = 3;
pub const ASSET_MIN_BALANCE: u128 = 1000;
// `Assets` pallet index
pub const ASSETS_PALLET_ID: u8 = 50;

pub type RelayToSystemParaTest = Test<Polkadot, AssetHubPolkadot>;
pub type RelayToParaTest = Test<Polkadot, PenpalB>;
pub type SystemParaToRelayTest = Test<AssetHubPolkadot, Polkadot>;
pub type SystemParaToParaTest = Test<AssetHubPolkadot, PenpalB>;
pub type ParaToSystemParaTest = Test<PenpalB, AssetHubPolkadot>;
pub type ParaToParaThroughRelayTest = Test<PenpalB, PenpalA, Polkadot>;
pub type ParaToParaThroughAHTest = Test<PenpalB, PenpalA, AssetHubPolkadot>;
pub type RelayToParaThroughAHTest = Test<Polkadot, PenpalB, AssetHubPolkadot>;

#[cfg(test)]
mod tests;
