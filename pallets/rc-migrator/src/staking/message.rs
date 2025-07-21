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
	*, types::{defensive_vector_truncate, defensive_vector_translate},
};
use alloc::collections::BTreeMap;
use codec::{EncodeLike, HasCompact};
use core::fmt::Debug;
pub use frame_election_provider_support::PageIndex;
use crate::types::DefensiveTruncateInto;
use pallet_staking::{
	slashing::{SlashingSpans, SpanIndex, SpanRecord}, EraRewardPoints, Nominations, RewardDestination, StakingLedger,
	ValidatorPrefs,
};
use sp_runtime::{Perbill, Percent};
use sp_staking::{EraIndex, Page, SessionIndex};

/// Portable staking migration message.
///
/// It is portable since it does not have any generic type parameters.
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
		slash: u128,
	},
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
	pub active_era: Option<PortableActiveEraInfo>,
	pub force_era: Option<PortableForcing>,
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
			active_era: ActiveEra::<T>::take().map(IntoPortable::into_portable),
			force_era: ForceEra::<T>::exists().then(ForceEra::<T>::take).map(IntoPortable::into_portable),
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

impl<T: pallet_staking_async::Config> StakingMigrator<T> {
	pub fn put_values(values: StakingValues<pallet_staking_async::BalanceOf<T>>) {
		use pallet_staking_async::*;

		values.validator_count.map(ValidatorCount::<T>::put);
		// MinimumValidatorCount is not migrated
		values.min_nominator_bond.map(MinNominatorBond::<T>::put);
		values.min_validator_bond.map(MinValidatorBond::<T>::put);
		values.min_active_stake.map(MinimumActiveStake::<T>::put);
		values.min_commission.map(MinCommission::<T>::put);
		values.max_validators_count.map(MaxValidatorsCount::<T>::put);
		values.max_nominators_count.map(MaxNominatorsCount::<T>::put);
		values.active_era.map(|active_era| {
			let active_era: pallet_staking_async::ActiveEraInfo = active_era.into();
			ActiveEra::<T>::put(&active_era);
			CurrentEra::<T>::put(active_era.index);
		});
		values.force_era.map(|force_era| {
			let force_era: pallet_staking_async::Forcing = force_era.into();
			ForceEra::<T>::put(force_era);
		});
		values.max_staked_rewards.map(MaxStakedRewards::<T>::put);
		values.slash_reward_fraction.map(SlashRewardFraction::<T>::put);
		values.canceled_slash_payout.map(CanceledSlashPayout::<T>::put);
		// CurrentPlannedSession is not migrated
		values.chill_threshold.map(ChillThreshold::<T>::put);
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
pub struct PortableActiveEraInfo {
	/// Index of era.
	pub index: EraIndex,
	/// Moment of start expressed as millisecond from `$UNIX_EPOCH`.
	///
	/// Start can be none if start hasn't been set for the era yet,
	/// Start is set on the first on_finalize of the era to guarantee usage of `Time`.
	pub start: Option<u64>,
}

impl IntoPortable for pallet_staking::ActiveEraInfo {
	type Portable = PortableActiveEraInfo;
	
	fn into_portable(self) -> Self::Portable {
		PortableActiveEraInfo { index: self.index, start: self.start }
	}
}

impl Into<pallet_staking_async::ActiveEraInfo> for PortableActiveEraInfo {
	fn into(self) -> pallet_staking_async::ActiveEraInfo {
		pallet_staking_async::ActiveEraInfo { index: self.index, start: self.start }
	}
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
pub enum PortableForcing {
	/// Not forcing anything - just let whatever happen.
	NotForcing,
	/// Force a new era, then reset to `NotForcing` as soon as it is done.
	/// Note that this will force to trigger an election until a new era is triggered, if the
	/// election failed, the next session end will trigger a new election again, until success.
	ForceNew,
	/// Avoid a new era indefinitely.
	ForceNone,
	/// Force a new era at the end of all sessions indefinitely.
	ForceAlways,
}

// Forcing: RC -> Portable
impl IntoPortable for pallet_staking::Forcing {
	type Portable = PortableForcing;
	
	fn into_portable(self) -> Self::Portable {
		match self {
			pallet_staking::Forcing::NotForcing => PortableForcing::NotForcing,
			pallet_staking::Forcing::ForceNew => PortableForcing::ForceNew,
			pallet_staking::Forcing::ForceNone => PortableForcing::ForceNone,
			pallet_staking::Forcing::ForceAlways => PortableForcing::ForceAlways,
		}
	}
}

// Forcing: Portable -> AH
impl Into<pallet_staking_async::Forcing> for PortableForcing {
	fn into(self) -> pallet_staking_async::Forcing {
		match self {
			PortableForcing::NotForcing => pallet_staking_async::Forcing::NotForcing,
			PortableForcing::ForceNew => pallet_staking_async::Forcing::ForceNew,
			PortableForcing::ForceNone => pallet_staking_async::Forcing::ForceNone,
			PortableForcing::ForceAlways => pallet_staking_async::Forcing::ForceAlways,
		}
	}
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

impl<T: Config> IntoPortable for pallet_staking::StakingLedger<T> {
	type Portable = PortableStakingLedger;
	
	fn into_portable(self) -> Self::Portable {
		// TODO @kianenigma what to do with `legacy_claimed_rewards`?
		defensive_assert!(self.legacy_claimed_rewards.is_empty());
		
		PortableStakingLedger {
			stash: self.stash,
			total: self.total,
			active: self.active,
			unlocking: defensive_vector_translate(self.unlocking),
			// TODO @kianenigma controller is ignored, right?
			// self.controller,
		}
	}
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

impl IntoPortable for pallet_staking::UnlockChunk<u128> {
	type Portable = PortableUnlockChunk;
	
	fn into_portable(self) -> Self::Portable {
		PortableUnlockChunk { value: self.value, era: self.era }
	}
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
	pub reporters: BoundedVec<AccountId32, ConstU32<100>>, // 100 is an upper bound TODO @kianenigma review
	/// The amount of payout.
	pub payout: u128,
}

impl IntoPortable for pallet_staking::UnappliedSlash<AccountId32, u128> {
	type Portable = PortableUnappliedSlash;
	
	fn into_portable(self) -> Self::Portable {
		PortableUnappliedSlash {
			validator: self.validator,
			own: self.own,
			others: self.others.defensive_truncate_into(),
			reporters: self.reporters.defensive_truncate_into(),
			payout: self.payout,
		}
	}
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
	/// Deprecated
	Controller,
	/// Pay into a specified account.
	Account(AccountId32),
	/// Receive no reward.
	None,
}

impl IntoPortable for pallet_staking::RewardDestination<AccountId32> {
	type Portable = PortableRewardDestination;
	
	fn into_portable(self) -> Self::Portable {
		use PortableRewardDestination::*;
		
		match self {
			RewardDestination::Staked => Staked,
			RewardDestination::Stash => Stash,
			RewardDestination::Controller => Controller,
			RewardDestination::Account(account) => Account(account),
			RewardDestination::None => None,
		}
	}
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

impl<T: Config> IntoPortable for pallet_staking::Nominations<T> {
	type Portable = PortableNominations;
	
	fn into_portable(self) -> Self::Portable {
		PortableNominations {
			targets: defensive_vector_truncate(self.targets),
			submitted_in: self.submitted_in,
			suppressed: self.suppressed,
		}
	}
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

impl IntoPortable for sp_staking::PagedExposureMetadata<u128> {
	type Portable = PortablePagedExposureMetadata;
	
	fn into_portable(self) -> Self::Portable {
		PortablePagedExposureMetadata {
			total: self.total,
			own: self.own,
			nominator_count: self.nominator_count,
			page_count: self.page_count,
		}
	}
}

/// A snapshot of the stake backing a single validator in the system.
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, DecodeWithMemTracking)]
pub struct PortableExposurePage {
	/// The total balance of this chunk/page.
	pub page_total: u128,
	/// The portions of nominators stashes that are exposed.
	pub others: BoundedVec<PortableIndividualExposure, ConstU32<100>>, // 100 is an upper bound TODO @kianenigma review
}

impl IntoPortable for sp_staking::ExposurePage<AccountId32, u128> {
	type Portable = PortableExposurePage;
	
	fn into_portable(self) -> Self::Portable {
		PortableExposurePage {
			page_total: self.page_total,
			others: self.others.into_iter().map(IntoPortable::into_portable).collect::<Vec<_>>().defensive_truncate_into(),
		}
	}
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

impl IntoPortable for sp_staking::IndividualExposure<AccountId32, u128> {
	type Portable = PortableIndividualExposure;
	
	fn into_portable(self) -> Self::Portable {
		PortableIndividualExposure { who: self.who, value: self.value }
	}
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

impl IntoPortable for pallet_staking::EraRewardPoints<AccountId32> {
	type Portable = PortableEraRewardPoints;

	fn into_portable(self) -> Self::Portable {
		// TODO @ggwpez
		if self.individual.len() > 100 {
			defensive!("EraRewardPoints truncated");
		}
		let individual = self.individual.into_iter().take(100).collect::<BTreeMap<_, _>>();

		PortableEraRewardPoints {
			total: self.total,
			individual: BoundedBTreeMap::try_from(individual).defensive().unwrap_or_default(),
		}
	}
}
