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
	v3::{Error, NetworkId::Polkadot as PolkadotId},
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
pub use parachains_common::{AccountId, Balance};
pub use polkadot_system_emulated_network::{
	people_polkadot_emulated_chain::{
		genesis::ED as PEOPLE_KUSAMA_ED, PeoplePolkadotParaPallet as PeoplePolkadotPallet,
	},
	polkadot_emulated_chain::{genesis::ED as KUSAMA_ED, PolkadotRelayPallet as PolkadotPallet},
	PenpalAPara as PenpalA, PeoplePolkadotPara as PeoplePolkadot,
	PeoplePolkadotParaReceiver as PeoplePolkadotReceiver,
	PeoplePolkadotParaSender as PeoplePolkadotSender, PolkadotRelay as Polkadot,
	PolkadotRelayReceiver as PolkadotReceiver, PolkadotRelaySender as PolkadotSender,
};

pub type RelayToSystemParaTest = Test<Polkadot, PeoplePolkadot>;
pub type RelayToParaTest = Test<Polkadot, PenpalA>;
pub type SystemParaToRelayTest = Test<PeoplePolkadot, Polkadot>;
pub type SystemParaToParaTest = Test<PeoplePolkadot, PenpalA>;
pub type ParaToSystemParaTest = Test<PenpalA, PeoplePolkadot>;

#[cfg(test)]
mod tests;
