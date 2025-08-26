// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Bridge definitions that can be used by multiple bridges.

use crate::{
	bridge_to_ethereum_config::InboundQueueV2Location,
	weights,
	xcm_config::{XcmConfig, XcmRouter},
	AccountId, Balance, Balances, BlockNumber, Runtime, RuntimeCall, RuntimeEvent,
};
use alloc::boxed::Box;
use bp_bridge_hub_polkadot::snowbridge::EthereumNetwork;
use bp_messages::LegacyLaneId;
use bp_relayers::RewardsAccountParams;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use frame_support::parameter_types;
use polkadot_runtime_constants as constants;
use scale_info::TypeInfo;
use system_parachains_constants::polkadot::locations::AssetHubLocation;
use xcm::{opaque::latest::Location, VersionedLocation};
use xcm_executor::XcmExecutor;

parameter_types! {
	/// Reserve identifier, used by the `pallet_bridge_relayers` to hold funds of registered relayer.
	pub const RelayerStakeReserveId: [u8; 8] = *b"brdgrlrs";
	/// Minimal period of relayer registration. Roughly, it is the 1 hour of real time.
	pub const RelayerStakeLease: u32 = 300;
	/// Registered relayer stake.
	///
	/// Any relayer may reserve this amount on his account and get a priority boost for his
	/// message delivery transactions. In exchange, he risks losing his stake if he would
	/// submit an invalid transaction. The set of such (registered) relayers is tracked
	/// by the `pallet_bridge_relayers` pallet at the Polkadot Bridge Hub.
	pub storage RequiredStakeForStakeAndSlash: Balance = 500 * constants::currency::UNITS;
}

/// Showcasing that we can handle multiple different rewards with the same pallet.
#[derive(
	Clone,
	Copy,
	Debug,
	Decode,
	DecodeWithMemTracking,
	Encode,
	Eq,
	MaxEncodedLen,
	PartialEq,
	TypeInfo,
)]
pub enum BridgeReward {
	/// Rewards for the P/K bridge—distinguished by the `RewardsAccountParams` key.
	PolkadotKusamaBridge(RewardsAccountParams<LegacyLaneId>),
	/// Rewards for Snowbridge.
	Snowbridge,
}

impl From<RewardsAccountParams<LegacyLaneId>> for BridgeReward {
	fn from(value: RewardsAccountParams<LegacyLaneId>) -> Self {
		Self::PolkadotKusamaBridge(value)
	}
}

/// An enum representing the different types of supported beneficiaries.
#[derive(
	Clone, Debug, Decode, DecodeWithMemTracking, Encode, Eq, MaxEncodedLen, PartialEq, TypeInfo,
)]
pub enum BridgeRewardBeneficiaries {
	/// A local chain account.
	LocalAccount(AccountId),
	/// A beneficiary specified by a VersionedLocation.
	AssetHubLocation(Box<VersionedLocation>),
}

impl From<sp_runtime::AccountId32> for BridgeRewardBeneficiaries {
	fn from(value: sp_runtime::AccountId32) -> Self {
		BridgeRewardBeneficiaries::LocalAccount(value)
	}
}

/// Implementation of `bp_relayers::PaymentProcedure` as a pay/claim rewards scheme.
pub struct BridgeRewardPayer;
impl bp_relayers::PaymentProcedure<AccountId, BridgeReward, u128> for BridgeRewardPayer {
	type Error = sp_runtime::DispatchError;
	type Beneficiary = BridgeRewardBeneficiaries;

	fn pay_reward(
		relayer: &AccountId,
		reward_kind: BridgeReward,
		reward: u128,
		beneficiary: BridgeRewardBeneficiaries,
	) -> Result<(), Self::Error> {
		match reward_kind {
			BridgeReward::PolkadotKusamaBridge(lane_params) => {
				match beneficiary {
					BridgeRewardBeneficiaries::LocalAccount(account) => {
						bp_relayers::PayRewardFromAccount::<
							Balances,
							AccountId,
							LegacyLaneId,
							u128,
						>::pay_reward(
							relayer, lane_params, reward, account,
						)
					},
					BridgeRewardBeneficiaries::AssetHubLocation(_) => Err(Self::Error::Other("`AssetHubLocation` beneficiary is not supported for `PolkadotKusamaBridge` rewards!")),
				}
			},
			BridgeReward::Snowbridge => {
				match beneficiary {
					BridgeRewardBeneficiaries::LocalAccount(_) => Err(Self::Error::Other("`LocalAccount` beneficiary is not supported for `Snowbridge` rewards!")),
					BridgeRewardBeneficiaries::AssetHubLocation(account_location) => {
						let account_location = Location::try_from(account_location.as_ref().clone())
							.map_err(|_| Self::Error::Other("`AssetHubLocation` beneficiary location version is not supported for `Snowbridge` rewards!"))?;
						snowbridge_core::reward::PayAccountOnLocation::<
							AccountId,
							u128,
							EthereumNetwork,
							AssetHubLocation,
							InboundQueueV2Location,
							XcmRouter,
							XcmExecutor<XcmConfig>,
							RuntimeCall
						>::pay_reward(
							relayer, (), reward, account_location
						)
					}
				}
			}
		}
	}
}

/// Allows collect and claim rewards for relayers.
pub type BridgeRelayersInstance = ();
impl pallet_bridge_relayers::Config<BridgeRelayersInstance> for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type RewardBalance = Balance;
	type Reward = BridgeReward;
	type PaymentProcedure = BridgeRewardPayer;
	type StakeAndSlash = pallet_bridge_relayers::StakeAndSlashNamed<
		AccountId,
		BlockNumber,
		Balances,
		RelayerStakeReserveId,
		RequiredStakeForStakeAndSlash,
		RelayerStakeLease,
	>;
	type Balance = Balance;
	type WeightInfo = weights::pallet_bridge_relayers::WeightInfo<Runtime>;
}
