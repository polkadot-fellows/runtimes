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

//! The Polkadot Technical Fellowship.

mod origins;
mod tracks;
use crate::{
	impls::ToParentTreasury, weights, xcm_config::TreasurerBodyId, AccountId, AssetRate, Balance,
	Balances, BlockNumber, FellowshipReferenda, GovernanceLocation, PolkadotTreasuryAccount,
	Preimage, Runtime, RuntimeCall, RuntimeEvent, RuntimeOrigin, Scheduler, DAYS,
};
use cumulus_primitives_core::Junction::GeneralIndex;
use frame_support::{
	parameter_types,
	traits::{EitherOf, EitherOfDiverse, MapSuccess, OriginTrait, TryWithMorphedArg},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess};
pub use origins::{
	pallet_origins as pallet_fellowship_origins, Architects, EnsureCanPromoteTo, EnsureCanRetainAt,
	EnsureFellowship, Fellows, Masters, Members, ToVoice,
};
use pallet_ranked_collective::EnsureOfRank;
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use parachains_common::polkadot::account;
use polkadot_runtime_common::impls::{
	LocatableAssetConverter, VersionedLocatableAsset, VersionedMultiLocationConverter,
};
use polkadot_runtime_constants::{
	currency::GRAND, time::HOURS, xcm::body::FELLOWSHIP_ADMIN_INDEX, DOLLARS,
};
use sp_arithmetic::Permill;
use sp_core::{ConstU128, ConstU32};
use sp_runtime::traits::{
	AccountIdConversion, ConstU16, ConvertToValue, IdentityLookup, Replace, TakeFirst,
};
use xcm::latest::BodyId;
use xcm_builder::{AliasesIntoAccountId32, LocatableAssetId, PayOverXcm};

#[cfg(feature = "runtime-benchmarks")]
use crate::impls::benchmarks::{OpenHrmpChannel, PayWithEnsure};

/// The Fellowship members' ranks.
pub mod ranks {
	use pallet_ranked_collective::Rank;

	pub const DAN_1: Rank = 1; // aka Members.
	pub const DAN_2: Rank = 2;
	pub const DAN_3: Rank = 3; // aka Fellows.
	pub const DAN_4: Rank = 4; // aka Architects.
	pub const DAN_5: Rank = 5;
	pub const DAN_6: Rank = 6;
	pub const DAN_7: Rank = 7; // aka Masters.
	pub const DAN_8: Rank = 8;
	pub const DAN_9: Rank = 9;
}

parameter_types! {
	// Referenda pallet account, used to temporarily deposit slashed imbalance before teleporting.
	pub ReferendaPalletAccount: AccountId = account::REFERENDA_PALLET_ID.into_account_truncating();
	pub const FellowshipAdminBodyId: BodyId = BodyId::Index(FELLOWSHIP_ADMIN_INDEX);
}

impl pallet_fellowship_origins::Config for Runtime {}

pub type FellowshipReferendaInstance = pallet_referenda::Instance1;

impl pallet_referenda::Config<FellowshipReferendaInstance> for Runtime {
	type WeightInfo = weights::pallet_referenda::WeightInfo<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	// Fellows can submit proposals.
	type SubmitOrigin = EitherOf<
		pallet_ranked_collective::EnsureMember<Runtime, FellowshipCollectiveInstance, 3>,
		MapSuccess<
			TryWithMorphedArg<
				RuntimeOrigin,
				<RuntimeOrigin as OriginTrait>::PalletsOrigin,
				ToVoice,
				EnsureOfRank<Runtime, FellowshipCollectiveInstance>,
				(AccountId, u16),
			>,
			TakeFirst,
		>,
	>;
	type CancelOrigin = Architects;
	type KillOrigin = Masters;
	type Slash = ToParentTreasury<PolkadotTreasuryAccount, ReferendaPalletAccount, Runtime>;
	type Votes = pallet_ranked_collective::Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime, FellowshipCollectiveInstance>;
	type SubmissionDeposit = ConstU128<0>;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = ConstU32<{ 7 * DAYS }>;
	type AlarmInterval = ConstU32<1>;
	type Tracks = tracks::TracksInfo;
	type Preimages = Preimage;
}

pub type FellowshipCollectiveInstance = pallet_ranked_collective::Instance1;

impl pallet_ranked_collective::Config<FellowshipCollectiveInstance> for Runtime {
	type WeightInfo = weights::pallet_ranked_collective::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "runtime-benchmarks"))]
	// Promotions and the induction of new members are serviced by `FellowshipCore` pallet instance.
	type PromoteOrigin = frame_system::EnsureNever<pallet_ranked_collective::Rank>;
	#[cfg(feature = "runtime-benchmarks")]
	// The maximum value of `u16` set as a success value for the root to ensure the benchmarks will
	// pass.
	type PromoteOrigin = EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>;

	// Demotion is by any of:
	// - Root can demote arbitrarily.
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	//
	// The maximum value of `u16` set as a success value for the root to ensure the benchmarks will
	// pass.
	type DemoteOrigin = EitherOf<
		EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>,
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::DAN_9 }>>,
		>,
	>;
	type Polls = FellowshipReferenda;
	type MinRankOfClass = tracks::MinRankOfClass;
	type VoteWeight = pallet_ranked_collective::Geometric;
}

pub type FellowshipCoreInstance = pallet_core_fellowship::Instance1;

impl pallet_core_fellowship::Config<FellowshipCoreInstance> for Runtime {
	type WeightInfo = weights::pallet_core_fellowship::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Members = pallet_ranked_collective::Pallet<Runtime, FellowshipCollectiveInstance>;
	type Balance = Balance;
	// Parameters are set by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Fellows.
	type ParamsOrigin = EitherOfDiverse<
		EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
		Fellows,
	>;
	// Induction (creating a candidate) is by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a single Fellow;
	// - a vote among all Members.
	type InductOrigin = EitherOfDiverse<
		EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
		EitherOfDiverse<
			pallet_ranked_collective::EnsureMember<
				Runtime,
				FellowshipCollectiveInstance,
				{ ranks::DAN_3 },
			>,
			Members,
		>,
	>;
	// Approval (rank-retention) of a Member's current rank is by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote by the rank two above the current rank for all retention up to the Master rank.
	type ApproveOrigin = EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::DAN_9 }>>,
		>,
		EnsureCanRetainAt,
	>;
	// Promotion is by any of:
	// - Root can promote arbitrarily.
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote by the rank two above the new rank for all promotions up to the Master rank.
	type PromoteOrigin = EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::DAN_9 }>>,
		>,
		EnsureCanPromoteTo,
	>;
	type EvidenceSize = ConstU32<65536>;
}

pub type FellowshipSalaryInstance = pallet_salary::Instance1;

use xcm::prelude::*;

parameter_types! {
	pub AssetHub: MultiLocation = (Parent, Parachain(1000)).into();
	pub AssetHubUsdtId: AssetId = (PalletInstance(50), GeneralIndex(1984)).into();
	pub UsdtAsset: LocatableAssetId = LocatableAssetId {
		location: AssetHub::get(),
		asset_id: AssetHubUsdtId::get(),
	};
	// The interior location on AssetHub for the paying account. This is the Fellowship Salary
	// pallet instance (which sits at index 64). This sovereign account will need funding.
	pub Interior: InteriorMultiLocation = PalletInstance(64).into();
}

const USDT_UNITS: u128 = 1_000_000;

/// [`PayOverXcm`] setup to pay the Fellowship salary on the AssetHub in USDT.
pub type FellowshipSalaryPaymaster = PayOverXcm<
	Interior,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	AccountId,
	(),
	ConvertToValue<UsdtAsset>,
	AliasesIntoAccountId32<(), AccountId>,
>;

impl pallet_salary::Config<FellowshipSalaryInstance> for Runtime {
	type WeightInfo = weights::pallet_salary::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = FellowshipSalaryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = PayWithEnsure<FellowshipSalaryPaymaster, OpenHrmpChannel<ConstU32<1000>>>;
	type Members = pallet_ranked_collective::Pallet<Runtime, FellowshipCollectiveInstance>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Salary = pallet_core_fellowship::Pallet<Runtime, FellowshipCoreInstance>;
	#[cfg(feature = "runtime-benchmarks")]
	type Salary = frame_support::traits::tokens::ConvertRank<
		crate::impls::benchmarks::RankToSalary<Balances>,
	>;
	// 15 days to register for a salary payment.
	type RegistrationPeriod = ConstU32<{ 15 * DAYS }>;
	// 15 days to claim the salary payment.
	type PayoutPeriod = ConstU32<{ 15 * DAYS }>;
	// Total monthly salary budget.
	type Budget = ConstU128<{ 100_000 * USDT_UNITS }>;
}

parameter_types! {
	// TODO: reference the constant value from common crate with polkadot-sdk 1.5
	pub const FellowshipTreasuryPalletId: PalletId = PalletId(*b"py/feltr");
	pub const ProposalBond: Permill = Permill::from_percent(1);
	pub const ProposalBondMinimum: Balance = 5 * DOLLARS;
	pub const ProposalBondMaximum: Balance = 10 * DOLLARS;
	pub const SpendPeriod: BlockNumber = 7 * DAYS;
	pub const Burn: Permill = Permill::from_percent(0);
	pub const PayoutSpendPeriod: BlockNumber = 30 * DAYS;
	// The asset's interior location for the paying account. This is the Fellowship Treasury
	// pallet instance (which sits at index 65).
	pub FellowshipTreasuryInteriorLocation: InteriorMultiLocation = PalletInstance(65).into();
}

/// [`PayOverXcm`] setup to pay the Fellowship Treasury.
pub type FellowshipTreasuryPaymaster = PayOverXcm<
	FellowshipTreasuryInteriorLocation,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	VersionedMultiLocation,
	VersionedLocatableAsset,
	LocatableAssetConverter,
	VersionedMultiLocationConverter,
>;

pub type FellowshipTreasuryInstance = pallet_treasury::Instance1;

impl pallet_treasury::Config<FellowshipTreasuryInstance> for Runtime {
	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type PalletId = FellowshipTreasuryPalletId;
	type Currency = Balances;
	// This parameter guards a deprecated call and should not be used.
	// TODO: replace by NeverEnsure with polkadot-sdk 1.5
	type ApproveOrigin = EnsureRoot<AccountId>;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		EitherOfDiverse<EnsureXcm<IsVoiceOfBody<GovernanceLocation, TreasurerBodyId>>, Fellows>,
	>;
	type RuntimeEvent = RuntimeEvent;
	// This type should never be triggered since it meant for deprecated functionality.
	type OnSlash = ();
	type ProposalBond = ProposalBond;
	type ProposalBondMinimum = ProposalBondMinimum;
	type ProposalBondMaximum = ProposalBondMaximum;
	type SpendPeriod = SpendPeriod;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = EitherOf<
		EitherOf<
			EnsureRootWithSuccess<AccountId, ConstU128<{ 10_000 * GRAND }>>,
			MapSuccess<
				EnsureXcm<IsVoiceOfBody<GovernanceLocation, TreasurerBodyId>>,
				Replace<ConstU128<{ 10_000 * GRAND }>>,
			>,
		>,
		EitherOf<
			MapSuccess<Architects, Replace<ConstU128<{ 10_000 * GRAND }>>>,
			MapSuccess<Fellows, Replace<ConstU128<{ 10 * GRAND }>>>,
		>,
	>;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedMultiLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = FellowshipTreasuryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = PayWithEnsure<FellowshipTreasuryPaymaster, OpenHrmpChannel<ConstU32<1000>>>;
	type BalanceConverter = AssetRate;
	type PayoutPeriod = PayoutSpendPeriod;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = benchmarks::TreasuryArguments<sp_core::ConstU8<1>, ConstU32<1000>>;
}

// TODO: replace by [`polkadot_runtime_common::impls::benchmarks::TreasuryArguments`] with
// polkadot-sdk 1.5
#[cfg(feature = "runtime-benchmarks")]
mod benchmarks {
	use super::VersionedLocatableAsset;
	use core::marker::PhantomData;
	use frame_support::traits::Get;
	use pallet_treasury::ArgumentsFactory as TreasuryArgumentsFactory;
	use sp_core::{ConstU32, ConstU8};
	use xcm::prelude::*;

	/// Provide factory methods for the [`VersionedLocatableAsset`] and the `Beneficiary` of the
	/// [`VersionedMultiLocation`]. The location of the asset is determined as a Parachain with an
	/// ID equal to the passed seed.
	pub struct TreasuryArguments<Parents = ConstU8<0>, ParaId = ConstU32<0>>(
		PhantomData<(Parents, ParaId)>,
	);
	impl<Parents: Get<u8>, ParaId: Get<u32>>
		TreasuryArgumentsFactory<VersionedLocatableAsset, VersionedMultiLocation>
		for TreasuryArguments<Parents, ParaId>
	{
		fn create_asset_kind(seed: u32) -> VersionedLocatableAsset {
			VersionedLocatableAsset::V3 {
				location: xcm::v3::MultiLocation::new(Parents::get(), X1(Parachain(ParaId::get()))),
				asset_id: xcm::v3::MultiLocation::new(
					0,
					X2(PalletInstance(seed.try_into().unwrap()), GeneralIndex(seed.into())),
				)
				.into(),
			}
		}
		fn create_beneficiary(seed: [u8; 32]) -> VersionedMultiLocation {
			VersionedMultiLocation::V3(xcm::v3::MultiLocation::new(
				0,
				X1(AccountId32 { network: None, id: seed }),
			))
		}
	}
}
