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

#[cfg(test)]
mod imports {
	pub(crate) use codec::Encode;
	pub(crate) use emulated_integration_tests_common::{
		assert_whitelisted,
		impls::{assert_expected_events, bx, Parachain, RelayChain, TestExt},
		xcm_emulator::Chain,
		xcm_helpers::{
			build_xcm_send_authorize_upgrade_call, call_hash_of,
			dispatch_whitelisted_call_with_preimage,
		},
	};
	pub(crate) use frame_support::{assert_err, assert_ok};
	pub(crate) use polkadot_runtime::{governance::pallet_custom_origins::Origin, Dmp};
	pub(crate) use sp_runtime::{traits::Dispatchable, DispatchError};
	pub(crate) use xcm::{latest::prelude::*, VersionedLocation, VersionedXcm};

	pub(crate) use polkadot_system_emulated_network::{
		AssetHubPolkadotPara as AssetHubPolkadot, BridgeHubPolkadotPara as BridgeHubPolkadot,
		CollectivesPolkadotPara as CollectivesPolkadot, CoretimePolkadotPara as CoretimePolkadot,
		PeoplePolkadotPara as PeoplePolkadot, PolkadotRelay as Polkadot,
	};

	pub(crate) use integration_tests_helpers::*;
}

#[cfg(test)]
mod common;

#[cfg(test)]
mod open_gov_on_asset_hub;

#[cfg(test)]
mod open_gov_on_relay;
