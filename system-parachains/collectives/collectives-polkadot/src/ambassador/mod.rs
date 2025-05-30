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
//! - Managed set of program members, where every member has a [rank](ranks) (via
//!   [AmbassadorCollective](pallet_ranked_collective_ambassador)).
//! - Referendum functionality for the program members to propose, vote on, and execute proposals on
//!   behalf of the members of a certain [rank](Origin) (via
//!   [AmbassadorReferenda](pallet_referenda)).
//! - Promotion and demotion periods, register of members' activity(via
//!   [AmbassadorCore](pallet_core_fellowship)). inducted into
//!   [AmbassadorCore](pallet_core_fellowship)).
//! - Ambassador Program Sub-Treasury (via [AmbassadorTreasury](pallet_treasury)).

pub mod origins;
mod tracks;

pub use origins::pallet_origins as pallet_ambassador_origins;

use crate::{xcm_config::FellowshipAdminBodyId, AssetRateWithNative, *};
use frame_support::{
	pallet_prelude::PalletInfoAccess,
	traits::{EitherOf, MapSuccess, TryMapSuccess},
};
use frame_system::EnsureRootWithSuccess;
use origins::pallet_origins::{
	EnsureAmbassadorsFrom, EnsureCanFastPromoteTo, EnsureCanPromoteTo, EnsureCanRetainAt,
	GlobalHeadAmbassador, Spender,
};
use pallet_ranked_collective_ambassador::{MemberIndex, Rank, Votes};
use polkadot_runtime_common::impls::{LocatableAssetConverter, VersionedLocationConverter};
use polkadot_runtime_constants::time::HOURS;
use sp_runtime::{
	traits::{CheckedReduceBy, Convert, IdentityLookup, MaybeConvert, Replace},
	Permill,
};
use xcm::prelude::*;
use xcm_builder::PayOverXcm;

/// The Ambassador Program's member ranks.
pub mod ranks {
	use super::Rank;

	#[allow(dead_code)]
	pub const ADVOCATE_AMBASSADOR: Rank = 0; // aka Candidate.
	pub const ASSOCIATE_AMBASSADOR: Rank = 1;
	pub const LEAD_AMBASSADOR: Rank = 2;
	pub const SENIOR_AMBASSADOR: Rank = 3;
	pub const PRINCIPAL_AMBASSADOR: Rank = 4;
	pub const GLOBAL_AMBASSADOR: Rank = 5;
	pub const GLOBAL_HEAD_AMBASSADOR: Rank = 6;
}

impl pallet_ambassador_origins::Config for Runtime {}

/// Demotion is by any of:
/// - Root can demote arbitrarily;
/// - the FellowshipAdmin voice (i.e. token holder referendum) can demote arbitrarily;
/// - Senior Ambassadors voice can demote Ambassador.
pub type DemoteOrigin = EitherOf<
	EnsureRootWithSuccess<AccountId, ConstU16<65535>>,
	EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::GLOBAL_HEAD_AMBASSADOR }>>,
		>,
		TryMapSuccess<
			EnsureAmbassadorsFrom<ConstU16<{ ranks::SENIOR_AMBASSADOR }>>,
			CheckedReduceBy<ConstU16<1>>,
		>,
	>,
>;

/// Root, FellowshipAdmin or HeadAmbassadors.
pub type OpenGovOrGlobalHeadAmbassador = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EitherOfDiverse<
		GlobalHeadAmbassador,
		EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
	>,
>;

/// Ambassadors' vote weights for referendums.
/// - Rank 0 (Advocate): 1 votes (excluded in the voting system)
/// - Rank I (Associate): 1 vote
/// - Rank II (Lead): 3 votes (1+2)
/// - Rank III (Senior): 6 votes (1+2+3)
/// - Rank IV (Principal): 10 votes (1+2+3+4)
/// - Rank V (Global): 15 votes (1+2+3+4+5)
/// - Rank VI (Global Head): 21 votes (1+2+3+4+5+6)
pub struct VoteWeight;
impl Convert<Rank, Votes> for VoteWeight {
	fn convert(absolute_rank: Rank) -> Votes {
    	(absolute_rank * (absolute_rank + 1) / 2).into()
    }
}

pub type AmbassadorCollectiveInstance = pallet_ranked_collective_ambassador::Instance2;

impl pallet_ranked_collective_ambassador::Config<AmbassadorCollectiveInstance> for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	// Promotions must be done through the [`crate::AmbassadorCore`] pallet instance.
	#[cfg(not(feature = "runtime-benchmarks"))]
	type PromoteOrigin = frame_support::traits::NeverEnsureOrigin<Rank>;
	#[cfg(feature = "runtime-benchmarks")]
	type PromoteOrigin = EnsureRootWithSuccess<AccountId, ConstU16<65535>>;
	type DemoteOrigin = DemoteOrigin;
	type RemoveOrigin = Self::DemoteOrigin;
	type Polls = AmbassadorReferenda;
	type MinRankOfClass = tracks::MinRankOfClass;
	type VoteWeight = pallet_ranked_collective_ambassador::Geometric;
	type ExchangeOrigin = OpenGovOrGlobalHeadAmbassador;
	type MemberSwappedHandler = crate::AmbassadorCore;
	#[cfg(feature = "runtime-benchmarks")]
	type MaxMemberCount = ();
	#[cfg(not(feature = "runtime-benchmarks"))]
	type MaxMemberCount = AmbassadorMemberCount;
	type Currency = Balances;
	type InductionDeposit = InductionDeposit;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = crate::AmbassadorCore;
}

/// Limits the number of Global Head Ambassadors to 21.
///
/// The value of 21 comes from the initial OpenGov proposal: <https://github.com/polkadot-fellows/runtimes/issues/264>
pub struct AmbassadorMemberCount;
impl MaybeConvert<Rank, MemberIndex> for AmbassadorMemberCount {
	fn maybe_convert(rank: Rank) -> Option<MemberIndex> {
		(rank == 6).then_some(21)
	}
}

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 0;
	pub const UndecidingTimeout: BlockNumber = 7 * DAYS;
	pub const InductionDeposit: u64 = 1;
}

pub type AmbassadorReferendaInstance = pallet_referenda::Instance2;

impl pallet_referenda::Config<AmbassadorReferendaInstance> for Runtime {
	type WeightInfo = weights::pallet_referenda_ambassador_referenda::WeightInfo<Runtime>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	// Any member of the Ambassador Program can submit a proposal.
	type SubmitOrigin = pallet_ranked_collective_ambassador::EnsureMember<
		Runtime,
		AmbassadorCollectiveInstance,
		{ ranks::ASSOCIATE_AMBASSADOR },
	>;
	// Referendum can be canceled by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Global Head Ambassadors.
	type CancelOrigin = OpenGovOrGlobalHeadAmbassador;
	// Referendum can be killed by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Global Head Ambassadors.
	type KillOrigin = OpenGovOrGlobalHeadAmbassador;
	type Slash = ToParentTreasury<PolkadotTreasuryAccount, LocationToAccountId, Runtime>;
	type Votes = Votes;
	type Tally =
		pallet_ranked_collective_ambassador::TallyOf<Runtime, AmbassadorCollectiveInstance>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<20>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = tracks::TracksInfo;
	type Preimages = Preimage;
}

pub type AmbassadorCoreInstance = pallet_core_fellowship_ambassador::Instance2;

impl pallet_core_fellowship_ambassador::Config<AmbassadorCoreInstance> for Runtime {
	type WeightInfo = ();
	type RuntimeEvent = RuntimeEvent;
	type Members =
		pallet_ranked_collective_ambassador::Pallet<Runtime, AmbassadorCollectiveInstance>;
	type Balance = Balance;
	// Parameters are set by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Head Ambassadors.
	type ParamsOrigin = OpenGovOrGlobalHeadAmbassador;
	type ApproveOrigin = EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::GLOBAL_HEAD_AMBASSADOR }>>,
		>,
		EnsureCanRetainAt,
	>;
	type PromoteOrigin = EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::GLOBAL_HEAD_AMBASSADOR }>>,
		>,
		EnsureCanPromoteTo,
	>;
	type FastPromoteOrigin = EnsureCanFastPromoteTo;
	type Currency = Balances;
	type EvidenceSize = ConstU32<65536>;
	// TODO https://github.com/polkadot-fellows/runtimes/issues/370
	type MaxRank = ConstU16<6>;
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

pub type TreasurySpender = EitherOf<EnsureRootWithSuccess<AccountId, MaxBalance>, Spender>;

pub type AmbassadorTreasuryInstance = pallet_treasury::Instance2;

impl pallet_treasury::Config<AmbassadorTreasuryInstance> for Runtime {
	type WeightInfo = weights::pallet_treasury_ambassador_treasury::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type PalletId = AmbassadorTreasuryPalletId;
	type Currency = Balances;
	type RejectOrigin = OpenGovOrGlobalHeadAmbassador;
	type SpendPeriod = ConstU32<{ 7 * DAYS }>;
	type Burn = Burn;
	type BurnDestination = ();
	type SpendFunds = ();
	type MaxApprovals = ConstU32<100>;
	type SpendOrigin = TreasurySpender;
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
	type BalanceConverter = AssetRateWithNative;
	type PayoutPeriod = ConstU32<{ 90 * DAYS }>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = polkadot_runtime_common::impls::benchmarks::TreasuryArguments<
		sp_core::ConstU8<1>,
		ConstU32<1000>,
	>;
	type BlockNumberProvider = System;
}

#[cfg(all(test, not(feature = "runtime-benchmarks")))]
mod tests {
	use super::*;

	type Limit = <Runtime as pallet_ranked_collective_ambassador::Config<
		AmbassadorCollectiveInstance,
	>>::MaxMemberCount;

	#[test]
	fn ambassador_rank_limit_works() {
		assert_eq!(Limit::maybe_convert(0), None);
		assert_eq!(Limit::maybe_convert(1), None);
		assert_eq!(Limit::maybe_convert(2), None);
		assert_eq!(Limit::maybe_convert(3), Some(21));
		assert_eq!(Limit::maybe_convert(4), None);
	}
}
