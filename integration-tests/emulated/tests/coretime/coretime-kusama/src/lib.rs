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
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchError, DispatchResult},
	traits::fungibles::Inspect,
};

// Polkadot
pub use xcm::{
	prelude::{AccountId32 as AccountId32Junction, *},
	v3::{Error, NetworkId::Kusama as KusamaId},
};

// Cumulus
pub use asset_test_utils::xcm_helpers;
pub use emulated_integration_tests_common::{
	xcm_emulator::{
		assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
		RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
	},
	xcm_helpers::{xcm_transact_paid_execution, xcm_transact_unpaid_execution},
	PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, XCM_V3,
};
pub use kusama_system_emulated_network::{
	asset_hub_kusama_emulated_chain::{
		genesis::ED as ASSET_HUB_KUSAMA_ED, AssetHubKusamaParaPallet as AssetHubKusamaPallet,
	},
	coretime_kusama_emulated_chain::{
		genesis::ED as CORETIME_KUSAMA_ED, CoretimeKusamaParaPallet as CoretimeKusamaPallet,
	},
	kusama_emulated_chain::{genesis::ED as KUSAMA_ED, KusamaRelayPallet as KusamaPallet},
	AssetHubKusamaPara as AssetHubKusama, AssetHubKusamaParaReceiver as AssetHubKusamaReceiver,
	AssetHubKusamaParaSender as AssetHubKusamaSender, CoretimeKusamaPara as CoretimeKusama,
	CoretimeKusamaParaReceiver as CoretimeKusamaReceiver,
	CoretimeKusamaParaSender as CoretimeKusamaSender, KusamaRelay as Kusama,
	KusamaRelayReceiver as KusamaReceiver, KusamaRelaySender as KusamaSender,
	PenpalAPara as PenpalA,
};
pub use parachains_common::{AccountId, Balance};

#[cfg(test)]
mod tests;
