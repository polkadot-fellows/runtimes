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
use codec::DecodeAll;
use frame_support::pallet_prelude::TypeInfo;
use pallet_ah_migrator::LOG_TARGET;
use parachains_common::pay::VersionedLocatableAccount;
use polkadot_runtime_common::impls::{LocatableAssetConverter, VersionedLocatableAsset};
use sp_core::Get;
use sp_runtime::traits::{Convert, TryConvert};

impl From<pallet_rc_migrator::types::PortableHoldReason> for RuntimeHoldReason {
	fn from(reason: pallet_rc_migrator::types::PortableHoldReason) -> Self {
		use pallet_rc_migrator::types::PortableHoldReason;
		use RuntimeHoldReason::*;

		match reason {
			PortableHoldReason::Preimage(preimage) => Preimage(preimage),
			PortableHoldReason::Staking(staking) => match staking {
				pallet_staking::HoldReason::Staking =>
					Staking(pallet_staking_async::HoldReason::Staking),
			},
			PortableHoldReason::StateTrieMigration(state_trie_migration) =>
				StateTrieMigration(state_trie_migration),
			PortableHoldReason::DelegatedStaking(delegated_staking) =>
				DelegatedStaking(delegated_staking),
			PortableHoldReason::Session(session) => Session(session),
			PortableHoldReason::XcmPallet(xcm_pallet) => PolkadotXcm(xcm_pallet),
		}
	}
}

impl From<pallet_rc_migrator::types::PortableFreezeReason> for RuntimeFreezeReason {
	fn from(reason: pallet_rc_migrator::types::PortableFreezeReason) -> Self {
		use pallet_rc_migrator::types::PortableFreezeReason;

		match reason {
			PortableFreezeReason::NominationPools(nomination_pools) =>
				RuntimeFreezeReason::NominationPools(nomination_pools),
		}
	}
}

/// Treasury accounts migrating to the new treasury account address (same account address that was
/// used on the Relay Chain).
pub struct TreasuryAccounts;
impl Get<(AccountId, Vec<cumulus_primitives_core::Location>)> for TreasuryAccounts {
	fn get() -> (AccountId, Vec<cumulus_primitives_core::Location>) {
		// Treasury account on Asset Hub has only KSM
		(xcm_config::PreMigrationRelayTreasuryPalletAccount::get(), vec![])
	}
}

pub type RcProxyType = kusama_runtime_constants::proxy::ProxyType;

pub struct RcToProxyType;
impl TryConvert<RcProxyType, ProxyType> for RcToProxyType {
	fn try_convert(p: RcProxyType) -> Result<ProxyType, RcProxyType> {
		use kusama_runtime_constants::proxy::ProxyType::*;

		match p {
			Any => Ok(ProxyType::Any),
			NonTransfer => Ok(ProxyType::NonTransfer),
			Governance => Ok(ProxyType::Governance),
			Staking => Ok(ProxyType::Staking),
			CancelProxy => Ok(ProxyType::CancelProxy),
			Auction => Ok(ProxyType::Auction),
			NominationPools => Ok(ProxyType::NominationPools),
			ParaRegistration => Ok(ProxyType::ParaRegistration),
			Society => Ok(ProxyType::Society),
			Spokesperson => Ok(ProxyType::Spokesperson),
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
	#[codec(index = 43u8)]
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
	#[codec(index = 18u8)]
	Treasury(RcTreasuryCall),
	#[codec(index = 21u8)]
	Referenda(pallet_referenda::Call<Runtime>),
	#[codec(index = 24u8)]
	Utility(RcUtilityCall),
	#[codec(index = 35u8)]
	Bounties(pallet_bounties::Call<Runtime>),
	#[codec(index = 40u8)]
	ChildBounties(pallet_child_bounties::Call<Runtime>),
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

/// Convert an encoded Relay Chain Call to a local AH one.
pub struct RcToAhCall;
impl<'a> TryConvert<&'a [u8], RuntimeCall> for RcToAhCall {
	fn try_convert(mut a: &'a [u8]) -> Result<RuntimeCall, &'a [u8]> {
		let rc_call = match RcRuntimeCall::decode_all(&mut a) {
			Ok(rc_call) => rc_call,
			Err(e) => {
				log::error!(target: LOG_TARGET, "Failed to decode RC call with error: {e:?}",);
				return Err(a)
			},
		};
		Self::map(rc_call).map_err(|_| a)
	}
}
impl RcToAhCall {
	fn map(rc_call: RcRuntimeCall) -> Result<RuntimeCall, ()> {
		match rc_call {
			RcRuntimeCall::System(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode RC Bounties call to AH System call: {err:?}",
						);
					})?;
				Ok(RuntimeCall::System(call))
			},
			RcRuntimeCall::Utility(RcUtilityCall::dispatch_as { as_origin, call }) => {
				let origin = RcToAhPalletsOrigin::try_convert(*as_origin).map_err(|err| {
					log::error!(
						target: LOG_TARGET,
						"Failed to decode RC dispatch_as origin: {err:?}",
					);
				})?;
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::dispatch_as {
					as_origin: Box::new(origin),
					call: Box::new(Self::map(*call)?),
				}))
			},
			RcRuntimeCall::Utility(RcUtilityCall::batch { calls }) =>
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::batch {
					calls: calls.into_iter().map(Self::map).collect::<Result<Vec<_>, _>>()?,
				})),
			RcRuntimeCall::Utility(RcUtilityCall::batch_all { calls }) =>
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::batch_all {
					calls: calls.into_iter().map(Self::map).collect::<Result<Vec<_>, _>>()?,
				})),
			RcRuntimeCall::Utility(RcUtilityCall::force_batch { calls }) =>
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::force_batch {
					calls: calls.into_iter().map(Self::map).collect::<Result<Vec<_>, _>>()?,
				})),
			RcRuntimeCall::Treasury(RcTreasuryCall::spend {
				asset_kind,
				amount,
				beneficiary,
				valid_from,
			}) => {
				let (asset_kind, beneficiary) =
					RcToAhTreasurySpend::convert((*asset_kind, *beneficiary))?;
				Ok(RuntimeCall::Treasury(pallet_treasury::Call::<Runtime>::spend {
					asset_kind: Box::new(asset_kind),
					amount,
					beneficiary: Box::new(beneficiary),
					valid_from,
				}))
			},
			RcRuntimeCall::Treasury(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode inner RC call into inner AH call: {err:?}",
						);
					})?;
				Ok(RuntimeCall::Treasury(call))
			},
			RcRuntimeCall::Referenda(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode RC Referenda call to AH Referenda call: {err:?}",
						);
					})?;
				Ok(RuntimeCall::Referenda(call))
			},
			RcRuntimeCall::Bounties(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode RC Bounties call to AH Bounties call: {err:?}",
						);
					})?;
				Ok(RuntimeCall::Bounties(call))
			},
			RcRuntimeCall::ChildBounties(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode RC ChildBounties call to AH ChildBounties call: {err:?}",
						);
					})?;
				Ok(RuntimeCall::ChildBounties(call))
			},
		}
	}
}

/// Convert RC Treasury Spend (AssetKind, Beneficiary) parameters to AH Treasury Spend (AssetKind,
/// Beneficiary) parameters.
pub struct RcToAhTreasurySpend;
impl
	Convert<
		(VersionedLocatableAsset, VersionedLocation),
		Result<(VersionedLocatableAsset, VersionedLocatableAccount), ()>,
	> for RcToAhTreasurySpend
{
	fn convert(
		(asset_kind, beneficiary): (VersionedLocatableAsset, VersionedLocation),
	) -> Result<(VersionedLocatableAsset, VersionedLocatableAccount), ()> {
		let asset_kind = LocatableAssetConverter::try_convert(asset_kind).map_err(|_| {
			log::error!(target: LOG_TARGET, "Failed to convert RC asset kind to latest version");
		})?;
		if asset_kind.location != xcm::v5::Location::new(0, xcm::v5::Junction::Parachain(1000)) {
			log::error!(
				target: LOG_TARGET,
				"Unsupported RC asset kind location: {:?}",
				asset_kind.location
			);
			return Err(());
		};
		let asset_kind = VersionedLocatableAsset::V5 {
			location: xcm::v5::Location::here(),
			asset_id: asset_kind.asset_id,
		};
		let beneficiary = beneficiary.try_into().map_err(|_| {
			log::error!(
				target: LOG_TARGET,
				"Failed to convert RC beneficiary type to the latest version"
			);
		})?;
		let beneficiary = VersionedLocatableAccount::V4 {
			location: xcm::v4::Location::here(),
			account_id: beneficiary,
		};
		Ok((asset_kind, beneficiary))
	}
}
