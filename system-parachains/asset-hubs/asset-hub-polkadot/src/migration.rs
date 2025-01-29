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
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::traits::{Convert, TryConvert};

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
	fn convert(_: RcHoldReason) -> RuntimeHoldReason {
		PreimageHoldReason::get()
	}
}

pub struct RcToAhFreezeReason;
impl Convert<RcFreezeReason, RuntimeFreezeReason> for RcToAhFreezeReason {
	fn convert(_: RcFreezeReason) -> RuntimeFreezeReason {
		todo!()
	}
}
/// Relay Chain Proxy Type
///
/// Coped from https://github.com/polkadot-fellows/runtimes/blob/dde99603d7dbd6b8bf541d57eb30d9c07a4fce32/relay/polkadot/src/lib.rs#L986-L1010
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcProxyType {
	Any = 0,
	NonTransfer = 1,
	Governance = 2,
	Staking = 3,
	// Skip 4 as it is now removed (was SudoBalances)
	// Skip 5 as it was IdentityJudgement
	CancelProxy = 6,
	Auction = 7,
	NominationPools = 8,
}

pub struct RcToProxyType;
impl TryConvert<RcProxyType, ProxyType> for RcToProxyType {
	fn try_convert(p: RcProxyType) -> Result<ProxyType, RcProxyType> {
		match p {
			RcProxyType::Any => Ok(ProxyType::Any),
			RcProxyType::NonTransfer => Ok(ProxyType::NonTransfer),
			RcProxyType::Governance => Ok(ProxyType::Governance),
			RcProxyType::Staking => Ok(ProxyType::Staking),
			RcProxyType::CancelProxy => Ok(ProxyType::CancelProxy),
			RcProxyType::Auction => Err(p), // Does not exist on AH
			RcProxyType::NominationPools => Ok(ProxyType::NominationPools),
		}
	}
}

/// Convert a Relay Chain Proxy Delay to a local AH one.
// NOTE we assume Relay Chain and AH to have the same block type
pub struct RcToAhDelay;
impl Convert<BlockNumberFor<Runtime>, BlockNumberFor<Runtime>> for RcToAhDelay {
	fn convert(rc: BlockNumberFor<Runtime>) -> BlockNumberFor<Runtime> {
		// Polkadot Relay chain: 6 seconds per block
		// Asset Hub: 12 seconds per block
		rc / 2
	}
}
