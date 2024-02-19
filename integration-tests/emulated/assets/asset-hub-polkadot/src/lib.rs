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

// Disable tests when `runtime-benchmarks` is enabled, because of `type MessageProcessor =
// pallet_message_queue::mock_helpers::NoopMessageProcessor`.
#![cfg(not(feature = "runtime-benchmarks"))]

pub use asset_test_utils::xcm_helpers;
pub use codec::Encode;
pub use emulated_chains::{
	asset_hub_polkadot::ED as ASSET_HUB_POLKADOT_ED, polkadot::ED as POLKADOT_ED,
	AssetHubPolkadotPara as AssetHubPolkadot, AssetHubPolkadotParaPallet as AssetHubPolkadotPallet,
	AssetHubPolkadotParaReceiver as AssetHubPolkadotReceiver,
	AssetHubPolkadotParaSender as AssetHubPolkadotSender,
	BridgeHubPolkadotPara as BridgeHubPolkadot,
	BridgeHubPolkadotParaReceiver as BridgeHubPolkadotReceiver, PenpalLocalTeleportableToAssetHub,
	PenpalPolkadotAPara as PenpalPolkadotA, PenpalPolkadotAParaPallet as PenpalPolkadotAPallet,
	PenpalPolkadotAParaReceiver as PenpalPolkadotAReceiver,
	PenpalPolkadotAParaSender as PenpalPolkadotASender, PenpalPolkadotBPara as PenpalPolkadotB,
	PenpalPolkadotBParaPallet as PenpalPolkadotBPallet, PenpalXcmConfig, PolkadotRelay as Polkadot,
	PolkadotRelayPallet as PolkadotPallet, PolkadotRelayReceiver as PolkadotReceiver,
	PolkadotRelaySender as PolkadotSender,
};
pub use frame_support::{
	assert_err, assert_ok,
	pallet_prelude::Weight,
	sp_runtime::{AccountId32, DispatchError, DispatchResult},
	traits::fungibles::Inspect,
};
pub use integration_tests_common::{
	test_parachain_is_trusted_teleporter,
	xcm_helpers::{non_fee_asset, xcm_transact_paid_execution, xcm_transact_unpaid_execution},
	PROOF_SIZE_THRESHOLD, REF_TIME_THRESHOLD, XCM_V3,
};
pub use parachains_common::{AccountId, Balance};
pub use xcm::{
	prelude::{AccountId32 as AccountId32Junction, *},
	v3::{Error, NetworkId::Polkadot as PolkadotId},
};
pub use xcm_emulator::{
	assert_expected_events, bx, helpers::weight_within_threshold, Chain, Parachain as Para,
	RelayChain as Relay, Test, TestArgs, TestContext, TestExt,
};

pub const ASSET_ID: u32 = 1;
pub const ASSET_MIN_BALANCE: u128 = 1000;
// `Assets` pallet index
pub const ASSETS_PALLET_ID: u8 = 50;

pub type RelayToSystemParaTest = Test<Polkadot, AssetHubPolkadot>;
pub type RelayToParaTest = Test<Polkadot, PenpalPolkadotA>;
pub type SystemParaToRelayTest = Test<AssetHubPolkadot, Polkadot>;
pub type SystemParaToParaTest = Test<AssetHubPolkadot, PenpalPolkadotA>;
pub type ParaToSystemParaTest = Test<PenpalPolkadotA, AssetHubPolkadot>;

/// Returns a `TestArgs` instance to be used for the Relay Chain across integration tests
pub fn relay_test_args(dest: Location, beneficiary_id: AccountId32, amount: Balance) -> TestArgs {
	TestArgs {
		dest,
		beneficiary: AccountId32Junction { network: None, id: beneficiary_id.into() }.into(),
		amount,
		assets: (Here, amount).into(),
		asset_id: None,
		fee_asset_item: 0,
		weight_limit: WeightLimit::Unlimited,
	}
}

/// Returns a `TestArgs` instance to be used by parachains across integration tests
pub fn para_test_args(
	dest: Location,
	beneficiary_id: AccountId32,
	amount: Balance,
	assets: Assets,
	asset_id: Option<u32>,
	fee_asset_item: u32,
) -> TestArgs {
	TestArgs {
		dest,
		beneficiary: AccountId32Junction { network: None, id: beneficiary_id.into() }.into(),
		amount,
		assets,
		asset_id,
		fee_asset_item,
		weight_limit: WeightLimit::Unlimited,
	}
}

#[cfg(test)]
#[cfg(not(feature = "runtime-benchmarks"))]
mod tests;
