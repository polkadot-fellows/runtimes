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
use frame_support::pallet_prelude::TypeInfo;
use sp_runtime::traits::Convert;

/// Relay Chain Hold Reason
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcHoldReason {
	#[codec(index = 10u8)]
	Preimage(pallet_preimage::HoldReason),
	// TODO
	// #[codec(index = 98u8)]
	// StateTrieMigration(pallet_state_trie_migration::HoldReason),
}

/// Relay Chain Freeze Reason
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcFreezeReason {
	// TODO
	// #[codec(index = 39u8)]
	// NominationPools(pallet_nomination_pools::FreezeReason),
}

pub struct RcToAhHoldReason;
impl Convert<RcHoldReason, RuntimeHoldReason> for RcToAhHoldReason {
	fn convert(a: RcHoldReason) -> RuntimeHoldReason {
		match a {
			// TODO mapping
			_ => PreimageHoldReason::get(),
		}
	}
}

pub struct RcToAhFreezeReason;
impl Convert<RcFreezeReason, ()> for RcToAhFreezeReason {
	fn convert(a: RcFreezeReason) -> () {
		match a {
			// TODO mapping
			_ => (),
		}
	}
}
