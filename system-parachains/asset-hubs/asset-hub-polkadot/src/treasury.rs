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

use crate::{governance::Treasurer, *};
use frame_support::traits::{
	fungible::HoldConsideration, tokens::UnityOrOuterConversion, FromContains,
};
use parachains_common::pay::{AccountIdToLocalLocation, LocalPay, VersionedLocatableAccount};
use polkadot_runtime_common::impls::{ContainsParts, VersionedLocatableAsset};

parameter_types! {
	pub const SpendPeriod: BlockNumber = 24 * RC_DAYS;
	pub const DisableSpends: BlockNumber = BlockNumber::MAX;
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

impl pallet_treasury::Config for Runtime {
	type PalletId = TreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type RuntimeEvent = RuntimeEvent;
	type SpendPeriod = pallet_ah_migrator::LeftOrRight<AhMigrator, DisableSpends, SpendPeriod>;
	type Burn = ();
	type BurnDestination = ();
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
	pub const CuratorDepositMin: Balance = 10 * DOLLARS;
	pub const CuratorDepositMax: Balance = 200 * DOLLARS;
	pub const BountyValueMinimum: Balance = 10 * DOLLARS;
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
	pub const MultiAssetCuratorHoldReason: RuntimeHoldReason =
		RuntimeHoldReason::MultiAssetBounties(pallet_multi_asset_bounties::HoldReason::CuratorDeposit);
}

impl pallet_multi_asset_bounties::Config for Runtime {
	type Balance = Balance;
	type RejectOrigin = EitherOfDiverse<EnsureRoot<AccountId>, Treasurer>;
	type SpendOrigin = TreasurySpender;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocatableAccount;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyValueMinimum = ChildBountyValueMinimum;
	type MaxActiveChildBountyCount = MaxActiveChildBountyCount;
	type WeightInfo = weights::pallet_multi_asset_bounties::WeightInfo<Runtime>;
	type FundingSource = pallet_multi_asset_bounties::PalletIdAsFundingSource<
		TreasuryPalletId,
		Runtime,
		AccountIdToLocalLocation,
	>;
	type BountySource =
		system_parachains_common::multi_asset_bounty_sources::MultiAssetBountySourceFromPalletId<
			TreasuryPalletId,
			Runtime,
			AccountIdToLocalLocation,
		>;
	type ChildBountySource =
		system_parachains_common::multi_asset_bounty_sources::MultiAssetChildBountySourceFromPalletId<
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
			CuratorDepositMultiplier,
			CuratorDepositMin,
			CuratorDepositMax,
			Balance,
		>,
		Balance,
	>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = parachains_common::pay::benchmarks::LocalPayWithSourceArguments<
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
