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

use pallet_rc_migrator::{
	staking::{
		message::{PortableNominations, PortableUnappliedSlash},
		RcData,
	},
};
use sp_runtime::{AccountId32, Perbill};

impl<T: crate::Config> crate::types::AhMigrationCheck
	for pallet_rc_migrator::staking::StakingMigratedCorrectly<T>
{
	type RcPrePayload = RcData;
	type AhPrePayload = ();

	fn pre_check(_rc: Self::RcPrePayload) -> Self::AhPrePayload {}

	fn post_check(rc: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		// Storage Values
		assert_eq!(rc.validator_count, pallet_staking_async::ValidatorCount::<T>::get());
		// Min validator count is not migrated and instead configured via `MinimumValidatorSetSize`
		assert_eq!(rc.min_nominator_bond, pallet_staking_async::MinNominatorBond::<T>::get());
		assert_eq!(rc.min_validator_bond, pallet_staking_async::MinValidatorBond::<T>::get());
		assert_eq!(rc.min_active_stake, pallet_staking_async::MinimumActiveStake::<T>::get());
		assert_eq!(rc.min_commission, pallet_staking_async::MinCommission::<T>::get());
		assert_eq!(rc.max_validators_count, pallet_staking_async::MaxValidatorsCount::<T>::get());
		assert_eq!(rc.max_nominators_count, pallet_staking_async::MaxNominatorsCount::<T>::get());
		assert_eq!(
			pallet_staking_async::CurrentEra::<T>::get().expect("Must be set"),
			pallet_staking_async::ActiveEra::<T>::get().expect("Must be set").index
		);
		assert_eq!(
			rc.active_era.map(translate_active_era),
			pallet_staking_async::ActiveEra::<T>::get()
		);
		assert_eq!(translate_forcing(rc.force_era), pallet_staking_async::ForceEra::<T>::get());
		assert_eq!(rc.max_staked_rewards, pallet_staking_async::MaxStakedRewards::<T>::get());
		assert_eq!(rc.slash_reward_fraction, pallet_staking_async::SlashRewardFraction::<T>::get());
		assert_eq!(rc.canceled_slash_payout, pallet_staking_async::CanceledSlashPayout::<T>::get());
		// Current planned session is not migrated
		assert_eq!(rc.chill_threshold, pallet_staking_async::ChillThreshold::<T>::get());

		// Storage Maps
		assert_eq!(rc.invulnerables, pallet_staking_async::Invulnerables::<T>::get().into_inner());
		assert_eq!(rc.bonded, pallet_staking_async::Bonded::<T>::iter().collect::<Vec<_>>());
		assert_eq!(
			rc.ledger.into_iter().map(|(k, v)| (k, v.into())).collect::<Vec<_>>(),
			pallet_staking_async::Ledger::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.payee
				.into_iter()
				.map(|(k, v)| (k, translate_reward_destination(v)))
				.collect::<Vec<_>>(),
			pallet_staking_async::Payee::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.validators
				.into_iter()
				.map(|(k, v)| (k, translate_validator_prefs(v)))
				.collect::<Vec<_>>(),
			pallet_staking_async::Validators::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.nominators
				.into_iter()
				.map(|(k, v)| (k, translate_nominations(v)))
				.collect::<Vec<_>>(),
			pallet_staking_async::Nominators::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.virtual_stakers,
			pallet_staking_async::VirtualStakers::<T>::iter_keys().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.eras_stakers_overview
				.into_iter()
				.map(|(k1, k2, v)| (k1, k2, v.into()))
				.collect::<Vec<_>>(),
			pallet_staking_async::ErasStakersOverview::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.eras_stakers_paged
				.into_iter()
				.map(|(k, v)| (k, v.into()))
				.collect::<Vec<_>>(),
			pallet_staking_async::ErasStakersPaged::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.claimed_rewards,
			pallet_staking_async::ClaimedRewards::<T>::iter()
				.map(|(k1, k2, v)| (k1, k2, v.into_inner()))
				.collect::<Vec<_>>()
		);
		assert_eq!(
			rc.eras_validator_prefs
				.into_iter()
				.map(|(k1, k2, v)| (k1, k2, v.into()))
				.collect::<Vec<_>>(),
			pallet_staking_async::ErasValidatorPrefs::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.eras_validator_reward,
			pallet_staking_async::ErasValidatorReward::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.eras_reward_points
				.into_iter()
				.map(|(k, v)| (k, v.into()))
				.collect::<Vec<_>>(),
			pallet_staking_async::ErasRewardPoints::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.eras_total_stake,
			pallet_staking_async::ErasTotalStake::<T>::iter().collect::<Vec<_>>()
		);
		check_unapplied_slashes::<T>(rc.unapplied_slashes);
		assert_eq!(rc.bonded_eras, pallet_staking_async::BondedEras::<T>::get().into_inner());
		assert_eq!(
			rc.validator_slash_in_era,
			pallet_staking_async::ValidatorSlashInEra::<T>::iter().collect::<Vec<_>>()
		);
	}
}

#[allow(deprecated)]
fn translate_reward_destination(
	destination: pallet_staking::RewardDestination<AccountId32>,
) -> pallet_staking_async::RewardDestination<AccountId32> {
	use pallet_staking_async::RewardDestination::*;

	match destination {
		pallet_staking::RewardDestination::Staked => Staked,
		pallet_staking::RewardDestination::Stash => Stash,
		pallet_staking::RewardDestination::Controller => Controller,
		pallet_staking::RewardDestination::Account(account) => Account(account),
		pallet_staking::RewardDestination::None => None,
	}
}

fn translate_active_era(era: pallet_staking::ActiveEraInfo) -> pallet_staking_async::ActiveEraInfo {
	pallet_staking_async::ActiveEraInfo { index: era.index, start: era.start }
}

fn translate_forcing(forcing: pallet_staking::Forcing) -> pallet_staking_async::Forcing {
	use pallet_staking_async::Forcing;
	match forcing {
		pallet_staking::Forcing::NotForcing => Forcing::NotForcing,
		pallet_staking::Forcing::ForceNew => Forcing::ForceNew,
		pallet_staking::Forcing::ForceNone => Forcing::ForceNone,
		pallet_staking::Forcing::ForceAlways => Forcing::ForceAlways,
	}
}

fn translate_validator_prefs(
	prefs: pallet_staking::ValidatorPrefs,
) -> pallet_staking_async::ValidatorPrefs {
	pallet_staking_async::ValidatorPrefs { commission: prefs.commission, blocked: prefs.blocked }
}

fn translate_nominations<T: crate::Config>(
	nominations: PortableNominations,
) -> pallet_staking_async::Nominations<T> {
	pallet_staking_async::Nominations {
		targets: nominations.targets.into_inner().try_into().expect("Must not truncate"),
		submitted_in: nominations.submitted_in,
		suppressed: nominations.suppressed,
	}
}

fn check_unapplied_slashes<T: crate::Config>(rc: Vec<(u32, Vec<PortableUnappliedSlash>)>) {
	let mut expected_slashes =
		Vec::<(u32, (AccountId32, Perbill, u32), PortableUnappliedSlash)>::new();

	for (era, slashes) in rc {
		for slash in slashes {
			// We insert all slashes with this special key
			let key = (slash.validator.clone(), Perbill::from_percent(99), 9999);
			expected_slashes.push((era, key, slash));
		}
	}

	// TODO assert
}
