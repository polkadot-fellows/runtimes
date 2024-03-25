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
	impls::ToParentTreasury,
	weights,
	xcm_config::{LocationToAccountId, TreasurerBodyId},
	AccountId, AssetRate, Balance, Balances, FellowshipReferenda, GovernanceLocation,
	ParachainInfo, PolkadotTreasuryAccount, Preimage, Runtime, RuntimeCall, RuntimeEvent,
	RuntimeOrigin, Scheduler, DAYS, FELLOWSHIP_TREASURY_PALLET_ID,
};
use cumulus_primitives_core::Junction::GeneralIndex;
use frame_support::{
	parameter_types,
	traits::{
		EitherOf, EitherOfDiverse, MapSuccess, OriginTrait, PalletInfoAccess, TryWithMorphedArg,
	},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess};
pub use origins::{
	pallet_origins as pallet_fellowship_origins, Architects, EnsureCanPromoteTo, EnsureCanRetainAt,
	EnsureFellowship, Fellows, Masters, Members, ToVoice,
};
use pallet_ranked_collective::EnsureOfRank;
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use polkadot_runtime_common::impls::{
	LocatableAssetConverter, VersionedLocatableAsset, VersionedLocationConverter,
};
use polkadot_runtime_constants::{
	currency::GRAND, system_parachain, time::HOURS, xcm::body::FELLOWSHIP_ADMIN_INDEX,
};
use sp_arithmetic::Permill;
use sp_core::{ConstU128, ConstU32};
use sp_runtime::traits::{ConstU16, ConvertToValue, IdentityLookup, Replace, TakeFirst};
use xcm::latest::BodyId;
use xcm_builder::{AliasesIntoAccountId32, LocatableAssetId, PayOverXcm};

#[cfg(feature = "runtime-benchmarks")]
use crate::{
	impls::benchmarks::{OpenHrmpChannel, PayWithEnsure},
	ExistentialDeposit,
};

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
	type Slash = ToParentTreasury<PolkadotTreasuryAccount, LocationToAccountId, Runtime>;
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
	// Exchange is by any of:
	// - Root can exchange arbitrarily.
	// - the Fellows origin
	type ExchangeOrigin =
		EitherOf<frame_system::EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>, Fellows>;
	type Polls = FellowshipReferenda;
	type MinRankOfClass = tracks::MinRankOfClass;
	type MemberSwappedHandler = (crate::FellowshipCore, crate::FellowshipSalary);
	type VoteWeight = pallet_ranked_collective::Geometric;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = (crate::FellowshipCore, crate::FellowshipSalary);
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
	pub AssetHub: Location = (Parent, Parachain(system_parachain::ASSET_HUB_ID)).into();
	pub AssetHubUsdtId: AssetId = (PalletInstance(50), GeneralIndex(1984)).into();
	pub UsdtAsset: LocatableAssetId = LocatableAssetId {
		location: AssetHub::get(),
		asset_id: AssetHubUsdtId::get(),
	};
	// The interior location on AssetHub for the paying account. This is the Fellowship Salary
	// pallet instance. This sovereign account will need funding.
	pub Interior: InteriorLocation = PalletInstance(<crate::FellowshipSalary as PalletInfoAccess>::index() as u8).into();
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
	type Budget = ConstU128<{ 250_000 * USDT_UNITS }>;
}

parameter_types! {
	pub const FellowshipTreasuryPalletId: PalletId = FELLOWSHIP_TREASURY_PALLET_ID;
	pub const ProposalBond: Permill = Permill::from_percent(100);
	pub const Burn: Permill = Permill::from_percent(0);
	pub const MaxBalance: Balance = Balance::max_value();
	// The asset's interior location for the paying account. This is the Fellowship Treasury
	// pallet instance.
	pub FellowshipTreasuryInteriorLocation: InteriorLocation =
		PalletInstance(<crate::FellowshipTreasury as PalletInfoAccess>::index() as u8).into();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub const ProposalBondForBenchmark: Permill = Permill::from_percent(5);
}

/// [`PayOverXcm`] setup to pay the Fellowship Treasury.
pub type FellowshipTreasuryPaymaster = PayOverXcm<
	FellowshipTreasuryInteriorLocation,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	VersionedLocation,
	VersionedLocatableAsset,
	LocatableAssetConverter,
	VersionedLocationConverter,
>;

pub type FellowshipTreasuryInstance = pallet_treasury::Instance1;

impl pallet_treasury::Config<FellowshipTreasuryInstance> for Runtime {
	// The creation of proposals via the treasury pallet is deprecated and should not be utilized.
	// Instead, public or fellowship referenda should be used to propose and command the treasury
	// spend or spend_local dispatchables. The parameters below have been configured accordingly to
	// discourage its use.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ApproveOrigin = frame_support::traits::NeverEnsureOrigin<Balance>;
	#[cfg(feature = "runtime-benchmarks")]
	type ApproveOrigin = EnsureRoot<AccountId>;
	type OnSlash = ();
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ProposalBond = ProposalBond;
	#[cfg(feature = "runtime-benchmarks")]
	type ProposalBond = ProposalBondForBenchmark;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ProposalBondMinimum = MaxBalance;
	#[cfg(feature = "runtime-benchmarks")]
	type ProposalBondMinimum = ConstU128<{ ExistentialDeposit::get() * 100 }>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type ProposalBondMaximum = MaxBalance;
	#[cfg(feature = "runtime-benchmarks")]
	type ProposalBondMaximum = ConstU128<{ ExistentialDeposit::get() * 500 }>;
	// end.

	type WeightInfo = weights::pallet_treasury::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type PalletId = FellowshipTreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		EitherOfDiverse<EnsureXcm<IsVoiceOfBody<GovernanceLocation, TreasurerBodyId>>, Fellows>,
	>;
	type SpendPeriod = ConstU32<{ 7 * DAYS }>;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = EitherOf<
		EitherOf<
			EnsureRootWithSuccess<AccountId, MaxBalance>,
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
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = FellowshipTreasuryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = PayWithEnsure<FellowshipTreasuryPaymaster, OpenHrmpChannel<ConstU32<1000>>>;
	type BalanceConverter = crate::impls::NativeOnSiblingParachain<AssetRate, ParachainInfo>;
	type PayoutPeriod = ConstU32<{ 30 * DAYS }>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::TreasuryArguments<
		sp_core::ConstU8<1>,
		ConstU32<1000>,
	>;
}
