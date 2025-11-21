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

pub mod call_filter;

extern crate alloc;

use super::*;
use alloc::boxed::Box;
use frame_support::pallet_prelude::{PalletInfoAccess, TypeInfo};
use polkadot_runtime_common::impls::VersionedLocatableAsset;
use sp_core::Get;
use sp_runtime::traits::TryConvert;

/// Treasury accounts migrating to the new treasury account address (same account address that was
/// used on the Relay Chain).
pub struct TreasuryAccounts;
impl Get<(AccountId, Vec<cumulus_primitives_core::Location>)> for TreasuryAccounts {
	fn get() -> (AccountId, Vec<cumulus_primitives_core::Location>) {
		let assets_id = <crate::Assets as PalletInfoAccess>::index() as u8;
		(
			xcm_config::PreMigrationRelayTreasuryPalletAccount::get(),
			vec![
				// USDT
				cumulus_primitives_core::Location::new(
					0,
					[
						cumulus_primitives_core::Junction::PalletInstance(assets_id),
						cumulus_primitives_core::Junction::GeneralIndex(1984),
					],
				),
				// USDC
				cumulus_primitives_core::Location::new(
					0,
					[
						cumulus_primitives_core::Junction::PalletInstance(assets_id),
						cumulus_primitives_core::Junction::GeneralIndex(1337),
					],
				),
				// DED
				cumulus_primitives_core::Location::new(
					0,
					[
						cumulus_primitives_core::Junction::PalletInstance(assets_id),
						cumulus_primitives_core::Junction::GeneralIndex(30),
					],
				),
				// STINK
				cumulus_primitives_core::Location::new(
					0,
					[
						cumulus_primitives_core::Junction::PalletInstance(assets_id),
						cumulus_primitives_core::Junction::GeneralIndex(42069),
					],
				),
			],
		)
	}
}

pub type RcProxyType = polkadot_runtime_constants::proxy::ProxyType;

pub struct RcToProxyType;
impl TryConvert<RcProxyType, ProxyType> for RcToProxyType {
	fn try_convert(p: RcProxyType) -> Result<ProxyType, RcProxyType> {
		use polkadot_runtime_constants::proxy::ProxyType::*;

		match p {
			Any => Ok(ProxyType::Any),
			NonTransfer => Ok(ProxyType::NonTransfer),
			Governance => Ok(ProxyType::Governance),
			Staking => Ok(ProxyType::Staking),
			CancelProxy => Ok(ProxyType::CancelProxy),
			Auction => Ok(ProxyType::Auction),
			NominationPools => Ok(ProxyType::NominationPools),
			ParaRegistration => Ok(ProxyType::ParaRegistration),
		}
	}
}

/// A subset of Relay Chain origins.
///
/// These origins are utilized in Governance and mapped to Asset Hub origins for active referendums.
#[allow(non_camel_case_types)]
#[derive(Encode, DecodeWithMemTracking, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcPalletsOrigin {
	#[codec(index = 0u8)]
	system(frame_system::Origin<Runtime>),
	#[codec(index = 22u8)]
	Origins(pallet_custom_origins::Origin),
}

impl Default for RcPalletsOrigin {
	fn default() -> Self {
		RcPalletsOrigin::system(frame_system::Origin::<Runtime>::Root)
	}
}

/// Convert a Relay Chain origin to an Asset Hub one.
pub struct RcToAhPalletsOrigin;
impl TryConvert<RcPalletsOrigin, OriginCaller> for RcToAhPalletsOrigin {
	fn try_convert(a: RcPalletsOrigin) -> Result<OriginCaller, RcPalletsOrigin> {
		match a {
			RcPalletsOrigin::system(a) => Ok(OriginCaller::system(a)),
			RcPalletsOrigin::Origins(a) => Ok(OriginCaller::Origins(a)),
		}
	}
}

/// Relay Chain Runtime Call.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcRuntimeCall {
	#[codec(index = 0u8)]
	System(frame_system::Call<Runtime>),
	#[codec(index = 1u8)]
	Scheduler(RcSchedulerCall),
	#[codec(index = 19u8)]
	Treasury(RcTreasuryCall),
	#[codec(index = 21u8)]
	Referenda(pallet_referenda::Call<Runtime>),
	#[codec(index = 26u8)]
	Utility(RcUtilityCall),
	#[codec(index = 34u8)]
	Bounties(pallet_bounties::Call<Runtime>),
	#[codec(index = 38u8)]
	ChildBounties(pallet_child_bounties::Call<Runtime>),
	#[codec(index = 99u8)]
	XcmPallet(RcXcmCall),
}

/// Relay Chain Treasury Call obtained from cargo expand.
#[allow(non_camel_case_types)]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcTreasuryCall {
	/// Propose and approve a spend of treasury funds.
	#[codec(index = 3u8)]
	spend_local {
		#[codec(compact)]
		amount: Balance,
		beneficiary: Address,
	},
	/// Force a previously approved proposal to be removed from the approval queue.
	#[codec(index = 4u8)]
	remove_approval {
		#[codec(compact)]
		proposal_id: pallet_treasury::ProposalIndex,
	},
	/// Propose and approve a spend of treasury funds.
	#[codec(index = 5u8)]
	spend {
		asset_kind: Box<VersionedLocatableAsset>,
		#[codec(compact)]
		amount: Balance,
		beneficiary: Box<VersionedLocation>,
		valid_from: Option<BlockNumber>,
	},
	/// Claim a spend.
	#[codec(index = 6u8)]
	payout { index: pallet_treasury::SpendIndex },
	#[codec(index = 7u8)]
	check_status { index: pallet_treasury::SpendIndex },
	#[codec(index = 8u8)]
	void_spend { index: pallet_treasury::SpendIndex },
}

/// Relay Chain Utility Call obtained from cargo expand.
///
/// The variants that are not generally used in Governance are not included.
#[allow(non_camel_case_types)]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcUtilityCall {
	/// Send a batch of dispatch calls.
	#[codec(index = 0u8)]
	batch { calls: Vec<RcRuntimeCall> },
	/// Send a batch of dispatch calls and atomically execute them.
	#[codec(index = 2u8)]
	batch_all { calls: Vec<RcRuntimeCall> },
	/// Dispatches a function call with a provided origin.
	#[codec(index = 3u8)]
	dispatch_as { as_origin: Box<RcPalletsOrigin>, call: Box<RcRuntimeCall> },
	/// Send a batch of dispatch calls.
	/// Unlike `batch`, it allows errors and won't interrupt.
	#[codec(index = 4u8)]
	force_batch { calls: Vec<RcRuntimeCall> },
}

/// Relay Chain Scheduler Call.
///
/// The variants that are not generally used in Governance are not included.
#[allow(non_camel_case_types)]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcSchedulerCall {
	#[codec(index = 4u8)]
	schedule_after {
		after: BlockNumber,
		maybe_periodic: Option<frame_support::traits::schedule::Period<BlockNumber>>,
		priority: frame_support::traits::schedule::Priority,
		call: Box<RcRuntimeCall>,
	},
}

/// Relay Chain XCM Call.
///
/// The variants that are not generally used in Governance are not included.
#[allow(non_camel_case_types)]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcXcmCall {
	#[codec(index = 0u8)]
	send { dest: Box<VersionedLocation>, message: Box<VersionedXcm<()>> },
	#[codec(index = 3u8)]
	execute { message: Box<VersionedXcm<()>>, max_weight: Weight },
	#[codec(index = 8u8)]
	limited_reserve_transfer_assets {
		dest: Box<VersionedLocation>,
		beneficiary: Box<VersionedLocation>,
		assets: Box<VersionedAssets>,
		fee_asset_item: u32,
		weight_limit: WeightLimit,
	},
}
