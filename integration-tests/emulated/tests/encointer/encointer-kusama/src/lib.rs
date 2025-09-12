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

#[cfg(test)]
mod tests;

// Substrate
pub use emulated_integration_tests_common::{
	xcm_emulator::{
		assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
		RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
	},
	PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, RESERVABLE_ASSET_ID, XCM_V5,
};
pub use frame_support::{
	assert_err, assert_ok,
	dispatch::RawOrigin,
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchError, DispatchResult},
	traits::fungibles::Inspect,
};
pub use integration_tests_helpers::{
	test_parachain_is_trusted_teleporter_for_relay, test_relay_is_trusted_teleporter,
};
pub use kusama_system_emulated_network::{
	asset_hub_kusama_emulated_chain::{
		genesis::{AssetHubKusamaAssetOwner, ED as ASSET_HUB_KUSAMA_ED},
		AssetHubKusamaParaPallet as AssetHubKusamaPallet,
	},
	encointer_kusama_emulated_chain::{
		genesis::ED as ENCOINTER_KUSAMA_ED, EncointerKusamaParaPallet as EncointerKusamaPallet,
	},
	kusama_emulated_chain::{genesis::ED as KUSAMA_ED, KusamaRelayPallet as KusamaPallet},
	penpal_emulated_chain::{
		CustomizableAssetFromSystemAssetHub, PenpalAParaPallet as PenpalAPallet, PenpalAssetOwner,
		PenpalBParaPallet as PenpalBPallet, ED as PENPAL_ED,
	},
	AssetHubKusamaPara as AssetHubKusama, AssetHubKusamaParaReceiver as AssetHubKusamaReceiver,
	AssetHubKusamaParaSender as AssetHubKusamaSender, BridgeHubKusamaPara as BridgeHubKusama,
	BridgeHubKusamaParaReceiver as BridgeHubKusamaReceiver, EncointerKusamaPara as EncointerKusama,
	EncointerKusamaParaReceiver as EncointerKusamaReceiver,
	EncointerKusamaParaSender as EncointerKusamaSender, KusamaRelay as Kusama,
	KusamaRelayReceiver as KusamaReceiver, KusamaRelaySender as KusamaSender,
	PenpalAPara as PenpalA, PenpalAParaReceiver as PenpalAReceiver,
	PenpalAParaSender as PenpalASender, PenpalBPara as PenpalB,
	PenpalBParaReceiver as PenpalBReceiver,
};
pub use parachains_common::{AccountId, Balance};
pub use sp_runtime::traits::Dispatchable;
pub use xcm::{
	prelude::{AccountId32 as AccountId32Junction, *},
	v5::{self, Error, NetworkId::Kusama as KusamaId},
};

pub use asset_test_utils::xcm_helpers;

pub type EncointerParaToRelayTest = Test<EncointerKusama, Kusama>;
pub type ParaToSystemParaTest = Test<EncointerKusama, AssetHubKusama>;
