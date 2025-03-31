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
use codec::DecodeAll;
use frame_support::pallet_prelude::{PalletInfoAccess, TypeInfo};
use frame_system::pallet_prelude::BlockNumberFor;
use pallet_ah_migrator::LOG_TARGET;
use polkadot_runtime_common::impls::{LocatableAssetConverter, VersionedLocatableAsset};
use sp_core::Get;
use sp_runtime::traits::{Convert, TryConvert};
use system_parachains_common::pay::VersionedLocatableAccount;
use xcm::latest::prelude::*;

/// Treasury accounts migrating to the new treasury account address (same account address that was
/// used on the Relay Chain).
pub struct TreasuryAccounts;
impl Get<(AccountId, Vec<Location>)> for TreasuryAccounts {
	fn get() -> (AccountId, Vec<Location>) {
		let assets_id = <crate::Assets as PalletInfoAccess>::index() as u8;
		(
			xcm_config::RelayTreasuryPalletAccount::get(),
			vec![
				// USDT
				Location::new(0, [PalletInstance(assets_id), GeneralIndex(1984)]),
				// USDC
				Location::new(0, [PalletInstance(assets_id), GeneralIndex(1337)]),
				// DED
				Location::new(0, [PalletInstance(assets_id), GeneralIndex(30)]),
				// STINK
				Location::new(0, [PalletInstance(assets_id), GeneralIndex(42069)]),
			],
		)
	}
}

/// Relay Chain Hold Reason
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcHoldReason {
	#[codec(index = 10)]
	Preimage(pallet_preimage::HoldReason),
	#[codec(index = 98)]
	StateTrieMigration(pallet_state_trie_migration::HoldReason),
	#[codec(index = 41)]
	DelegatedStaking(pallet_delegated_staking::HoldReason),
}

impl Default for RcHoldReason {
	fn default() -> Self {
		RcHoldReason::Preimage(pallet_preimage::HoldReason::Preimage)
	}
}

/// Relay Chain Freeze Reason
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcFreezeReason {
	#[codec(index = 39u8)]
	NominationPools(pallet_nomination_pools::FreezeReason),
}

impl Default for RcFreezeReason {
	fn default() -> Self {
		RcFreezeReason::NominationPools(pallet_nomination_pools::FreezeReason::PoolMinBalance)
	}
}

pub struct RcToAhHoldReason;
impl Convert<RcHoldReason, RuntimeHoldReason> for RcToAhHoldReason {
	fn convert(_: RcHoldReason) -> RuntimeHoldReason {
		PreimageHoldReason::get()
	}
}

pub struct RcToAhFreezeReason;
impl Convert<RcFreezeReason, RuntimeFreezeReason> for RcToAhFreezeReason {
	fn convert(reason: RcFreezeReason) -> RuntimeFreezeReason {
		match reason {
			RcFreezeReason::NominationPools(
				pallet_nomination_pools::FreezeReason::PoolMinBalance,
			) => RuntimeFreezeReason::NominationPools(
				pallet_nomination_pools::FreezeReason::PoolMinBalance,
			),
		}
	}
}

/// Relay Chain Proxy Type
///
/// Coped from https://github.com/polkadot-fellows/runtimes/blob/dde99603d7dbd6b8bf541d57eb30d9c07a4fce32/relay/polkadot/src/lib.rs#L986-L1010
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, Default)]
pub enum RcProxyType {
	#[default]
	Any = 0,
	NonTransfer = 1,
	Governance = 2,
	Staking = 3,
	// Skip 4 as it is now removed (was SudoBalances)
	// Skip 5 as it was IdentityJudgement
	CancelProxy = 6,
	Auction = 7,
	NominationPools = 8,
	ParaRegistration = 9,
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
			RcProxyType::ParaRegistration => Err(p), // Does not exist on AH
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

/// A subset of Relay Chain origins.
///
/// These origins are utilized in Governance and mapped to Asset Hub origins for active referendums.
#[allow(non_camel_case_types)]
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum RcPalletsOrigin {
	#[codec(index = 0u8)]
	system(frame_system::Origin<Runtime>),
	#[codec(index = 22u8)]
	Origins(pallet_custom_origins::Origin),
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
	// TODO: variant set code for Relay Chain
	// TODO: variant set code for Parachains
	// TODO: whitelisted caller
	// TODO: remark
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
				log::error!(target: LOG_TARGET, "Failed to decode RC call with error: {:?}", e);
				return Err(a)
			},
		};
		Self::map(rc_call).map_err(|_| a)
	}
}
impl RcToAhCall {
	fn map(rc_call: RcRuntimeCall) -> Result<RuntimeCall, ()> {
		match rc_call {
			RcRuntimeCall::Utility(RcUtilityCall::dispatch_as { as_origin, call }) => {
				let origin = RcToAhPalletsOrigin::try_convert(*as_origin).map_err(|err| {
					log::error!(
						target: LOG_TARGET,
						"Failed to decode RC dispatch_as origin: {:?}",
						err
					);
				})?;
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::dispatch_as {
					as_origin: Box::new(origin),
					call: Box::new(Self::map(*call)?),
				}))
			},
			RcRuntimeCall::Utility(RcUtilityCall::batch { calls }) =>
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::batch {
					calls: calls
						.into_iter()
						.map(|c| Self::map(c))
						.collect::<Result<Vec<_>, _>>()?,
				})),
			RcRuntimeCall::Utility(RcUtilityCall::batch_all { calls }) =>
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::batch_all {
					calls: calls
						.into_iter()
						.map(|c| Self::map(c))
						.collect::<Result<Vec<_>, _>>()?,
				})),
			RcRuntimeCall::Utility(RcUtilityCall::force_batch { calls }) =>
				Ok(RuntimeCall::Utility(pallet_utility::Call::<Runtime>::force_batch {
					calls: calls
						.into_iter()
						.map(|c| Self::map(c))
						.collect::<Result<Vec<_>, _>>()?,
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
							"Failed to decode inner RC call into inner AH call: {:?}",
							err
						);
					})?;
				Ok(RuntimeCall::Treasury(call))
			},
			RcRuntimeCall::Referenda(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode RC Referenda call to AH Referenda call: {:?}",
							err
						);
					})?;
				Ok(RuntimeCall::Referenda(call))
			},
			RcRuntimeCall::Bounties(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode RC Bounties call to AH Bounties call: {:?}",
							err
						);
					})?;
				Ok(RuntimeCall::Bounties(call))
			},
			RcRuntimeCall::ChildBounties(inner_call) => {
				let call =
					inner_call.using_encoded(|mut e| Decode::decode(&mut e)).map_err(|err| {
						log::error!(
							target: LOG_TARGET,
							"Failed to decode RC ChildBounties call to AH ChildBounties call: {:?}",
							err
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
		rc: (VersionedLocatableAsset, VersionedLocation),
	) -> Result<(VersionedLocatableAsset, VersionedLocatableAccount), ()> {
		let (asset_kind, beneficiary) = rc;
		let asset_kind = LocatableAssetConverter::try_convert(asset_kind).map_err(|_| {
			log::error!(target: LOG_TARGET, "Failed to convert RC asset kind to latest version");
		})?;
		if asset_kind.location != Location::new(0, Parachain(1000)) {
			log::error!(
				target: LOG_TARGET,
				"Unsupported RC asset kind location: {:?}",
				asset_kind.location
			);
			return Err(());
		};
		let asset_kind = VersionedLocatableAsset::V4 {
			location: Location::here(),
			asset_id: asset_kind.asset_id,
		};
		let beneficiary = beneficiary.try_into().map_err(|_| {
			log::error!(
				target: LOG_TARGET,
				"Failed to convert RC beneficiary type to the latest version"
			);
		})?;
		let beneficiary =
			VersionedLocatableAccount::V4 { location: Location::here(), account_id: beneficiary };
		Ok((asset_kind, beneficiary))
	}
}
