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

//! The Polkadot Tooling Collective.

mod origins;
mod tracks;

use frame_support::traits::NeverEnsureOrigin;
use crate::{
	impls::ToParentTreasury,
	weights,
	xcm_config::{AssetHubUsdt, LocationToAccountId, TreasurerBodyId},
	AccountId, AssetRateWithNative, Balance, Balances, GovernanceLocation, PolkadotTreasuryAccount,
	PotocReferenda, Preimage, Runtime, RuntimeCall, RuntimeEvent, Scheduler, DAYS,
	POTOC_TREASURY_PALLET_ID, *,
};
// There is only one admin for all collectives:
use crate::xcm_config::FellowshipAdminBodyId as PotocAdminBodyId;
use frame_support::{
	parameter_types,
	traits::{
		EitherOf, EitherOfDiverse, MapSuccess, PalletInfoAccess,
	},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess};
pub use origins::{
	pallet_origins as pallet_potoc_origins, Members,
};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use polkadot_runtime_common::impls::{
	LocatableAssetConverter, VersionedLocatableAsset, VersionedLocationConverter,
};
use polkadot_runtime_constants::{currency::GRAND, time::HOURS};
use sp_arithmetic::Permill;
use sp_core::{ConstU128, ConstU32};
use sp_runtime::traits::{
	ConstU16, ConvertToValue, IdentityLookup, Replace, ReplaceWithDefault,
};
use xcm_builder::{AliasesIntoAccountId32, PayOverXcm};

#[cfg(feature = "runtime-benchmarks")]
use crate::impls::benchmarks::{OpenHrmpChannel, PayWithEnsure};

/// PoToC's ranks.
pub mod ranks {
	use pallet_ranked_collective::Rank;

	/// A Candidate.
	pub const CANDIDATE: Rank = 0;
	/// A Member.
	pub const MEMBER: Rank = 1;
}

/// Origin of either Member vote, OpenGov or Root.
pub type OpenGovOrMembers = EitherOfDiverse<
	Members,
	EitherOfDiverse<
		EnsureXcm<IsVoiceOfBody<GovernanceLocation, PotocAdminBodyId>>,
		EnsureRoot<AccountId>,
	>,
>;

/// Promote origin, either:
/// - Root
/// - PoToC's Admin origin (i.e. token holder referendum)
/// - Members vote
pub type PromoteOrigin = MapSuccess<OpenGovOrMembers, Replace<ConstU16<1>>>;

impl pallet_potoc_origins::Config for Runtime {}

pub type PotocReferendaInstance = pallet_referenda::Instance3;
impl pallet_referenda::Config<PotocReferendaInstance> for Runtime {
	type WeightInfo = weights::pallet_referenda_fellowship_referenda::WeightInfo<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	// Members can submit proposals, candidates cannot.
	type SubmitOrigin =
		pallet_ranked_collective::EnsureMember<Runtime, PotocCollectiveInstance, { ranks::MEMBER }>;
	type CancelOrigin = OpenGovOrMembers;
	type KillOrigin = OpenGovOrMembers;
	type Slash = ToParentTreasury<PolkadotTreasuryAccount, LocationToAccountId, Runtime>;
	type Votes = pallet_ranked_collective::Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime, PotocCollectiveInstance>;
	type SubmissionDeposit = ConstU128<0>;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = ConstU32<{ 7 * DAYS }>;
	type AlarmInterval = ConstU32<1>;
	type Tracks = tracks::TracksInfo;
	type Preimages = Preimage;
}

pub type PotocCollectiveInstance = pallet_ranked_collective::Instance3;
impl pallet_ranked_collective::Config<PotocCollectiveInstance> for Runtime {
	type WeightInfo = weights::pallet_ranked_collective_fellowship_collective::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	// Promotions and the induction of new members are serviced by `PotocCore` pallet instance.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type PromoteOrigin = frame_system::EnsureNever<Rank>;
	// The maximum value of `u16` set as a success value for the root to ensure the benchmarks will
	// pass.
	#[cfg(feature = "runtime-benchmarks")]
	type PromoteOrigin = EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>;

	// Demotion is by any of:
	// - Root can demote arbitrarily.
	// - PoToC's Admin origin (i.e. token holder referendum);
	type DemoteOrigin = EitherOf<
		EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>,
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, PotocAdminBodyId>>,
			Replace<ConstU16<65535>>,
		>,
	>;
	// Exchange is by any of:
	// - Root can exchange arbitrarily.
	// - the Members origin
	type ExchangeOrigin =
		EitherOf<frame_system::EnsureRoot<Self::AccountId>, Members>;
	type AddOrigin = MapSuccess<Self::PromoteOrigin, ReplaceWithDefault<()>>;
	type RemoveOrigin = Self::DemoteOrigin;
	type Polls = PotocReferenda;
	// Map ranks 1:1 to the tracks that they can vote on.
	type MinRankOfClass = sp_runtime::traits::Identity;
	type MemberSwappedHandler = (crate::PotocCore, crate::PotocSalary);
	type VoteWeight = pallet_ranked_collective::Geometric;
	type MaxMemberCount = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = (crate::PotocCore, crate::PotocSalary);
}

pub type PotocCoreInstance = pallet_core_fellowship::Instance3;

impl pallet_core_fellowship::Config<PotocCoreInstance> for Runtime {
	type WeightInfo = weights::pallet_core_fellowship_fellowship_core::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Members = pallet_ranked_collective::Pallet<Runtime, PotocCollectiveInstance>;
	type Balance = Balance;
	type ParamsOrigin = OpenGovOrMembers;
	type InductOrigin = OpenGovOrMembers;
	type PromoteOrigin = PromoteOrigin;
	type ApproveOrigin = Self::PromoteOrigin;
	// Fast promotions are not needed with a single rank and would require higher turnout.
	type FastPromoteOrigin = NeverEnsureOrigin<u16>;
	type EvidenceSize = ConstU32<65536>;
	type MaxRank = ConstU32<9>;
}

pub type PotocSalaryInstance = pallet_salary::Instance3;

use xcm::prelude::*;

parameter_types! {
	// The interior location on AssetHub for the paying account. This is PoToC's Salary
	// pallet instance. This sovereign account will need funding.
	pub Interior: InteriorLocation = PalletInstance(<crate::PotocSalary as PalletInfoAccess>::index() as u8).into();
}

const USDT_UNITS: u128 = 1_000_000;

/// [`PayOverXcm`] setup to pay PoToC's salary on the AssetHub in USDT.
pub type PotocSalaryPaymaster = PayOverXcm<
	Interior,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	AccountId,
	(),
	ConvertToValue<AssetHubUsdt>,
	AliasesIntoAccountId32<(), AccountId>,
>;

impl pallet_salary::Config<PotocSalaryInstance> for Runtime {
	type WeightInfo = weights::pallet_salary_fellowship_salary::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = PotocSalaryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = PayWithEnsure<PotocSalaryPaymaster, OpenHrmpChannel<ConstU32<1000>>>;
	type Members = pallet_ranked_collective::Pallet<Runtime, PotocCollectiveInstance>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Salary = pallet_core_fellowship::Pallet<Runtime, PotocCoreInstance>;
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
	pub const PotocTreasuryPalletId: PalletId = POTOC_TREASURY_PALLET_ID;
	pub const ProposalBond: Permill = Permill::from_percent(100);
	pub const Burn: Permill = Permill::from_percent(0);
	pub const MaxBalance: Balance = Balance::MAX;
	// The asset's interior location for the paying account. This is PoToC's Treasury
	// pallet instance.
	pub PotocTreasuryInteriorLocation: InteriorLocation =
		PalletInstance(<crate::PotocTreasury as PalletInfoAccess>::index() as u8).into();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub const ProposalBondForBenchmark: Permill = Permill::from_percent(5);
}

/// [`PayOverXcm`] setup to pay PoToC's Treasury.
pub type PotocTreasuryPaymaster = PayOverXcm<
	PotocTreasuryInteriorLocation,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	VersionedLocation,
	VersionedLocatableAsset,
	LocatableAssetConverter,
	VersionedLocationConverter,
>;

pub type PotocTreasuryInstance = pallet_treasury::Instance3;

impl pallet_treasury::Config<PotocTreasuryInstance> for Runtime {
	type WeightInfo = weights::pallet_treasury_fellowship_treasury::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type PalletId = PotocTreasuryPalletId;
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
		MapSuccess<Members, Replace<ConstU128<{ 10 * GRAND }>>>,
	>;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = PotocTreasuryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = PayWithEnsure<PotocTreasuryPaymaster, OpenHrmpChannel<ConstU32<1000>>>;
	type BalanceConverter = AssetRateWithNative;
	type PayoutPeriod = ConstU32<{ 30 * DAYS }>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::TreasuryArguments<
		sp_core::ConstU8<1>,
		ConstU32<1000>,
	>;
}
