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

//! Checks that the staking migration succeeded.

use crate::{
	staking::{
		message::{
			PortableEraRewardPoints, PortableExposurePage, PortableForcing, PortableNominations,
			PortablePagedExposureMetadata, PortableStakingLedger, PortableUnappliedSlash,
			PortableValidatorPrefs, StakingValues,
		},
		PortableStakingMessage,
	},
	types::IntoPortable,
	BalanceOf,
};
use pallet_staking::Pallet as Staking;
use sp_runtime::{AccountId32, Perbill, Percent};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RcData {
	// Storage Values
	pub validator_count: u32,
	pub min_validator_count: u32,
	pub min_nominator_bond: u128,
	pub min_validator_bond: u128,
	pub min_active_stake: u128,
	pub min_commission: Perbill,
	pub max_validators_count: Option<u32>,
	pub max_nominators_count: Option<u32>,
	pub current_era: Option<u32>,
	pub active_era: Option<pallet_staking::ActiveEraInfo>,
	pub force_era: pallet_staking::Forcing,
	pub max_staked_rewards: Option<Percent>,
	pub slash_reward_fraction: Perbill,
	pub canceled_slash_payout: u128,
	pub current_planned_session: u32,
	pub chill_threshold: Option<Percent>,
	// Storage Maps
	pub invulnerables: Vec<AccountId32>,
	pub bonded: Vec<(AccountId32, AccountId32)>,
	pub ledger: Vec<(AccountId32, PortableStakingLedger)>,
	pub payee: Vec<(AccountId32, pallet_staking::RewardDestination<AccountId32>)>,
	pub validators: Vec<(AccountId32, pallet_staking::ValidatorPrefs)>,
	pub nominators: Vec<(AccountId32, PortableNominations)>,
	pub virtual_stakers: Vec<AccountId32>,
	pub eras_stakers_overview: Vec<(u32, AccountId32, PortablePagedExposureMetadata)>,
	pub eras_stakers_paged: Vec<((u32, AccountId32, u32), PortableExposurePage)>,
	pub claimed_rewards: Vec<(u32, AccountId32, Vec<u32>)>,
	pub eras_validator_prefs: Vec<(u32, AccountId32, PortableValidatorPrefs)>,
	pub eras_validator_reward: Vec<(u32, u128)>,
	pub eras_reward_points: Vec<(u32, PortableEraRewardPoints)>,
	pub eras_total_stake: Vec<(u32, u128)>,
	pub unapplied_slashes: Vec<(u32, Vec<PortableUnappliedSlash>)>,
	pub bonded_eras: Vec<(u32, u32)>,
	pub validator_slash_in_era: Vec<(u32, AccountId32, (Perbill, u128))>,
}

pub struct StakingMigratedCorrectly<T>(pub core::marker::PhantomData<T>);

impl<T: crate::Config> crate::types::RcMigrationCheck for StakingMigratedCorrectly<T> {
	type RcPrePayload = RcData;

	fn pre_check() -> Self::RcPrePayload {
		RcData {
			// Storage Values
			validator_count: pallet_staking::ValidatorCount::<T>::get(),
			min_validator_count: pallet_staking::MinimumValidatorCount::<T>::get(),
			min_nominator_bond: pallet_staking::MinNominatorBond::<T>::get(),
			min_validator_bond: pallet_staking::MinValidatorBond::<T>::get(),
			min_active_stake: pallet_staking::MinimumActiveStake::<T>::get(),
			min_commission: pallet_staking::MinCommission::<T>::get(),
			max_validators_count: pallet_staking::MaxValidatorsCount::<T>::get(),
			max_nominators_count: pallet_staking::MaxNominatorsCount::<T>::get(),
			current_era: pallet_staking::CurrentEra::<T>::get(),
			active_era: pallet_staking::ActiveEra::<T>::get(),
			force_era: pallet_staking::ForceEra::<T>::get(),
			max_staked_rewards: pallet_staking::MaxStakedRewards::<T>::get(),
			slash_reward_fraction: pallet_staking::SlashRewardFraction::<T>::get(),
			canceled_slash_payout: pallet_staking::CanceledSlashPayout::<T>::get(),
			current_planned_session: pallet_staking::CurrentPlannedSession::<T>::get(),
			chill_threshold: pallet_staking::ChillThreshold::<T>::get(),

			// Storage Maps
			invulnerables: pallet_staking::Invulnerables::<T>::get(),
			bonded: pallet_staking::Bonded::<T>::iter().collect(),
			ledger: pallet_staking::Ledger::<T>::iter()
				.map(|(k, v)| (k, v.into_portable()))
				.collect(),
			payee: pallet_staking::Payee::<T>::iter().collect(),
			validators: pallet_staking::Validators::<T>::iter().collect(),
			nominators: pallet_staking::Nominators::<T>::iter()
				.map(|(k, v)| (k, v.into_portable()))
				.collect(),
			virtual_stakers: pallet_staking::VirtualStakers::<T>::iter_keys().collect(),
			eras_stakers_overview: pallet_staking::ErasStakersOverview::<T>::iter()
				.map(|(k1, k2, v)| (k1, k2, v.into_portable()))
				.collect(),
			eras_stakers_paged: pallet_staking::ErasStakersPaged::<T>::iter()
				.map(|(k, v)| (k, v.into_portable()))
				.collect(),
			claimed_rewards: pallet_staking::ClaimedRewards::<T>::iter().collect(),
			eras_validator_prefs: pallet_staking::ErasValidatorPrefs::<T>::iter()
				.map(|(k1, k2, v)| (k1, k2, v.into_portable()))
				.collect(),
			eras_validator_reward: pallet_staking::ErasValidatorReward::<T>::iter().collect(),
			eras_reward_points: pallet_staking::ErasRewardPoints::<T>::iter()
				.map(|(k, v)| (k, v.into_portable()))
				.collect(),
			eras_total_stake: pallet_staking::ErasTotalStake::<T>::iter().collect(),
			unapplied_slashes: pallet_staking::UnappliedSlashes::<T>::iter()
				.map(|(k, v)| (k, v.into_iter().map(IntoPortable::into_portable).collect()))
				.collect(),
			bonded_eras: pallet_staking::BondedEras::<T>::get(),
			validator_slash_in_era: pallet_staking::ValidatorSlashInEra::<T>::iter().collect(),
		}
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload) {
		use pallet_staking::*;
		// All storage values are gone
		assert!(!ValidatorCount::<T>::exists());
		assert!(!MinimumValidatorCount::<T>::exists());
		assert!(!MinNominatorBond::<T>::exists());
		assert!(!MinValidatorBond::<T>::exists());
		assert!(!MinimumActiveStake::<T>::exists());
		assert!(!MinCommission::<T>::exists());
		assert!(!MaxValidatorsCount::<T>::exists());
		assert!(!MaxNominatorsCount::<T>::exists());
		assert!(!CurrentEra::<T>::exists());
		assert!(!ActiveEra::<T>::exists());
		assert!(!ForceEra::<T>::exists());
		assert!(!MaxStakedRewards::<T>::exists());
		assert!(!SlashRewardFraction::<T>::exists());
		assert!(!CanceledSlashPayout::<T>::exists());
		assert!(!CurrentPlannedSession::<T>::exists());
		assert!(!ChillThreshold::<T>::exists());

		assert!(!Invulnerables::<T>::exists());
		assert!(Bonded::<T>::iter_keys().next().is_none());
		assert!(Ledger::<T>::iter_keys().next().is_none());
		assert!(Payee::<T>::iter_keys().next().is_none());
		assert!(Validators::<T>::iter_keys().next().is_none());
		assert!(Nominators::<T>::iter_keys().next().is_none());
		assert!(VirtualStakers::<T>::iter_keys().next().is_none());
		assert!(ErasStakersOverview::<T>::iter_keys().next().is_none());
		assert!(ErasStakersPaged::<T>::iter_keys().next().is_none());
		assert!(ClaimedRewards::<T>::iter_keys().next().is_none());
		assert!(ErasValidatorPrefs::<T>::iter_keys().next().is_none());
		assert!(ErasValidatorReward::<T>::iter_keys().next().is_none());
		assert!(ErasRewardPoints::<T>::iter_keys().next().is_none());
		assert!(ErasTotalStake::<T>::iter_keys().next().is_none());
		assert!(UnappliedSlashes::<T>::iter_keys().next().is_none());
		assert!(!BondedEras::<T>::exists());
		assert!(ValidatorSlashInEra::<T>::iter_keys().next().is_none());
		assert!(NominatorSlashInEra::<T>::iter_keys().next().is_none());
		assert!(SlashingSpans::<T>::iter_keys().next().is_none());
		assert!(SpanSlash::<T>::iter_keys().next().is_none());
	}
}
