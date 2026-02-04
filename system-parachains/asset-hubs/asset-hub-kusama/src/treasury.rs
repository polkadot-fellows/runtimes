// Copyright (C) Polkadot Fellows.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! This file contains relevant configuration of treasury (migrated from the RC with AHM).

use super::*;

use crate::governance::{Treasurer, TreasurySpender};
use frame_support::traits::{
	fungible::HoldConsideration,
	tokens::{UnityOrOuterConversion, ConversionFromAssetBalance, ConversionToAssetBalance},
	Currency, FromContains, Get, OnUnbalanced,
};
use parachains_common::pay::{AccountIdToLocalLocation, LocalPay, VersionedLocatableAccount};
use polkadot_runtime_common::impls::{ContainsParts, VersionedLocatableAsset};
use scale_info::TypeInfo;
use sp_runtime::traits::IdentityLookup;

parameter_types! {
	pub const SpendPeriod: BlockNumber = 6 * RC_DAYS;
	pub const DisableSpends: BlockNumber = BlockNumber::MAX;
	pub const Burn: Permill = Permill::from_percent(1);
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const PayoutSpendPeriod: BlockNumber = 90 * RC_DAYS;
	pub const MaxApprovals: u32 = 100;
	// Account address: `13UVJyLnbVp9RBZYFwFGyDvVd1y27Tt8tkntv6Q7JVPhFsTB`
	pub TreasuryAccount: AccountId = Treasury::account_id();
}

pub type TreasuryPaymaster = parachains_common::pay::LocalPay<
	NativeAndAssets,
	TreasuryAccount,
	xcm_config::LocationToAccountId,
>;

#[derive(
	Default,
	MaxEncodedLen,
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	Clone,
	Eq,
	PartialEq,
	Debug,
)]
pub struct BurnDestinationAccount(pub Option<polkadot_core_primitives::AccountId>);

impl BurnDestinationAccount {
	pub fn is_set(&self) -> bool {
		self.0.is_some()
	}
}

pub type BalancesNegativeImbalance = <Balances as Currency<AccountId>>::NegativeImbalance;
pub struct TreasuryBurnHandler;

impl OnUnbalanced<BalancesNegativeImbalance> for TreasuryBurnHandler {
	fn on_nonzero_unbalanced(amount: BalancesNegativeImbalance) {
		let destination = dynamic_params::treasury::BurnDestination::get();

		if let BurnDestinationAccount(Some(account)) = destination {
			// Must resolve into existing but better to be safe.
			Balances::resolve_creating(&account, amount);
		} else {
			// If no account to destinate the funds to, just drop the
			// imbalance.
			<() as OnUnbalanced<_>>::on_nonzero_unbalanced(amount)
		}
	}
}

impl Get<Permill> for TreasuryBurnHandler {
	fn get() -> Permill {
		let destination = dynamic_params::treasury::BurnDestination::get();

		if destination.is_set() {
			dynamic_params::treasury::BurnPortion::get()
		} else {
			Permill::zero()
		}
	}
}

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = pallet_ah_migrator::LeftOrRight<AhMigrator, DisableSpends, SpendPeriod>;
	type Burn = TreasuryBurnHandler;
	type BurnDestination = TreasuryBurnHandler;
	type SpendFunds = Bounties;
	type MaxApprovals = MaxApprovals;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type SpendOrigin = TreasurySpender;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocatableAccount;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type Paymaster = TreasuryPaymaster;
	type BalanceConverter = AssetRateWithNative;
	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = parachains_common::pay::benchmarks::LocalPayArguments<
		xcm_config::TrustBackedAssetsPalletIndex,
	>;
	type BlockNumberProvider = RelaychainDataProvider<Runtime>;
}

parameter_types! {
	// where `176` is the size of the `Bounty` struct in bytes.
	pub const BountyDepositBase: Balance = system_para_deposit(0, 176);
	// per byte for the bounty description.
	pub const DataDepositPerByte: Balance = system_para_deposit(0, 1);
	pub const BountyDepositPayoutDelay: BlockNumber = 0;
	// Bounties expire after 10 years.
	pub const BountyUpdatePeriod: BlockNumber = 10 * 12 * 30 * RC_DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 10 * CENTS;
	pub const CuratorDepositMax: Balance = 500 * CENTS;
	pub const BountyValueMinimum: Balance = 200 * CENTS;
}

impl pallet_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyManager = ChildBounties;
	type DataDepositPerByte = DataDepositPerByte;
	type MaximumReasonLength = MaximumReasonLength;
	type OnSlash = Treasury;
	type WeightInfo = weights::pallet_bounties::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MaxActiveChildBountyCount: u32 = 100;
	pub const ChildBountyValueMinimum: Balance = BountyValueMinimum::get() / 10;
}

impl pallet_child_bounties::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type WeightInfo = weights::pallet_child_bounties::WeightInfo<Runtime>;
}

parameter_types! {
	pub const MultiAssetBountyValueMinimum: Balance = 200 * CENTS;
	pub const MultiAssetChildBountyValueMinimum: Balance = MultiAssetBountyValueMinimum::get() / 10;
	pub const MultiAssetMaxActiveChildBountyCount: u32 = 100;
	pub const MultiAssetCuratorHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::MultiAssetBounties(pallet_multi_asset_bounties::HoldReason::CuratorDeposit);
	pub const MultiAssetCuratorDepositFromValueMultiplier: Permill = Permill::from_percent(50);
	pub const MultiAssetCuratorDepositMin: Balance = 10 * CENTS;
	pub const MultiAssetCuratorDepositMax: Balance = 500 * CENTS;
}

impl pallet_multi_asset_bounties::Config for Runtime {
	type Balance = Balance;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type SpendOrigin = TreasurySpender;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocatableAccount;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type BountyValueMinimum = MultiAssetBountyValueMinimum;
	type ChildBountyValueMinimum = MultiAssetChildBountyValueMinimum;
	type MaxActiveChildBountyCount = MultiAssetMaxActiveChildBountyCount;
	type WeightInfo = weights::pallet_multi_asset_bounties::WeightInfo<Runtime>; // todo @dhiraj
	type FundingSource = pallet_multi_asset_bounties::PalletIdAsFundingSource<
		TreasuryPalletId,
		Runtime,
		AccountIdToLocalLocation,
	>;
	type BountySource = pallet_multi_asset_bounties::BountySourceFromPalletId<
		TreasuryPalletId,
		Runtime,
		AccountIdToLocalLocation,
	>;
	type ChildBountySource = pallet_multi_asset_bounties::ChildBountySourceFromPalletId<
		TreasuryPalletId,
		Runtime,
		AccountIdToLocalLocation,
	>;
	type Paymaster = LocalPay<NativeAndAssets, AccountId, xcm_config::LocationToAccountId>;
	type BalanceConverter = AssetRateWithNative;
	type Preimages = Preimage;
	type Consideration = HoldConsideration<
		AccountId,
		Balances,
		MultiAssetCuratorHoldReason,
		pallet_multi_asset_bounties::CuratorDepositAmount<
			MultiAssetCuratorDepositFromValueMultiplier,
			MultiAssetCuratorDepositMin,
			MultiAssetCuratorDepositMax,
			Balance,
		>,
		Balance,
	>;
	#[cfg(feature = "runtime-benchmarks")] 
	type BenchmarkHelper = system_parachains_common::pay::benchmarks::LocalPayWithSourceArguments<
		xcm_config::TrustBackedAssetsPalletIndex,
	>;
}

/// The [frame_support::traits::tokens::ConversionFromAssetBalance] implementation provided by the
/// `AssetRate` pallet instance.
///
/// With additional decoration to identify different IDs/locations of native asset and provide a
/// one-to-one balance conversion for them.
pub type AssetRateWithNative = UnityOrOuterConversion<
	ContainsParts<
		FromContains<
			(
				xcm_builder::IsSiblingSystemParachain<ParaId, xcm_config::SelfParaId>,
				Equals<xcm_config::Here>,
			),
			xcm_builder::IsParentsOnly<ConstU8<1>>,
		>,
	>,
	AssetRate,
>;

impl pallet_asset_rate::Config for Runtime {
	type WeightInfo = weights::pallet_asset_rate::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type CreateOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RemoveOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type UpdateOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type Currency = Balances;
	type AssetKind = <Runtime as pallet_treasury::Config>::AssetKind;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::AssetRateArguments;
}
