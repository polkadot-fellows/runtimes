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

use super::*;
use crate::xcm_config::FellowshipLocation;
use frame_support::traits::{EitherOfDiverse, Nothing};
use frame_system::EnsureRoot;
use pallet_xcm::EnsureXcm;
use polkadot_runtime_constants::fellowship::IsFellowshipVoice;

parameter_types! {
	pub const MaxNameLen: u32 = 256;
}

impl pallet_tx_pause::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PauseOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsFellowshipVoice<FellowshipLocation>>>;
	type UnpauseOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsFellowshipVoice<FellowshipLocation>>>;
	type WhitelistedCalls = Nothing;
	type MaxNameLen = MaxNameLen;
	// TODO: replace with benchmarked weights once `frame-omni-bencher` has been run on this chain.
	type WeightInfo = pallet_tx_pause::weights::SubstrateWeight<Runtime>;
}
