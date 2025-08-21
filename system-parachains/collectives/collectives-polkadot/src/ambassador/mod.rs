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
//!   [AmbassadorCollective](pallet_ranked_collective)).
//! - Referendum functionality for the program members to propose, vote on, and execute proposals on
//!   behalf of the members of a certain [rank](Origin) (via
//!   [AmbassadorReferenda](pallet_referenda)).
//! - Promotion and demotion periods, register of members' activity(via
//!   [AmbassadorCore](pallet_core_fellowship)). inducted into
//!   [AmbassadorCore](pallet_core_fellowship)).
//! - Ambassador Program Sub-Treasury (via [AmbassadorTreasury](pallet_treasury)).

mod migration;
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
	GlobalHead, Spender,
};
use pallet_ranked_collective::{Rank, Votes};
use polkadot_runtime_common::impls::{LocatableAssetConverter, VersionedLocationConverter};
use polkadot_runtime_constants::time::HOURS;
use sp_runtime::{
	traits::{CheckedReduceBy, Convert, IdentityLookup, Replace},
	Permill,
};
use xcm::prelude::*;
use xcm_builder::PayOverXcm;

/// The Ambassador Program's member ranks.
pub mod ranks {
	use super::Rank;

	#[allow(dead_code)]
	pub const ADVOCATE: Rank = 0; // aka Candidate.
	pub const ASSOCIATE: Rank = 1;
	pub const LEAD: Rank = 2;
	pub const SENIOR: Rank = 3;
	pub const PRINCIPAL: Rank = 4;
	pub const GLOBAL: Rank = 5;
	pub const GLOBAL_HEAD: Rank = 6;
}

/// - Each member with an excess rank of 0 gets 0 votes;
/// - ...with an excess rank of 1 gets 1 vote;
/// - ...with an excess rank of 2 gets 3 votes;
/// - ...with an excess rank of 3 gets 6 votes;
/// - ...with an excess rank of 4 gets 10 votes.
/// - ...with an excess rank of 5 gets 15 votes.
/// - ...with an excess rank of 6 gets 21 votes.
pub struct Geometric;
impl Convert<Rank, Votes> for Geometric {
	fn convert(r: Rank) -> Votes {
		match r {
			0 => 0,
			1 => 1,
			2 => 3,
			3 => 6,
			4 => 10,
			5 => 15,
			6 => 21,
			// For ranks beyond 6, we return 0 since it's undefined
			_ => 0,
		}
	}
}

impl pallet_ambassador_origins::Config for Runtime {}

/// Demotion is by any of:
/// - Root can demote arbitrarily;
/// - the FellowshipAdmin voice (i.e. token holder referendum) can demote all but Global Head;
/// - Senior Ambassadors voice can demote Ambassador.
pub type DemoteOrigin = EitherOf<
	EnsureRootWithSuccess<AccountId, ConstU16<65535>>,
	EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::GLOBAL_HEAD }>>,
		>,
		TryMapSuccess<
			EnsureAmbassadorsFrom<ConstU16<{ ranks::SENIOR }>>,
			CheckedReduceBy<ConstU16<1>>,
		>,
	>,
>;

/// Root, FellowshipAdmin or HeadAmbassadors.
pub type OpenGovOrGlobalHead = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EitherOfDiverse<
		GlobalHead,
		EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
	>,
>;

/// Root orFellowshipAdmin
pub type RootOrOpenGov = EitherOfDiverse<
	EnsureRoot<AccountId>,
	EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
>;

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
	type RemoveOrigin = Self::DemoteOrigin;
	type Polls = AmbassadorReferenda;
	type MinRankOfClass = tracks::MinRankOfClass;
	type VoteWeight = Geometric;
	type AddOrigin = RootOrOpenGov;
	type ExchangeOrigin = RootOrOpenGov;
	type MemberSwappedHandler = crate::AmbassadorCore;
	type MaxMemberCount = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = crate::AmbassadorCore;
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
		{ ranks::ASSOCIATE },
	>;
	// Referendum can be canceled by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Global Head Ambassadors.
	type CancelOrigin = OpenGovOrGlobalHead;
	// Referendum can be killed by any of:
	// - Root;
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote among all Global Head Ambassadors.
	type KillOrigin = OpenGovOrGlobalHead;
	type Slash = ToParentTreasury<PolkadotTreasuryAccount, LocationToAccountId, Runtime>;
	type Votes = Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime, AmbassadorCollectiveInstance>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<20>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = tracks::TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
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
	// - a vote among all Global Head Ambassadors.
	type ParamsOrigin = OpenGovOrGlobalHead;
	type ApproveOrigin = EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::GLOBAL_HEAD }>>,
		>,
		EnsureCanRetainAt,
	>;
	type PromoteOrigin = EitherOf<
		MapSuccess<
			EnsureXcm<IsVoiceOfBody<GovernanceLocation, FellowshipAdminBodyId>>,
			Replace<ConstU16<{ ranks::GLOBAL_HEAD }>>,
		>,
		EnsureCanPromoteTo,
	>;
	type InductOrigin = OpenGovOrGlobalHead;
	type FastPromoteOrigin = EnsureCanFastPromoteTo;
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
	type RejectOrigin = OpenGovOrGlobalHead;
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
	use sp_runtime::traits::MaybeConvert;

	type MaxMemberCount =
		<Runtime as pallet_ranked_collective::Config<AmbassadorCollectiveInstance>>::MaxMemberCount;

	#[test]
	fn max_member_count_correct() {
		for i in 0..10 {
			let limit: Option<u16> = MaxMemberCount::maybe_convert(i);
			assert!(limit.is_none(), "Ambassador has no member limit");
		}
	}

	#[test]
	fn geometric_vote_weight_conversion() {
		use super::Geometric;
		use pallet_ranked_collective::Votes;

		// Test the Geometric vote weight conversion for ranks 0 to 6
		let test_cases = [
			(0, 0),  // rank 0 -> 0 votes
			(1, 1),  // rank 1 -> 1 vote
			(2, 3),  // rank 2 -> 3 votes
			(3, 6),  // rank 3 -> 6 votes
			(4, 10), // rank 4 -> 10 votes
			(5, 15), // rank 5 -> 15 votes
			(6, 21), // rank 6 -> 21 votes
		];

		for (rank, expected_votes) in test_cases {
			assert_eq!(
				Geometric::convert(rank),
				expected_votes as Votes,
				"Vote weight conversion failed for rank {}",
				rank
			);
		}

		// Test that ranks beyond 6 return 0
		assert_eq!(Geometric::convert(7), 0);
		assert_eq!(Geometric::convert(8), 0);
		assert_eq!(Geometric::convert(100), 0);
	}
}
