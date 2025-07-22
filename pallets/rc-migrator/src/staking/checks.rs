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
		message::{PortableForcing, PortableNominations, PortableStakingLedger, StakingValues},
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
		}
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload) {}
}
