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

use frame_support::traits::tokens::{Pay, PaymentStatus, UnityAssetBalanceConversion};
use frame_support::traits::{Currency, Get, OnUnbalanced};
use scale_info::TypeInfo;
use sp_runtime::traits::IdentityLookup;
use sp_runtime::DispatchError;
use xcm::prelude::*;

parameter_types! {
	pub const TreasuryPalletId: PalletId = PalletId(*b"py/trsry");
	pub const MaxApprovals: u32 = 100;

	// TODO: AH or RC DAYS?
	pub const SpendPeriod: BlockNumber = 6 * DAYS;
	pub const PayoutSpendPeriod: BlockNumber = 90 * DAYS;

	// TODO: revisit !!! Location on RC, find out how is
	// The asset's interior location for the paying account. This is the Treasury
	// pallet instance (which sits at index 18).
	// pub TreasuryInteriorLocation: InteriorLocation = PalletInstance(TREASURY_PALLET_ID).into();
}

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
	type SpendPeriod = SpendPeriod;
	type Burn = TreasuryBurnHandler;
	type BurnDestination = TreasuryBurnHandler;
	type MaxApprovals = MaxApprovals;
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type SpendFunds = Bounties;
	type SpendOrigin = TreasurySpender;

	// TODO: Do we still need `VersionedLocatableAsset`? (Check treasury migration!)
	type AssetKind = VersionedAssetId;
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	// TODO: confirm `RelayTreasuryLocation` (RC PalletId = 18) after AHM - the same or other?
	// Only local payments to the local accounts.
	type Paymaster = LocalPayments<AccountId, NativeAndAssets>;

	// TODO: can we replace with pools? Do we really need `AssetRateWithNative` on AH?
	// type BalanceConverter = AssetRateWithNative;
	type BalanceConverter = UnityAssetBalanceConversion; /* TMP to get compile */
	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::TreasuryArguments;
	// TODO: check System or RC?
	type BlockNumberProvider = System;
}

parameter_types! {
	// TODO: revisis all the params!!!
	pub const BountyDepositBase: Balance = 100 * CENTS;
	pub const BountyDepositPayoutDelay: BlockNumber = 0;
	pub const BountyUpdatePeriod: BlockNumber = 10 * 12 * 30 * DAYS;
	pub const MaximumReasonLength: u32 = 16384;
	pub const DataDepositPerByte: Balance = CENTS / 10; /* TODO: system_para_deposit(0, 1); ? */
	pub const CuratorDepositMultiplier: Permill = Permill::from_percent(50);
	pub const CuratorDepositMin: Balance = 10 * CENTS;
	pub const CuratorDepositMax: Balance = 500 * CENTS;
	pub const BountyValueMinimum: Balance = 200 * CENTS;
}

impl pallet_bounties::Config for Runtime {
	type BountyDepositBase = BountyDepositBase;
	type BountyDepositPayoutDelay = BountyDepositPayoutDelay;
	type BountyUpdatePeriod = BountyUpdatePeriod;
	type CuratorDepositMultiplier = CuratorDepositMultiplier;
	type CuratorDepositMin = CuratorDepositMin;
	type CuratorDepositMax = CuratorDepositMax;
	type BountyValueMinimum = BountyValueMinimum;
	type ChildBountyManager = ChildBounties;
	type DataDepositPerByte = DataDepositPerByte;
	type RuntimeEvent = RuntimeEvent;
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

/// TODO: finish local payments

pub struct LocalPayments<A, F>(core::marker::PhantomData<(A, F)>);
impl<A: Eq, F: fungibles::Mutate<A>> Pay for LocalPayments<A, F> {
	type Balance = F::Balance;
	type Beneficiary = VersionedLocation;
	type AssetKind = VersionedAssetId;
	// TODO: query id - use QueryId
	type Id = ();
	type Error = DispatchError;

	fn pay(
		who: &Self::Beneficiary,
		asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		// TODO: F::transfer() - Preservation::Expendable,
		todo!()
	}

	fn check_payment(id: Self::Id) -> PaymentStatus {
		todo!()
	}
}
