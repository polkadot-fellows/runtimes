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

//! The Polkadot Secretary Collective.

mod origins;
mod tracks;
use crate::{
	fellowship::{pallet_fellowship_origins::Fellows, FellowshipAdminBodyId},
	impls::ToParentTreasury,
	weights,
	xcm_config::{LocationToAccountId, TreasurerBodyId},
	AccountId, AssetRate, Balance, Balances, GovernanceLocation, ParachainInfo,
	PolkadotTreasuryAccount, Preimage, Runtime, RuntimeCall, RuntimeEvent, Scheduler,
	SecretaryReferenda, DAYS,
};
use frame_support::{
	parameter_types,
	traits::{tokens::GetSalary, EitherOf, EitherOfDiverse, MapSuccess, PalletInfoAccess},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureRootWithSuccess};
pub use origins::{pallet_origins as pallet_secretary_origins, Secretary};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use polkadot_runtime_common::impls::{
	LocatableAssetConverter, VersionedLocatableAsset, VersionedLocationConverter,
};
use polkadot_runtime_constants::{currency::GRAND, system_parachain, time::HOURS};
use sp_core::{ConstU128, ConstU32};
use sp_runtime::{
	traits::{ConstU16, ConvertToValue, Identity, IdentityLookup, Replace},
	Permill,
};
use system_parachains_constants::polkadot::account::SECRETARY_TREASURY_PALLET_ID;

/// The Secretary members' ranks.
pub mod ranks {
	use pallet_ranked_collective::Rank;

	#[allow(dead_code)]
	pub const SECRETARY_CANDIDATE: Rank = 0;
	pub const SECRETARY: Rank = 1;
}

type ParamsOrigin =
	EitherOfDiverse<EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>, Fellows>;

type ApproveOrigin = EitherOf<
	MapSuccess<
		EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
		Replace<ConstU16<{ ranks::SECRETARY }>>,
	>,
	Fellows,
>;

type OpenGovOrSecretary = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EitherOfDiverse<Secretary, EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>>,
>;

impl pallet_secretary_origins::Config for Runtime {}

pub type SecretaryReferendaInstance = pallet_referenda::Instance3;

pub type SecretaryCollectiveInstance = pallet_ranked_collective::Instance3;

impl pallet_referenda::Config<SecretaryReferendaInstance> for Runtime {
	type WeightInfo = weights::pallet_referenda::WeightInfo<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	// Secretary collective can submit proposals.
	type SubmitOrigin = pallet_ranked_collective::EnsureMember<
		Runtime,
		SecretaryCollectiveInstance,
		{ ranks::SECRETARY },
	>;
	// Referandum can be cancled by any of:
	// - Root;
	// - the FellowshipAdmiin origin(i.e token holder referendum)
	// - a vote by a member of the Secretary collective;
	type CancelOrigin = OpenGovOrSecretary;
	// Referandum can be killed by any of:
	// - Root;
	// the FellowshipAdmin oriigin (i.e. token holder referandum);
	// - a vote by a member of the Secretary collective;
	type KillOrigin = OpenGovOrSecretary;
	type Slash = ToParentTreasury<PolkadotTreasuryAccount, LocationToAccountId, Runtime>;
	type Votes = pallet_ranked_collective::Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime, SecretaryCollectiveInstance>;
	type SubmissionDeposit = ConstU128<0>;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = ConstU32<{ 7 * DAYS }>;
	type AlarmInterval = ConstU32<1>;
	type Tracks = tracks::TracksInfo;
	type Preimages = Preimage;
}

impl pallet_ranked_collective::Config<SecretaryCollectiveInstance> for Runtime {
	type WeightInfo = weights::pallet_ranked_collective::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "runtime-benchmarks"))]
	// Promotions and inductions should be done through the [`crate::SecretaryCore`] pallet instance instance.
	type PromoteOrigin = frame_system::EnsureNever<pallet_ranked_collective::Rank>;
	#[cfg(feature = "runtime-benchmarks")]
	// The maximum value of `u16` set as a success value for the root to ensure the benchmarks will
	// pass.
	type PromoteOrigin = EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>;
	// Demotion is by any of:
	// - Root can demote arbitrarily.
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	type DemoteOrigin = EitherOf<
		EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>,
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::SECRETARY }>>,
		>,
	>;
	// Exchange is by any of:
	// - Root can exchange arbitrarily.
	// - the Fellows origin
	type ExchangeOrigin =
		EitherOf<frame_system::EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>, Fellows>;
	type Polls = SecretaryReferenda;
	type MinRankOfClass = Identity;
	type MemberSwappedHandler = (crate::SecretaryCore, crate::SecretarySalary);
	type VoteWeight = pallet_ranked_collective::Geometric;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = (crate::SecretaryCore, crate::SecretarySalary);
}

pub type SecretaryCoreInstance = pallet_core_fellowship::Instance3;

impl pallet_core_fellowship::Config<SecretaryCoreInstance> for Runtime {
	type WeightInfo = weights::pallet_core_fellowship::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Members = pallet_ranked_collective::Pallet<Runtime, SecretaryCollectiveInstance>;
	type Balance = Balance;
	// Parameters are set by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a Fellow;
	type ParamsOrigin = ParamsOrigin;
	// Induction (creating a candidate) is by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a Fellow;
	type InductOrigin = ParamsOrigin;
	// Approval (rank-retention) of a Member's current rank is by any of:
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a Fellow;
	type ApproveOrigin = ApproveOrigin;
	// Promotion is by any of:
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a Fellow;
	type PromoteOrigin = ApproveOrigin;

	type EvidenceSize = ConstU32<65536>;
}

pub type SecretarySalaryInstance = pallet_salary::Instance3;

use xcm::prelude::*;
use xcm_builder::{AliasesIntoAccountId32, LocatableAssetId, PayOverXcm};

parameter_types! {
	pub AssetHub: Location = (Parent, Parachain(system_parachain::ASSET_HUB_ID)).into();
	pub AssetHubUsdtId: AssetId = (PalletInstance(50), GeneralIndex(1984)).into();
	pub UsdtAsset: LocatableAssetId = LocatableAssetId {
		location: AssetHub::get(),
		asset_id: AssetHubUsdtId::get(),
	};
	// The interior location on AssetHub for the paying account. This is the Secretary Salary
	// pallet instance. This sovereign account will need funding.
	pub Interior: InteriorLocation = PalletInstance(<crate::SecretarySalary as PalletInfoAccess>::index() as u8).into();
}

const USDT_UNITS: u128 = 1_000_000;

/// [`PayOverXcm`] setup to pay the Secretary salary on the AssetHub in USDT.
pub type SecretarySalaryPaymaster = PayOverXcm<
	Interior,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	AccountId,
	(),
	ConvertToValue<UsdtAsset>,
	AliasesIntoAccountId32<(), AccountId>,
>;

pub struct SalaryForRank;
impl GetSalary<u16, AccountId, Balance> for SalaryForRank {
	fn get_salary(a: u16, _: &AccountId) -> Balance {
		Balance::from(a) * 1000 * USDT_UNITS
	}
}

impl pallet_salary::Config<SecretarySalaryInstance> for Runtime {
	type WeightInfo = weights::pallet_salary::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = SecretarySalaryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = PayWithEnsure<FellowshipSalaryPaymaster, OpenHrmpChannel<ConstU32<1000>>>;
	type Members = pallet_ranked_collective::Pallet<Runtime, SecretaryCollectiveInstance>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Salary = SalaryForRank;
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
	pub const SecretaryTreasuryPalletId: PalletId = SECRETARY_TREASURY_PALLET_ID;
	pub const ProposalBond: Permill = Permill::from_percent(100);
	pub const Burn: Permill = Permill::from_percent(0);
	pub const MaxBalance: Balance = Balance::max_value();
	// The asset's interior location for the paying account. This is the Secretary Treasury
	// pallet instance.
	pub SecretaryTreasuryInteriorLocation: InteriorLocation =
		PalletInstance(<crate::SecretaryTreasury as PalletInfoAccess>::index() as u8).into();
}

/// [`PayOverXcm`] setup to pay the Fellowship Treasury.
pub type SecretaryTreasuryPaymaster = PayOverXcm<
	Interior,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	VersionedLocation,
	VersionedLocatableAsset,
	LocatableAssetConverter,
	VersionedLocationConverter,
>;

pub type SecretaryTreasuryInstance = pallet_treasury::Instance3;

impl pallet_treasury::Config<SecretaryTreasuryInstance> for Runtime {
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
	type PalletId = SecretaryTreasuryPalletId;
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
		EnsureRootWithSuccess<AccountId, MaxBalance>,
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, TreasurerBodyId>>,
			Replace<ConstU128<{ 10_000 * GRAND }>>,
		>,
	>;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = SecretaryTreasuryPaymaster;
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
