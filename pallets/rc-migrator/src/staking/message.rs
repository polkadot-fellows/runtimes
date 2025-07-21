// Copyright (C) Parity Technologies (UK) Ltd.
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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! The messages that we use to send the staking data over from RC to AH.

extern crate alloc;

use crate::{
	staking::{AccountIdOf, BalanceOf, IntoAh, StakingMigrator},
	*,
};
use alloc::collections::BTreeMap;
use codec::{EncodeLike, HasCompact};
use core::fmt::Debug;
pub use frame_election_provider_support::PageIndex;
use crate::types::DefensiveTruncateInto;
use pallet_staking::{
	slashing::{SlashingSpans, SpanIndex, SpanRecord},
	ActiveEraInfo, EraRewardPoints, Forcing, Nominations, RewardDestination, StakingLedger,
	ValidatorPrefs,
};
use sp_runtime::{Perbill, Percent};
use sp_staking::{EraIndex, Page, SessionIndex};

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	RuntimeDebugNoBound,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
)]
pub enum PortableStakingMessage
{
	Values(PortableStakingValues),
	Invulnerables(Vec<AccountId32>),
	Bonded {
		stash: AccountId32,
		controller: AccountId32,
	},
	// Stupid staking pallet forces us to use `T` since its staking ledger requires that...
	Ledger {
		controller: AccountId32,
		ledger: PortableStakingLedger,
	},
	Payee {
		stash: AccountId32,
		payment: PortableRewardDestination,
	},
	Validators {
		stash: AccountId32,
		validators: ValidatorPrefs,
	},
	Nominators {
		stash: AccountId32,
		nominations: PortableNominations,
	},
	VirtualStakers(AccountId32),
	ErasStakersOverview {
		era: EraIndex,
		validator: AccountId32,
		exposure: PortablePagedExposureMetadata,
	},
	ErasStakersPaged {
		era: EraIndex,
		validator: AccountId32,
		page: Page,
		exposure: PortableExposurePage,
	},
	ClaimedRewards {
		era: EraIndex,
		validator: AccountId32,
		rewards: Vec<Page>,
	},
	ErasValidatorPrefs {
		era: EraIndex,
		validator: AccountId32,
		prefs: ValidatorPrefs,
	},
	ErasValidatorReward {
		era: EraIndex,
		reward: u128,
	},
	ErasRewardPoints {
		era: EraIndex,
		points: PortableEraRewardPoints,
	},
	ErasTotalStake {
		era: EraIndex,
		total_stake: u128,
	},
	UnappliedSlashes {
		era: EraIndex,
		slash: PortableUnappliedSlash,
	},
	BondedEras(Vec<(EraIndex, SessionIndex)>),
	ValidatorSlashInEra {
		era: EraIndex,
		validator: AccountId32,
		slash: (Perbill, u128),
	},
	NominatorSlashInEra {
		era: EraIndex,
		validator: AccountId32,
		slash: (Perbill, u128),
	}
}

#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug, Clone, PartialEq, Eq)]
pub struct StakingValues<Balance> {
	pub validator_count: Option<u32>,
	pub min_validator_count: Option<u32>,
	pub min_nominator_bond: Option<Balance>,
	pub min_validator_bond: Option<Balance>,
	pub min_active_stake: Option<Balance>,
	pub min_commission: Option<Perbill>,
	pub max_validators_count: Option<u32>,
	pub max_nominators_count: Option<u32>,
	pub current_era: Option<EraIndex>,
	pub active_era: Option<ActiveEraInfo>,
	pub force_era: Option<Forcing>,
	pub max_staked_rewards: Option<Percent>,
	pub slash_reward_fraction: Option<Perbill>,
	pub canceled_slash_payout: Option<Balance>,
	pub current_planned_session: Option<SessionIndex>,
	pub chill_threshold: Option<Percent>,
}

impl<T: pallet_staking::Config> StakingMigrator<T> {
	/// Take and remove the values from the storage.
	pub fn take_values() -> StakingValues<BalanceOf<T>> {
		use pallet_staking::*;

		StakingValues {
			validator_count: ValidatorCount::<T>::exists().then(ValidatorCount::<T>::take),
			min_validator_count: MinimumValidatorCount::<T>::exists()
				.then(MinimumValidatorCount::<T>::take),
			min_nominator_bond: MinNominatorBond::<T>::exists().then(MinNominatorBond::<T>::take),
			min_validator_bond: MinValidatorBond::<T>::exists().then(MinValidatorBond::<T>::take),
			min_active_stake: MinimumActiveStake::<T>::exists().then(MinimumActiveStake::<T>::take),
			min_commission: MinCommission::<T>::exists().then(MinCommission::<T>::take),
			max_validators_count: MaxValidatorsCount::<T>::take(),
			max_nominators_count: MaxNominatorsCount::<T>::take(),
			current_era: CurrentEra::<T>::take(),
			active_era: ActiveEra::<T>::take(),
			force_era: ForceEra::<T>::exists().then(ForceEra::<T>::take),
			max_staked_rewards: MaxStakedRewards::<T>::take(),
			slash_reward_fraction: SlashRewardFraction::<T>::exists()
				.then(SlashRewardFraction::<T>::take),
			canceled_slash_payout: CanceledSlashPayout::<T>::exists()
				.then(CanceledSlashPayout::<T>::take),
			current_planned_session: CurrentPlannedSession::<T>::exists()
				.then(CurrentPlannedSession::<T>::take),
			chill_threshold: ChillThreshold::<T>::take(),
		}
	}
}

pub type PortableStakingValues = StakingValues<u128>;

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PortableStakingLedger {
	/// The stash account whose balance is actually locked and at stake.
	pub stash: AccountId32,

	/// The total amount of the stash's balance that we are currently accounting for.
	/// It's just `active` plus all the `unlocking` balances.
	pub total: u128,

	/// The total amount of the stash's balance that will be at stake in any forthcoming
	/// rounds.
	pub active: u128,

	/// Any balance that is becoming free, which may eventually be transferred out of the stash
	/// (assuming it doesn't get slashed first). It is assumed that this will be treated as a first
	/// in, first out queue where the new (higher value) eras get pushed on the back.
	pub unlocking: BoundedVec<PortableUnlockChunk, ConstU32<100>>, // 100 is an upper bound TODO @kianenigma review
}

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PortableUnlockChunk {
	/// Amount of funds to be unlocked.
	pub value: u128,
	/// Era number at which point it'll be unlocked.
	pub era: EraIndex,
}

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PortableUnappliedSlash {
	/// The stash ID of the offending validator.
	pub validator: AccountId32,
	/// The validator's own slash.
	pub own: u128,
	/// All other slashed stakers and amounts.
	pub others: BoundedVec<(AccountId32, u128), ConstU32<100>>, // 100 is an upper bound TODO @kianenigma review
	/// Reporters of the offence; bounty payout recipients.
	pub reporter: Option<AccountId32>,
	/// The amount of payout.
	pub payout: u128,
}

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum PortableRewardDestination {
	/// Pay into the stash account, increasing the amount at stake accordingly.
	Staked,
	/// Pay into the stash account, not increasing the amount at stake.
	Stash,
	#[deprecated(
		note = "`Controller` will be removed after January 2024. Use `Account(controller)` instead."
	)]
	Controller,
	/// Pay into a specified account.
	Account(AccountId32),
	/// Receive no reward.
	None,
}

#[derive(
	PartialEqNoBound, EqNoBound, Clone, Encode, Decode, RuntimeDebugNoBound, TypeInfo, MaxEncodedLen, DecodeWithMemTracking,
)]
pub struct PortableNominations {
	/// The targets of nomination.
	pub targets: BoundedVec<AccountId32, ConstU32<100>>, // 100 is an upper bound TODO @kianenigma review
	/// The era the nominations were submitted.
	///
	/// Except for initial nominations which are considered submitted at era 0.
	pub submitted_in: EraIndex,
	/// Whether the nominations have been suppressed. This can happen due to slashing of the
	/// validators, or other events that might invalidate the nomination.
	///
	/// NOTE: this for future proofing and is thus far not used.
	pub suppressed: bool,
}

#[derive(
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Clone,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	Default,
	MaxEncodedLen,
	DecodeWithMemTracking,
)]
pub struct PortablePagedExposureMetadata {
	/// The total balance backing this validator.
	pub total: u128,
	/// The validator's own stash that is exposed.
	pub own: u128,
	/// Number of nominators backing this validator.
	pub nominator_count: u32,
	/// Number of pages of nominators.
	pub page_count: Page,
}

/// A snapshot of the stake backing a single validator in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, DecodeWithMemTracking)]
pub struct PortableExposurePage {
	/// The total balance of this chunk/page.
	pub page_total: u128,
	/// The portions of nominators stashes that are exposed.
	pub others: Vec<PortableIndividualExposure>,
}

#[derive(
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Clone,
	Encode,
	Decode,
	DecodeWithMemTracking,
	RuntimeDebug,
	TypeInfo,
)]
pub struct PortableIndividualExposure {
	/// The stash account of the nominator in question.
	pub who: AccountId32,
	/// Amount of funds exposed.
	pub value: u128,
}

#[derive(
	PartialEq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen, DecodeWithMemTracking,
)]
pub struct PortableEraRewardPoints {
	/// Total number of points. Equals the sum of reward points for each validator.
	pub total: u32,
	/// The reward points earned by a given validator.
	pub individual: BoundedBTreeMap<AccountId32, u32, ConstU32<100>>, // 100 is an upper bound TODO @kianenigma review
}
