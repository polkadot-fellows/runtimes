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
use frame_support::traits::{Contains, EitherOfDiverse, MapSuccess};
use frame_system::EnsureRoot;
use pallet_xcm::EnsureXcm;
use polkadot_runtime_constants::fellowship::IsFellowshipVoice;
use sp_runtime::traits::Replace;

parameter_types! {
	pub const EnterDepositAmount: Option<Balance> = Some(100_000 * UNITS);
	pub const ExtendDepositAmount: Option<Balance> = Some(100_000 * UNITS);
	pub const EnterDuration: BlockNumber = 24 * RC_HOURS; // 1 day
	pub const ExtendDuration: BlockNumber = 24 * RC_HOURS; // 1 day
	pub const ReleaseDelay: Option<BlockNumber> = Some(60 * RC_DAYS); // 60 days
}

pub struct SafeModeWhitelist;
impl Contains<RuntimeCall> for SafeModeWhitelist {
	fn contains(call: &RuntimeCall) -> bool {
		matches!(
			call,
			RuntimeCall::System(_) |
				RuntimeCall::Timestamp(_) |
				RuntimeCall::ParachainSystem(_) |
				RuntimeCall::Referenda(_) |
				RuntimeCall::ConvictionVoting(_) |
				RuntimeCall::Preimage(_) |
				RuntimeCall::TxPause(_)
		)
	}
}

impl pallet_safe_mode::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WhitelistedCalls = SafeModeWhitelist;
	type EnterDuration = EnterDuration;
	type ExtendDuration = ExtendDuration;
	type EnterDepositAmount = EnterDepositAmount;
	type ExtendDepositAmount = ExtendDepositAmount;
	// ForceEnterOrigin success value must be BlockNumber (duration); use Replace to map () ->
	// EnterDuration
	type ForceEnterOrigin = MapSuccess<
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsFellowshipVoice<FellowshipLocation>>>,
		Replace<EnterDuration>,
	>;
	// ForceExtendOrigin success value must be BlockNumber (duration); use Replace to map () ->
	// ExtendDuration
	type ForceExtendOrigin = MapSuccess<
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsFellowshipVoice<FellowshipLocation>>>,
		Replace<ExtendDuration>,
	>;
	type ForceExitOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsFellowshipVoice<FellowshipLocation>>>;
	type ForceDepositOrigin =
		EitherOfDiverse<EnsureRoot<AccountId>, EnsureXcm<IsFellowshipVoice<FellowshipLocation>>>;
	type Notify = ();
	type ReleaseDelay = ReleaseDelay;
	// TODO: we will replace with benchmarked weights once `frame-omni-bencher` has been run on this
	// chain.
	type WeightInfo = pallet_safe_mode::weights::SubstrateWeight<Runtime>;
}
