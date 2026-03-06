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
	dispatch::RawOrigin,
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchError, DispatchResult},
	traits::fungibles::Inspect,
	BoundedVec,
};
pub use sp_runtime::traits::Dispatchable;

// Kusama
pub use xcm::{
	prelude::{AccountId32 as AccountId32Junction, *},
	v5::{self, Error, NetworkId::Kusama as KusamaId},
};
pub use xcm_executor::traits::TransferType;

// Cumulus
pub use asset_hub_kusama_runtime::xcm_config::{KsmLocation, XcmConfig as AssetHubKusamaXcmConfig};
pub use asset_test_utils::xcm_helpers;
pub use emulated_integration_tests_common::{
	test_parachain_is_trusted_teleporter,
	xcm_emulator::{
		assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
		RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
	},
	xcm_helpers::{xcm_transact_paid_execution, xcm_transact_unpaid_execution},
	PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, RESERVABLE_ASSET_ID, XCM_V5,
};
pub use integration_tests_helpers::{
	test_parachain_is_trusted_teleporter_for_relay, test_relay_is_trusted_teleporter,
};
pub use kusama_runtime::xcm_config::UniversalLocation as KusamaUniversalLocation;
pub use kusama_system_emulated_network::{
	asset_hub_kusama_emulated_chain::{
		genesis::{AssetHubKusamaAssetOwner, ED as ASSET_HUB_KUSAMA_ED},
		AssetHubKusamaParaPallet as AssetHubKusamaPallet, ForeignAssetReserveData,
	},
	bridge_hub_kusama_emulated_chain::BridgeHubKusamaParaPallet as BridgeHubKusamaPallet,
	coretime_kusama_emulated_chain::CoretimeKusamaParaPallet as CoretimeKusamaPallet,
	kusama_emulated_chain::{genesis::ED as KUSAMA_ED, KusamaRelayPallet as KusamaPallet},
	penpal_emulated_chain::{
		penpal_runtime::xcm_config::{
			LocalReservableFromAssetHub as PenpalLocalReservableFromAssetHub,
			LocalTeleportableToAssetHub as PenpalLocalTeleportableToAssetHub,
			UniversalLocation as PenpalUniversalLocation,
			UsdtFromAssetHub as PenpalUsdtFromAssetHub,
		},
		CustomizableAssetFromSystemAssetHub, PenpalAParaPallet as PenpalAPallet, PenpalAssetOwner,
		PenpalBParaPallet as PenpalBPallet, ED as PENPAL_ED,
	},
	people_kusama_emulated_chain::PeopleKusamaParaPallet as PeopleKusamaPallet,
	AssetHubKusamaPara as AssetHubKusama, AssetHubKusamaParaReceiver as AssetHubKusamaReceiver,
	AssetHubKusamaParaSender as AssetHubKusamaSender, BridgeHubKusamaPara as BridgeHubKusama,
	BridgeHubKusamaParaReceiver as BridgeHubKusamaReceiver, CoretimeKusamaPara as CoretimeKusama,
	KusamaRelay as Kusama, KusamaRelayReceiver as KusamaReceiver,
	KusamaRelaySender as KusamaSender, PenpalAPara as PenpalA,
	PenpalAParaReceiver as PenpalAReceiver, PenpalAParaSender as PenpalASender,
	PenpalBPara as PenpalB, PenpalBParaReceiver as PenpalBReceiver,
	PenpalBParaSender as PenpalBSender, PeopleKusamaPara as PeopleKusama,
};
pub use parachains_common::{AccountId, Balance};

pub const ASSET_ID: u32 = 3;
pub const ASSET_MIN_BALANCE: u128 = 1000;
// `Assets` pallet index
pub const ASSETS_PALLET_ID: u8 = 50;

pub type RelayToParaTest = Test<Kusama, PenpalA>;
pub type ParaToRelayTest = Test<PenpalA, Kusama>;
pub type SystemParaToRelayTest = Test<AssetHubKusama, Kusama>;
pub type SystemParaToParaTest = Test<AssetHubKusama, PenpalA>;
pub type ParaToSystemParaTest = Test<PenpalA, AssetHubKusama>;
pub type ParaToParaThroughRelayTest = Test<PenpalA, PenpalB, Kusama>;
pub type ParaToParaThroughAHTest = Test<PenpalA, PenpalB, AssetHubKusama>;
pub type RelayToParaThroughAHTest = Test<Kusama, PenpalA, AssetHubKusama>;

#[cfg(test)]
mod tests;
