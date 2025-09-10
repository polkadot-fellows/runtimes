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
	staking::{BalanceOf, StakingMigrator},
	types::{DefensiveTruncateInto, TranslateAccounts},
	*,
};
use alloc::collections::BTreeMap;
use pallet_staking::RewardDestination;
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
pub enum PortableStakingMessage {
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
		validators: PortableValidatorPrefs,
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
		prefs: PortableValidatorPrefs,
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
}

impl TranslateAccounts for PortableStakingMessage {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		use PortableStakingMessage::*;

		match self {
			Values(values) => Values(values),
			Invulnerables(invulnerables) =>
				Invulnerables(invulnerables.into_iter().map(f).collect::<Vec<_>>()),
			Bonded { stash, controller } => Bonded { stash: f(stash), controller: f(controller) },
			Ledger { controller, ledger } =>
				Ledger { controller: f(controller), ledger: ledger.translate_accounts(f) },
			Payee { stash, payment } =>
				Payee { stash: f(stash), payment: payment.translate_accounts(f) },
			Validators { stash, validators } =>
				Validators { stash: f(stash), validators: validators.translate_accounts(f) },
			Nominators { stash, nominations } =>
				Nominators { stash: f(stash), nominations: nominations.translate_accounts(f) },
			VirtualStakers(stash) => VirtualStakers(f(stash)),
			ErasStakersOverview { era, validator, exposure } => ErasStakersOverview {
				era,
				validator: f(validator),
				exposure: exposure.translate_accounts(f),
			},
			ErasStakersPaged { era, validator, page, exposure } => ErasStakersPaged {
				era,
				validator: f(validator),
				page,
				exposure: exposure.translate_accounts(f),
			},
			ClaimedRewards { era, validator, rewards } =>
				ClaimedRewards { era, validator: f(validator), rewards },
			ErasValidatorPrefs { era, validator, prefs } => ErasValidatorPrefs {
				era,
				validator: f(validator),
				prefs: prefs.translate_accounts(f),
			},
			ErasValidatorReward { era, reward } => ErasValidatorReward { era, reward },
			ErasRewardPoints { era, points } =>
				ErasRewardPoints { era, points: points.translate_accounts(f) },
			ErasTotalStake { era, total_stake } => ErasTotalStake { era, total_stake },
			UnappliedSlashes { era, slash } =>
				UnappliedSlashes { era, slash: slash.translate_accounts(f) },
			BondedEras(eras) => BondedEras(eras),
			ValidatorSlashInEra { era, validator, slash } =>
				ValidatorSlashInEra { era, validator: f(validator), slash },
		}
	}
}

/// Generic staking storage values.
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
			force_era: ForceEra::<T>::exists()
				.then(ForceEra::<T>::take)
				.map(IntoPortable::into_portable),
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
	/// Put the values into the storage.
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
	pub index: EraIndex,
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
	NotForcing,
	ForceNew,
	ForceNone,
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
	pub stash: AccountId32,
	pub total: u128,
	pub active: u128,
	// Expected to be around 32, but we can use 100 as upper bound.
	pub unlocking: BoundedVec<PortableUnlockChunk, ConstU32<100>>,
}

impl TranslateAccounts for PortableStakingLedger {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableStakingLedger {
			stash: f(self.stash),
			total: self.total,
			active: self.active,
			unlocking: self
				.unlocking
				.into_iter()
				.map(|c| c.translate_accounts(f))
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
		}
	}
}

// StakingLedger: RC -> Portable
impl<T: Config> IntoPortable for pallet_staking::StakingLedger<T> {
	type Portable = PortableStakingLedger;

	fn into_portable(self) -> Self::Portable {
		// We drop the `legacy_claimed_rewards` field, as they are not used anymore.

		PortableStakingLedger {
			stash: self.stash,
			total: self.total,
			active: self.active,
			unlocking: self
				.unlocking
				.into_iter()
				.map(IntoPortable::into_portable)
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
			// self.controller is ignored since its not part of the storage.
		}
	}
}

// StakingLedger: Portable -> AH
impl<
		T: pallet_staking_async::Config<CurrencyBalance = u128>
			+ frame_system::Config<AccountId = AccountId32>,
	> Into<pallet_staking_async::StakingLedger<T>> for PortableStakingLedger
{
	fn into(self) -> pallet_staking_async::StakingLedger<T> {
		pallet_staking_async::StakingLedger {
			stash: self.stash,
			total: self.total,
			active: self.active,
			unlocking: self
				.unlocking
				.into_iter()
				.map(Into::into)
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
			controller: None, // Not needed
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
	pub value: u128,
	pub era: EraIndex,
}

impl TranslateAccounts for PortableUnlockChunk {
	fn translate_accounts(self, _f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		// No-OP
		self
	}
}

// UnlockChunk: RC -> Portable
impl IntoPortable for pallet_staking::UnlockChunk<u128> {
	type Portable = PortableUnlockChunk;

	fn into_portable(self) -> Self::Portable {
		PortableUnlockChunk { value: self.value, era: self.era }
	}
}

// UnlockChunk: Portable -> AH
impl Into<pallet_staking_async::UnlockChunk<u128>> for PortableUnlockChunk {
	fn into(self) -> pallet_staking_async::UnlockChunk<u128> {
		pallet_staking_async::UnlockChunk { value: self.value, era: self.era }
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
	pub validator: AccountId32,
	pub own: u128,
	pub others: BoundedVec<(AccountId32, u128), ConstU32<600>>, // Range 0-512
	pub reporters: BoundedVec<AccountId32, ConstU32<10>>,       // Range 0-1
	pub payout: u128,
}

impl TranslateAccounts for PortableUnappliedSlash {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableUnappliedSlash {
			validator: f(self.validator),
			own: self.own,
			others: self
				.others
				.into_iter()
				.map(|(who, value)| (f(who), value))
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
			reporters: self
				.reporters
				.into_iter()
				.map(f)
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
			payout: self.payout,
		}
	}
}

// UnappliedSlash: RC -> Portable
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

// UnappliedSlash: Portable -> AH
impl<
		T: pallet_staking_async::Config<CurrencyBalance = u128>
			+ frame_system::Config<AccountId = AccountId32>,
	> Into<pallet_staking_async::UnappliedSlash<T>> for PortableUnappliedSlash
{
	fn into(self) -> pallet_staking_async::UnappliedSlash<T> {
		if self.others.len() > T::MaxExposurePageSize::get() as usize {
			defensive!("UnappliedSlash longer than the weak bound");
		}

		pallet_staking_async::UnappliedSlash {
			validator: self.validator,
			own: self.own,
			others: WeakBoundedVec::<_, T::MaxExposurePageSize>::force_from(
				self.others.into_inner(),
				None,
			),
			reporter: self.reporters.first().cloned(),
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
	Staked,
	Stash,
	Controller,
	Account(AccountId32),
	None,
}

impl TranslateAccounts for PortableRewardDestination {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		match self {
			PortableRewardDestination::Account(account) =>
				PortableRewardDestination::Account(f(account)),
			_ => self,
		}
	}
}

// RewardDestination: RC -> Portable
impl IntoPortable for pallet_staking::RewardDestination<AccountId32> {
	type Portable = PortableRewardDestination;

	#[allow(deprecated)]
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

// RewardDestination: Portable -> AH
impl Into<pallet_staking_async::RewardDestination<AccountId32>> for PortableRewardDestination {
	#[allow(deprecated)]
	fn into(self) -> pallet_staking_async::RewardDestination<AccountId32> {
		use pallet_staking_async::RewardDestination::*;
		match self {
			PortableRewardDestination::Staked => Staked,
			PortableRewardDestination::Stash => Stash,
			PortableRewardDestination::Controller => Controller,
			PortableRewardDestination::Account(account) => Account(account),
			PortableRewardDestination::None => None,
		}
	}
}

#[derive(
	PartialEqNoBound,
	EqNoBound,
	Clone,
	Encode,
	Decode,
	RuntimeDebugNoBound,
	TypeInfo,
	MaxEncodedLen,
	DecodeWithMemTracking,
)]
pub struct PortableNominations {
	pub targets: BoundedVec<AccountId32, ConstU32<32>>, // Range up to 16
	pub submitted_in: EraIndex,
	pub suppressed: bool,
}

impl TranslateAccounts for PortableNominations {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableNominations {
			targets: self.targets.into_iter().map(f).collect::<Vec<_>>().defensive_truncate_into(),
			submitted_in: self.submitted_in,
			suppressed: self.suppressed,
		}
	}
}

// Nominations: RC -> Portable
impl<T: Config> IntoPortable for pallet_staking::Nominations<T> {
	type Portable = PortableNominations;

	fn into_portable(self) -> Self::Portable {
		PortableNominations {
			targets: self.targets.into_inner().defensive_truncate_into(),
			submitted_in: self.submitted_in,
			suppressed: self.suppressed,
		}
	}
}

// Nominations: Portable -> AH
impl<T: pallet_staking_async::Config<CurrencyBalance = u128>>
	Into<pallet_staking_async::Nominations<T>> for PortableNominations
where
	<T as frame_system::Config>::AccountId: From<AccountId32>,
{
	fn into(self) -> pallet_staking_async::Nominations<T> {
		pallet_staking_async::Nominations {
			targets: self
				.targets
				.into_iter()
				.map(Into::into)
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
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
	pub total: u128,
	pub own: u128,
	pub nominator_count: u32,
	pub page_count: Page,
}

impl TranslateAccounts for PortablePagedExposureMetadata {
	fn translate_accounts(self, _f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		// No-OP
		PortablePagedExposureMetadata {
			total: self.total,
			own: self.own,
			nominator_count: self.nominator_count,
			page_count: self.page_count,
		}
	}
}

// PagedExposureMetadata: RC -> Portable
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

// PagedExposureMetadata: Portable -> AH
impl Into<sp_staking::PagedExposureMetadata<u128>> for PortablePagedExposureMetadata {
	fn into(self) -> sp_staking::PagedExposureMetadata<u128> {
		sp_staking::PagedExposureMetadata {
			total: self.total,
			own: self.own,
			nominator_count: self.nominator_count,
			page_count: self.page_count,
		}
	}
}

/// A snapshot of the stake backing a single validator in the system.
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
	DecodeWithMemTracking,
)]
pub struct PortableExposurePage {
	pub page_total: u128,
	pub others: BoundedVec<PortableIndividualExposure, ConstU32<600>>, // Range 0-512
}

impl TranslateAccounts for PortableExposurePage {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableExposurePage {
			page_total: self.page_total,
			others: self
				.others
				.into_iter()
				.map(|c| c.translate_accounts(f))
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
		}
	}
}

// ExposurePage: RC -> Portable
impl IntoPortable for sp_staking::ExposurePage<AccountId32, u128> {
	type Portable = PortableExposurePage;

	fn into_portable(self) -> Self::Portable {
		PortableExposurePage {
			page_total: self.page_total,
			others: self
				.others
				.into_iter()
				.map(IntoPortable::into_portable)
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
		}
	}
}

// ExposurePage: Portable -> AH (part 1)
impl Into<sp_staking::ExposurePage<AccountId32, u128>> for PortableExposurePage {
	fn into(self) -> sp_staking::ExposurePage<AccountId32, u128> {
		sp_staking::ExposurePage {
			page_total: self.page_total,
			others: self.others.into_iter().map(Into::into).collect::<Vec<_>>(),
		}
	}
}

// ExposurePage: Portable -> AH (part 2)
impl<
		T: pallet_staking_async::Config<CurrencyBalance = u128>
			+ frame_system::Config<AccountId = AccountId32>,
	> Into<pallet_staking_async::BoundedExposurePage<T>> for PortableExposurePage
{
	fn into(self) -> pallet_staking_async::BoundedExposurePage<T> {
		let page: sp_staking::ExposurePage<_, _> = self.into();
		pallet_staking_async::BoundedExposurePage::from(page)
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
	pub who: AccountId32,
	pub value: u128,
}

impl TranslateAccounts for PortableIndividualExposure {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableIndividualExposure { who: f(self.who), value: self.value }
	}
}

// IndividualExposure: RC -> Portable
impl IntoPortable for sp_staking::IndividualExposure<AccountId32, u128> {
	type Portable = PortableIndividualExposure;

	fn into_portable(self) -> Self::Portable {
		PortableIndividualExposure { who: self.who, value: self.value }
	}
}

// IndividualExposure: Portable -> AH
impl Into<sp_staking::IndividualExposure<AccountId32, u128>> for PortableIndividualExposure {
	fn into(self) -> sp_staking::IndividualExposure<AccountId32, u128> {
		sp_staking::IndividualExposure { who: self.who, value: self.value }
	}
}

#[derive(
	PartialEq,
	Eq,
	Clone,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	DecodeWithMemTracking,
)]
pub struct PortableEraRewardPoints {
	pub total: u32,
	// 1000 on Polkadot and 2000 on Kusama, so we just take the max.
	pub individual: BoundedVec<(AccountId32, u32), ConstU32<2000>>,
}

impl TranslateAccounts for PortableEraRewardPoints {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableEraRewardPoints {
			total: self.total,
			individual: self
				.individual
				.into_iter()
				.map(|(who, points)| (f(who), points))
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
		}
	}
}

// EraRewardPoints: RC -> Portable
impl IntoPortable for pallet_staking::EraRewardPoints<AccountId32> {
	type Portable = PortableEraRewardPoints;

	fn into_portable(self) -> Self::Portable {
		PortableEraRewardPoints {
			total: self.total,
			individual: self.individual.into_iter().collect::<Vec<_>>().defensive_truncate_into(),
		}
	}
}

// EraRewardPoints: Portable -> AH
impl<
		T: pallet_staking_async::Config<CurrencyBalance = u128>
			+ frame_system::Config<AccountId = AccountId32>,
	> Into<pallet_staking_async::EraRewardPoints<T>> for PortableEraRewardPoints
{
	fn into(self) -> pallet_staking_async::EraRewardPoints<T> {
		let individual = self
			.individual
			.into_iter()
			.take(T::MaxValidatorSet::get() as usize)
			.collect::<BTreeMap<_, _>>();
		let bounded = BoundedBTreeMap::<_, _, T::MaxValidatorSet>::try_from(individual)
			.defensive()
			.unwrap_or_default();

		pallet_staking_async::EraRewardPoints { total: self.total, individual: bounded }
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
pub struct PortableValidatorPrefs {
	pub commission: Perbill,
	pub blocked: bool,
}

impl TranslateAccounts for PortableValidatorPrefs {
	fn translate_accounts(self, _f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		// No-OP
		PortableValidatorPrefs { commission: self.commission, blocked: self.blocked }
	}
}

// ValidatorPrefs: RC -> Portable
impl IntoPortable for pallet_staking::ValidatorPrefs {
	type Portable = PortableValidatorPrefs;

	fn into_portable(self) -> Self::Portable {
		PortableValidatorPrefs { commission: self.commission, blocked: self.blocked }
	}
}

// ValidatorPrefs: Portable -> AH
impl Into<pallet_staking_async::ValidatorPrefs> for PortableValidatorPrefs {
	fn into(self) -> pallet_staking_async::ValidatorPrefs {
		pallet_staking_async::ValidatorPrefs { commission: self.commission, blocked: self.blocked }
	}
}
