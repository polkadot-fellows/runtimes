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

use codec::Encode;
use pallet_rc_migrator::{
	staking::{
		message::{PortableNominations, PortableUnappliedSlash},
		RcData,
	},
	types::{SortByEncoded, TranslateAccounts},
};
use sp_runtime::{AccountId32, Perbill, WeakBoundedVec};
use std::fmt::Debug;

impl<T: crate::Config> crate::types::AhMigrationCheck
	for pallet_rc_migrator::staking::StakingMigratedCorrectly<T>
{
	type RcPrePayload = RcData;
	type AhPrePayload = ();

	fn pre_check(_rc: Self::RcPrePayload) -> Self::AhPrePayload {}

	fn post_check(rc: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		let t = crate::Pallet::<T>::translate_account_rc_to_ah;

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
		assert_eq!(rc.max_staked_rewards, pallet_staking_async::MaxStakedRewards::<T>::get());
		assert_eq!(rc.slash_reward_fraction, pallet_staking_async::SlashRewardFraction::<T>::get());
		assert_eq!(rc.canceled_slash_payout, pallet_staking_async::CanceledSlashPayout::<T>::get());
		// Current planned session is not migrated
		assert_eq!(rc.chill_threshold, pallet_staking_async::ChillThreshold::<T>::get());

		// Storage Maps
		assert_equal_items(rc.invulnerables, pallet_staking_async::Invulnerables::<T>::get());
		assert_equal_items(
			rc.bonded.into_iter().map(|(a, b)| (t(a), t(b))),
			pallet_staking_async::Bonded::<T>::iter(),
		);
		assert_equal_items(
			rc.ledger.into_iter().map(|(k, v)| (t(k), v.translate_accounts(&t).into())),
			pallet_staking_async::Ledger::<T>::iter(),
		);
		assert_equal_items(
			rc.payee.into_iter().map(|(k, v)| (t(k), translate_reward_destination(v, &t))),
			pallet_staking_async::Payee::<T>::iter(),
		);
		assert_equal_items(
			rc.validators.into_iter().map(|(k, v)| (k, translate_validator_prefs(v))),
			pallet_staking_async::Validators::<T>::iter(),
		);
		assert_equal_items(
			rc.nominators.into_iter().map(|(k, v)| (t(k), translate_nominations(v, &t))),
			pallet_staking_async::Nominators::<T>::iter(),
		);
		assert_equal_items(
			rc.virtual_stakers.into_iter().map(t),
			pallet_staking_async::VirtualStakers::<T>::iter_keys(),
		);
		assert_equal_items(
			rc.eras_stakers_overview.into_iter().map(|(k1, k2, v)| (k1, t(k2), v.into())),
			pallet_staking_async::ErasStakersOverview::<T>::iter(),
		);
		assert_equal_items(
			rc.eras_stakers_paged
				.into_iter()
				.map(|((k0, k1, k2), v)| ((k0, t(k1), k2), v.translate_accounts(&t).into())),
			pallet_staking_async::ErasStakersPaged::<T>::iter(),
		);
		assert_equal_items(
			rc.claimed_rewards
				.into_iter()
				.map(|(k0, k1, v)| (k0, t(k1), WeakBoundedVec::force_from(v, None))),
			pallet_staking_async::ClaimedRewards::<T>::iter(),
		);
		assert_equal_items(
			rc.eras_validator_prefs.into_iter().map(|(k1, k2, v)| (k1, t(k2), v.into())),
			pallet_staking_async::ErasValidatorPrefs::<T>::iter(),
		);
		assert_equal_items(
			rc.eras_validator_reward,
			pallet_staking_async::ErasValidatorReward::<T>::iter(),
		);
		assert_equal_items(
			rc.eras_reward_points
				.into_iter()
				.map(|(k, v)| (k, v.translate_accounts(&t).into())),
			pallet_staking_async::ErasRewardPoints::<T>::iter(),
		);
		assert_equal_items(rc.eras_total_stake, pallet_staking_async::ErasTotalStake::<T>::iter());
		check_unapplied_slashes::<T>(rc.unapplied_slashes, &t);
		assert_equal_items(rc.bonded_eras, pallet_staking_async::BondedEras::<T>::get());
		assert_equal_items(
			rc.validator_slash_in_era.into_iter().map(|(k0, k1, v)| (k0, t(k1), v)),
			pallet_staking_async::ValidatorSlashInEra::<T>::iter(),
		);
	}
}

#[allow(deprecated)]
fn translate_reward_destination(
	destination: pallet_staking::RewardDestination<AccountId32>,
	t: &impl Fn(AccountId32) -> AccountId32,
) -> pallet_staking_async::RewardDestination<AccountId32> {
	use pallet_staking_async::RewardDestination::*;

	match destination {
		pallet_staking::RewardDestination::Staked => Staked,
		pallet_staking::RewardDestination::Stash => Stash,
		pallet_staking::RewardDestination::Controller => Controller,
		pallet_staking::RewardDestination::Account(account) => Account(t(account)),
		pallet_staking::RewardDestination::None => None,
	}
}

fn translate_active_era(era: pallet_staking::ActiveEraInfo) -> pallet_staking_async::ActiveEraInfo {
	pallet_staking_async::ActiveEraInfo { index: era.index, start: era.start }
}

fn translate_validator_prefs(
	prefs: pallet_staking::ValidatorPrefs,
) -> pallet_staking_async::ValidatorPrefs {
	pallet_staking_async::ValidatorPrefs { commission: prefs.commission, blocked: prefs.blocked }
}

fn translate_nominations<T: crate::Config>(
	nominations: PortableNominations,
	t: &impl Fn(AccountId32) -> AccountId32,
) -> pallet_staking_async::Nominations<T> {
	pallet_staking_async::Nominations {
		targets: nominations
			.targets
			.into_inner()
			.into_iter()
			.map(t)
			.collect::<Vec<_>>()
			.try_into()
			.expect("Must not truncate"),
		submitted_in: nominations.submitted_in,
		suppressed: nominations.suppressed,
	}
}

fn check_unapplied_slashes<T: crate::Config>(
	rc: Vec<(u32, Vec<PortableUnappliedSlash>)>,
	t: &impl Fn(AccountId32) -> AccountId32,
) {
	let mut expected_slashes = Vec::new();

	for (era, slashes) in rc {
		for slash in slashes {
			// We insert all slashes with this special key
			let key = (t(slash.clone().validator), Perbill::from_percent(99), 9999);
			expected_slashes.push((era, key, slash.translate_accounts(t).into()));
		}
	}

	assert_equal_items(expected_slashes, pallet_staking_async::UnappliedSlashes::<T>::iter());
}

/// Assert that two iterators have the same elements, regardless of their order.
fn assert_equal_items<
	V: Encode + PartialEq + Debug,
	I: IntoIterator<Item = V>,
	J: IntoIterator<Item = V>,
>(
	rc: I,
	ah: J,
) {
	let mut rc: Vec<V> = rc.into_iter().collect::<Vec<_>>();
	rc.sort_by_encoded();
	let mut ah: Vec<V> = ah.into_iter().collect::<Vec<_>>();
	ah.sort_by_encoded();

	for (i, (r, a)) in rc.iter().zip(ah.iter()).enumerate() {
		assert_eq!(r, a, "Entry #{i} mismatch: {r:?} != {a:?}");
	}
}
