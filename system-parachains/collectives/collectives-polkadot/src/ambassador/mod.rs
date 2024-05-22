// Copyright (C) 2022 Parity Technologies (UK) Ltd.
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

//! The Polkadot Ambassador Program.
//!
//! The module defines the following on-chain functionality of the Ambassador Program:
//!
//! - Managed set of program members, where every member has a [rank](ranks)
//! (via [AmbassadorCollective](pallet_ranked_collective)).
//! - Referendum functionality for the program members to propose, vote on, and execute
//! proposals on behalf of the members of a certain [rank](Origin)
//! (via [AmbassadorReferenda](pallet_referenda)).
//! - Promotion and demotion periods, register of members' activity, and rank based salaries
//! (via [AmbassadorCore](pallet_core_fellowship)).
//! - Members' salaries (via [AmbassadorSalary](pallet_salary), requiring a member to be
//! imported or inducted into [AmbassadorCore](pallet_core_fellowship)).
//! - Ambassador Program Sub-Treasury (via [AmbassadorTreasury](pallet_treasury)).

pub mod origins;
mod tracks;

pub use origins::pallet_origins as pallet_ambassador_origins;

use crate::{
	xcm_config::{AssetHubUsdt, FellowshipAdminBodyId},
	*,
};
use frame_support::{
	pallet_prelude::PalletInfoAccess,
	traits::{EitherOf, MapSuccess, TryMapSuccess},
};
use frame_system::EnsureRootWithSuccess;
use origins::pallet_origins::{EnsureAmbassadorsFrom, HeadAmbassadors, Origin, SeniorAmbassadors};
use pallet_ranked_collective::{Rank, Votes};
use polkadot_runtime_common::impls::{LocatableAssetConverter, VersionedLocationConverter};
use sp_core::ConstU128;
use sp_runtime::{
	traits::{CheckedReduceBy, Convert, ConvertToValue, IdentityLookup, Replace},
	Permill,
};
use xcm::prelude::*;
use xcm_builder::{AliasesIntoAccountId32, PayOverXcm};

/// The Ambassador Program's member ranks.
pub mod ranks {
	use super::Rank;

	#[allow(dead_code)]
	pub const CANDIDATE: Rank = 0;
	pub const AMBASSADOR: Rank = 1;
	pub const SENIOR_AMBASSADOR: Rank = 2;
	pub const HEAD_AMBASSADOR: Rank = 3;
}

impl pallet_ambassador_origins::Config for Runtime {}

/// Demotion is by any of:
/// - Root can demote arbitrarily;
/// - the FellowshipAdmin voice (i.e. token holder referendum) can demote arbitrarily;
/// - Head Ambassadors voice can demote Senior Ambassador or Ambassador;
/// - Senior Ambassadors voice can demote Ambassador.
pub type DemoteOrigin = EitherOf<
	EnsureRootWithSuccess<AccountId, ConstU16<65535>>,
	EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::HEAD_AMBASSADOR }>>,
		>,
		TryMapSuccess<
			EnsureAmbassadorsFrom<ConstU16<{ ranks::SENIOR_AMBASSADOR }>>,
			CheckedReduceBy<ConstU16<1>>,
		>,
	>,
>;

/// Promotion and approval (rank-retention) is by any of:
/// - Root can promote arbitrarily.
/// - the FellowshipAdmin voice (i.e. token holder referendum) can promote arbitrarily.
/// - Head Ambassadors voice can promote to Senior Ambassador and Ambassador;
/// - Senior Ambassadors voice can promote to Ambassador.
pub type PromoteOrigin = DemoteOrigin;

/// Root, FellowshipAdmin or HeadAmbassadors.
pub type OpenGovOrHeadAmbassadors = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EitherOfDiverse<
		HeadAmbassadors,
		EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
	>,
>;

/// Ambassadors' vote weights for referendums.
/// - Each member with an excess rank of 0 gets 1 vote;
/// - ...with an excess rank of 1 gets 5 votes;
/// - ...with an excess rank of 2 gets 10 votes;
/// - ...with an excess rank of 3 gets 15 votes;
pub struct VoteWeight;
impl Convert<Rank, Votes> for VoteWeight {
	fn convert(excess: Rank) -> Votes {
		if excess == 0 {
			1
		} else {
			(excess * 5).into()
		}
	}
}

pub type AmbassadorCollectiveInstance = pallet_ranked_collective::Instance2;

impl pallet_ranked_collective::Config<AmbassadorCollectiveInstance> for Runtime {
	type WeightInfo = weights::pallet_ranked_collective_ambassador_collective::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	// Promotions must be done through the [`crate::AmbassadorCore`] pallet instance.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type PromoteOrigin = frame_support::traits::NeverEnsureOrigin<Rank>;
	#[cfg(feature = "runtime-benchmarks")]
	type PromoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type DemoteOrigin = DemoteOrigin;
	type Polls = AmbassadorReferenda;
	type MinRankOfClass = sp_runtime::traits::Identity;
	type VoteWeight = VoteWeight;
	type ExchangeOrigin = OpenGovOrHeadAmbassadors;
	type MemberSwappedHandler = (crate::AmbassadorCore, crate::AmbassadorSalary);
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = (crate::AmbassadorCore, crate::AmbassadorSalary);
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 0;
	pub const UndecidingTimeout: BlockNumber = 7 * DAYS;
}

pub type AmbassadorReferendaInstance = pallet_referenda::Instance2;

impl pallet_referenda::Config<AmbassadorReferendaInstance> for Runtime {
	type WeightInfo = weights::pallet_referenda_ambassador_referenda::WeightInfo<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	// Any member of the Ambassador Program can submit a proposal.
	type SubmitOrigin = pallet_ranked_collective::EnsureMember<
		Runtime,
		AmbassadorCollectiveInstance,
		{ ranks::AMBASSADOR },
	>;
	// Referendum can be canceled by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Head Ambassadors.
	type CancelOrigin = OpenGovOrHeadAmbassadors;
	// Referendum can be killed by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Head Ambassadors.
	type KillOrigin = OpenGovOrHeadAmbassadors;
	type Slash = ToParentTreasury<PolkadotTreasuryAccount, LocationToAccountId, Runtime>;
	type Votes = Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime, AmbassadorCollectiveInstance>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<20>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = tracks::TracksInfo;
	type Preimages = Preimage;
}

pub type AmbassadorCoreInstance = pallet_core_fellowship::Instance2;

impl pallet_core_fellowship::Config<AmbassadorCoreInstance> for Runtime {
	type WeightInfo = weights::pallet_core_fellowship_ambassador_core::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type Members = pallet_ranked_collective::Pallet<Runtime, AmbassadorCollectiveInstance>;
	type Balance = Balance;
	// Parameters are set by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Head Ambassadors.
	type ParamsOrigin = OpenGovOrHeadAmbassadors;
	// Induction (creating a candidate) is by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a single member of the Ambassador Program;
	type InductOrigin = EitherOfDiverse<
		EnsureRoot<AccountId>,
		EitherOfDiverse<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			pallet_ranked_collective::EnsureMember<
				Runtime,
				AmbassadorCollectiveInstance,
				{ ranks::AMBASSADOR },
			>,
		>,
	>;
	type ApproveOrigin = PromoteOrigin;
	type PromoteOrigin = PromoteOrigin;
	type EvidenceSize = ConstU32<65536>;
}

parameter_types! {
	// The interior location on AssetHub for the paying account. This is the Ambassador Salary
	// pallet instance. This sovereign account will need funding.
	pub AmbassadorSalaryLocation: InteriorLocation =
		PalletInstance(<crate::AmbassadorSalary as PalletInfoAccess>::index() as u8).into();
}

const USDT_UNITS: u128 = 1_000_000;

/// [`PayOverXcm`] setup to pay the Ambassador salary on the AssetHub in USDt.
pub type AmbassadorSalaryPaymaster = PayOverXcm<
	AmbassadorSalaryLocation,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	AccountId,
	(),
	ConvertToValue<AssetHubUsdt>,
	AliasesIntoAccountId32<(), AccountId>,
>;

pub type AmbassadorSalaryInstance = pallet_salary::Instance2;

impl pallet_salary::Config<AmbassadorSalaryInstance> for Runtime {
	type WeightInfo = weights::pallet_salary_ambassador_salary::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = AmbassadorSalaryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = crate::impls::benchmarks::PayWithEnsure<
		AmbassadorSalaryPaymaster,
		crate::impls::benchmarks::OpenHrmpChannel<ConstU32<1000>>,
	>;
	type Members = pallet_ranked_collective::Pallet<Runtime, AmbassadorCollectiveInstance>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Salary = pallet_core_fellowship::Pallet<Runtime, AmbassadorCoreInstance>;
	#[cfg(feature = "runtime-benchmarks")]
	type Salary = frame_support::traits::tokens::ConvertRank<
		crate::impls::benchmarks::RankToSalary<Balances>,
	>;
	// 15 days to register for a salary payment.
	type RegistrationPeriod = ConstU32<{ 15 * DAYS }>;
	// 15 days to claim the salary payment.
	type PayoutPeriod = ConstU32<{ 15 * DAYS }>;
	// Total monthly salary budget.
	// 10,000 USDT for up to 21 members.
	type Budget = ConstU128<{ 10_000 * 21 * USDT_UNITS }>;
}

parameter_types! {
	pub const AmbassadorTreasuryPalletId: PalletId = AMBASSADOR_TREASURY_PALLET_ID;
	pub const ProposalBond: Permill = Permill::from_percent(100);
	pub const Burn: Permill = Permill::from_percent(0);
	pub const MaxBalance: Balance = Balance::MAX;
	// The asset's interior location for the paying account. This is the Ambassador Treasury
	// pallet instance.
	pub AmbassadorTreasuryInteriorLocation: InteriorLocation =
		PalletInstance(<crate::AmbassadorTreasury as PalletInfoAccess>::index() as u8).into();
}

#[cfg(feature = "runtime-benchmarks")]
parameter_types! {
	pub const ProposalBondForBenchmark: Permill = Permill::from_percent(5);
}

/// [`PayOverXcm`] setup to pay the Ambasssador Treasury.
pub type AmbassadorTreasuryPaymaster = PayOverXcm<
	AmbassadorTreasuryInteriorLocation,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	VersionedLocation,
	VersionedLocatableAsset,
	LocatableAssetConverter,
	VersionedLocationConverter,
>;

pub type AmbassadorTreasuryInstance = pallet_treasury::Instance2;

impl pallet_treasury::Config<AmbassadorTreasuryInstance> for Runtime {
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

	type WeightInfo = weights::pallet_treasury_ambassador_treasury::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type PalletId = AmbassadorTreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = OpenGovOrHeadAmbassadors;
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
			MapSuccess<SeniorAmbassadors, Replace<ConstU128<{ 100 * UNITS }>>>,
			MapSuccess<HeadAmbassadors, Replace<ConstU128<{ 10 * GRAND }>>>,
		>,
	>;
	type AssetKind = VersionedLocatableAsset;
	type Beneficiary = VersionedLocation;
	type BeneficiaryLookup = IdentityLookup<Self::Beneficiary>;
	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = AmbassadorTreasuryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = crate::impls::benchmarks::PayWithEnsure<
		AmbassadorTreasuryPaymaster,
		crate::impls::benchmarks::OpenHrmpChannel<ConstU32<1000>>,
	>;
	type BalanceConverter = crate::impls::NativeOnSiblingParachain<AssetRate, ParachainInfo>;
	type PayoutPeriod = ConstU32<{ 30 * DAYS }>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::TreasuryArguments<
		sp_core::ConstU8<1>,
		ConstU32<1000>,
	>;
}
