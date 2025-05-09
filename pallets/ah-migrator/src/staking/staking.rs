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

//! Pallet staking migration.

use crate::*;
use frame_support::traits::DefensiveTruncateInto;
use sp_runtime::Perbill;

impl<T: Config> Pallet<T> {
	pub fn staking_migration_start_hook() {}

	pub fn staking_migration_finish_hook() {}

	pub fn do_receive_staking_messages(messages: Vec<T::RcStakingMessage>) -> Result<(), Error<T>> {
		let (mut good, mut bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} StakingMessages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Staking,
			count: messages.len() as u32,
		});

		for message in messages {
			let translated = T::RcStakingMessage::intoAh(message);
			match Self::do_receive_staking_message(translated) {
				Ok(_) => good += 1,
				Err(_) => bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Staking,
			count_good: good as u32,
			count_bad: bad as u32,
		});

		Ok(())
	}

	fn do_receive_staking_message(
		message: AhEquivalentStakingMessageOf<T>,
	) -> Result<(), Error<T>> {
		use RcStakingMessage::*;

		match message {
			Values(values) => {
				log::debug!(target: LOG_TARGET, "Integrating StakingValues");
				pallet_rc_migrator::staking::StakingMigrator::<T>::put_values(values);
			},
			Invulnerables(invulnerables) => {
				log::debug!(target: LOG_TARGET, "Integrating StakingInvulnerables");
				let bounded: BoundedVec<_, _> = invulnerables.defensive_truncate_into();
				pallet_staking_async::Invulnerables::<T>::put(bounded);
			},
			Bonded { stash, controller } => {
				log::debug!(target: LOG_TARGET, "Integrating Bonded of stash {:?}", stash);
				pallet_staking_async::Bonded::<T>::insert(stash, controller);
			},
			Ledger { controller, ledger } => {
				log::debug!(target: LOG_TARGET, "Integrating Ledger of controller {:?}", controller);
				pallet_staking_async::Ledger::<T>::insert(controller, ledger);
			},
			Payee { stash, payment } => {
				log::debug!(target: LOG_TARGET, "Integrating Payee of stash {:?}", stash);
				pallet_staking_async::Payee::<T>::insert(stash, payment);
			},
			Validators { stash, validators } => {
				log::debug!(target: LOG_TARGET, "Integrating Validators of stash {:?}", stash);
				pallet_staking_async::Validators::<T>::insert(stash, validators);
			},
			Nominators { stash, nominations } => {
				log::debug!(target: LOG_TARGET, "Integrating Nominators of stash {:?}", stash);
				pallet_staking_async::Nominators::<T>::insert(stash, nominations);
			},
			VirtualStakers(staker) => {
				log::debug!(target: LOG_TARGET, "Integrating VirtualStakers of staker {:?}", staker);
				pallet_staking_async::VirtualStakers::<T>::insert(staker, ());
			},
			ErasStartSessionIndex { era, session } => {
				log::debug!(target: LOG_TARGET, "Integrating ErasStartSessionIndex {:?}/{:?}", era, session);
				pallet_staking_async::ErasStartSessionIndex::<T>::insert(era, session);
			},
			ErasStakersOverview { era, validator, exposure } => {
				log::debug!(target: LOG_TARGET, "Integrating ErasStakersOverview {:?}/{:?}", validator, era);
				pallet_staking_async::ErasStakersOverview::<T>::insert(era, validator, exposure);
			},
			ErasStakersPaged { era, validator, page, exposure } => {
				log::debug!(target: LOG_TARGET, "Integrating ErasStakersPaged {:?}/{:?}/{:?}", validator, era, page);
				pallet_staking_async::ErasStakersPaged::<T>::insert(
					(era, validator, page),
					exposure,
				);
			},
			ClaimedRewards { era, validator, rewards } => {
				// NOTE: This is being renamed from `ClaimedRewards` to `ErasClaimedRewards`
				log::debug!(target: LOG_TARGET, "Integrating ErasClaimedRewards {:?}/{:?}", validator, era);
				pallet_staking_async::ErasClaimedRewards::<T>::insert(era, validator, rewards);
			},
			ErasValidatorPrefs { era, validator, prefs } => {
				log::debug!(target: LOG_TARGET, "Integrating ErasValidatorPrefs {:?}/{:?}", validator, era);
				pallet_staking_async::ErasValidatorPrefs::<T>::insert(era, validator, prefs);
			},
			ErasValidatorReward { era, reward } => {
				log::debug!(target: LOG_TARGET, "Integrating ErasValidatorReward of era {:?}", era);
				pallet_staking_async::ErasValidatorReward::<T>::insert(era, reward);
			},
			ErasRewardPoints { era, points } => {
				log::debug!(target: LOG_TARGET, "Integrating ErasRewardPoints of era {:?}", era);
				pallet_staking_async::ErasRewardPoints::<T>::insert(era, points);
			},
			ErasTotalStake { era, total_stake } => {
				log::debug!(target: LOG_TARGET, "Integrating ErasTotalStake of era {:?}", era);
				pallet_staking_async::ErasTotalStake::<T>::insert(era, total_stake);
			},
			UnappliedSlashes { era, slash } => {
				log::debug!(target: LOG_TARGET, "Integrating UnappliedSlashes of era {:?}", era);
				let slash_key = (slash.validator.clone(), Perbill::from_percent(99), 9999);
				pallet_staking_async::UnappliedSlashes::<T>::insert(era, slash_key, slash);
			},
			BondedEras(bonded_eras) => {
				log::debug!(target: LOG_TARGET, "Integrating BondedEras");
				pallet_staking_async::BondedEras::<T>::put(bonded_eras);
			},
			ValidatorSlashInEra { era, validator, slash } => {
				log::debug!(target: LOG_TARGET, "Integrating ValidatorSlashInEra {:?}/{:?}", validator, era);
				pallet_staking_async::ValidatorSlashInEra::<T>::insert(era, validator, slash);
			},
			NominatorSlashInEra { era, validator, slash } => {
				log::debug!(target: LOG_TARGET, "Integrating NominatorSlashInEra {:?}/{:?}", validator, era);
				pallet_staking_async::NominatorSlashInEra::<T>::insert(era, validator, slash);
			},
			SlashingSpans { account, spans } => {
				log::debug!(target: LOG_TARGET, "Integrating SlashingSpans {:?}", account);
				pallet_staking_async::SlashingSpans::<T>::insert(account, spans);
			},
			SpanSlash { account, span, slash } => {
				log::debug!(target: LOG_TARGET, "Integrating SpanSlash {:?}/{:?}", account, span);
				pallet_staking_async::SpanSlash::<T>::insert((account, span), slash);
			},
		}

		Ok(())
	}
}

#[cfg(all(feature = "std", feature = "ahm-staking-migration"))]
impl<T: Config> crate::types::AhMigrationCheck for pallet_rc_migrator::staking::StakingMigrator<T> {
	
	type RcPrePayload = Vec<T::RcStakingMessage>;
	type AhPrePayload = ();

	fn pre_check(_rc_pre_payload: Self::RcPrePayload) -> Self::AhPrePayload {
        // "Assert storage 'StakingAsync::ValidatorCount::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::ValidatorCount::<T>::get(),
            0,
            "StakingAsync::ValidatorCount should be 0 on AH before migration"
        );

        // "Assert storage 'StakingAsync::MinNominatorBond::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::MinNominatorBond::<T>::get(),
            Default::default(),
            "StakingAsync::MinNominatorBond should be default on AH before migration"
        );

        // "Assert storage 'StakingAsync::MinValidatorBond::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::MinValidatorBond::<T>::get(),
            Default::default(),
            "StakingAsync::MinValidatorBond should be default on AH before migration"
        );

        // "Assert storage 'StakingAsync::MinimumActiveStake::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::MinimumActiveStake::<T>::get(),
            Default::default(),
            "StakingAsync::MinimumActiveStake should be default on AH before migration"
        );

        // "Assert storage 'StakingAsync::MinCommission::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::MinCommission::<T>::get(),
            Default::default(),
            "StakingAsync::MinCommission should be default on AH before migration"
        );

        // "Assert storage 'StakingAsync::MaxValidatorsCount::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::MaxValidatorsCount::<T>::get(),
            None,
            "StakingAsync::MaxValidatorsCount should be None on AH before migration"
        );

        // "Assert storage 'StakingAsync::MaxNominatorsCount::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::MaxNominatorsCount::<T>::get(),
            None,
            "StakingAsync::MaxNominatorsCount should be None on AH before migration"
        );

        // "Assert storage 'StakingAsync::CurrentEra::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::CurrentEra::<T>::get(),
            None,
            "StakingAsync::CurrentEra should be None on AH before migration"
        );

        // "Assert storage 'StakingAsync::ActiveEra::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::ActiveEra::<T>::get(),
            None,
            "StakingAsync::ActiveEra should be None on AH before migration"
        );

        // "Assert storage 'StakingAsync::ForceEra::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::ForceEra::<T>::get(),
            Default::default(), // Assumes Forcing::NotForcing or similar is default
            "StakingAsync::ForceEra should be default on AH before migration"
        );

        // "Assert storage 'StakingAsync::MaxStakedRewards::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::MaxStakedRewards::<T>::get(),
            None,
            "StakingAsync::MaxStakedRewards should be None on AH before migration"
        );

        // "Assert storage 'StakingAsync::SlashRewardFraction::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::SlashRewardFraction::<T>::get(),
            Default::default(),
            "StakingAsync::SlashRewardFraction should be default on AH before migration"
        );

        // "Assert storage 'StakingAsync::CanceledSlashPayout::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::CanceledSlashPayout::<T>::get(),
            Default::default(),
            "StakingAsync::CanceledSlashPayout should be default on AH before migration"
        );

        // "Assert storage 'StakingAsync::ChillThreshold::ah_pre::empty'"
        assert_eq!(
            pallet_staking_async::ChillThreshold::<T>::get(),
            None,
            "StakingAsync::ChillThreshold should be None on AH before migration"
        );

        // "Assert storage 'StakingAsync::Invulnerables::ah_pre::empty'"
        assert!(
            pallet_staking_async::Invulnerables::<T>::get().is_empty(),
            "StakingAsync::Invulnerables should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::BondedEras::ah_pre::empty'"
        assert!(
            pallet_staking_async::BondedEras::<T>::get().is_empty(),
            "StakingAsync::BondedEras should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::Bonded::ah_pre::empty'"
        assert!(
            pallet_staking_async::Bonded::<T>::iter().next().is_none(),
            "StakingAsync::Bonded map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::Ledger::ah_pre::empty'"
        assert!(
            pallet_staking_async::Ledger::<T>::iter().next().is_none(),
            "StakingAsync::Ledger map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::Payee::ah_pre::empty'"
        assert!(
            pallet_staking_async::Payee::<T>::iter().next().is_none(),
            "StakingAsync::Payee map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::Validators::ah_pre::empty'"
        assert!(
            pallet_staking_async::Validators::<T>::iter().next().is_none(),
            "StakingAsync::Validators map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::Nominators::ah_pre::empty'"
        assert!(
            pallet_staking_async::Nominators::<T>::iter().next().is_none(),
            "StakingAsync::Nominators map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::VirtualStakers::ah_pre::empty'"
        assert!(
            pallet_staking_async::VirtualStakers::<T>::iter().next().is_none(),
            "StakingAsync::VirtualStakers map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasStartSessionIndex::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasStartSessionIndex::<T>::iter().next().is_none(),
            "StakingAsync::ErasStartSessionIndex map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasStakersOverview::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasStakersOverview::<T>::iter().next().is_none(),
            "StakingAsync::ErasStakersOverview map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasStakersPaged::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasStakersPaged::<T>::iter().next().is_none(),
            "StakingAsync::ErasStakersPaged map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasClaimedRewards::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasClaimedRewards::<T>::iter().next().is_none(),
            "StakingAsync::ErasClaimedRewards map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasValidatorPrefs::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasValidatorPrefs::<T>::iter().next().is_none(),
            "StakingAsync::ErasValidatorPrefs map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasValidatorReward::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasValidatorReward::<T>::iter().next().is_none(),
            "StakingAsync::ErasValidatorReward map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasRewardPoints::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasRewardPoints::<T>::iter().next().is_none(),
            "StakingAsync::ErasRewardPoints map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ErasTotalStake::ah_pre::empty'"
        assert!(
            pallet_staking_async::ErasTotalStake::<T>::iter().next().is_none(),
            "StakingAsync::ErasTotalStake map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::UnappliedSlashes::ah_pre::empty'"
        assert!(
            pallet_staking_async::UnappliedSlashes::<T>::iter().next().is_none(),
            "StakingAsync::UnappliedSlashes map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::ValidatorSlashInEra::ah_pre::empty'"
        assert!(
            pallet_staking_async::ValidatorSlashInEra::<T>::iter().next().is_none(),
            "StakingAsync::ValidatorSlashInEra map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::NominatorSlashInEra::ah_pre::empty'"
        assert!(
            pallet_staking_async::NominatorSlashInEra::<T>::iter().next().is_none(),
            "StakingAsync::NominatorSlashInEra map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::SlashingSpans::ah_pre::empty'"
        assert!(
            pallet_staking_async::SlashingSpans::<T>::iter().next().is_none(),
            "StakingAsync::SlashingSpans map should be empty on AH before migration"
        );

        // "Assert storage 'StakingAsync::SpanSlash::ah_pre::empty'"
        assert!(
            pallet_staking_async::SpanSlash::<T>::iter().next().is_none(),
            "StakingAsync::SpanSlash map should be empty on AH before migration"
        );
    }

	fn post_check(rc_pre_payload: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
        use sp_staking::{EraIndex, Page, SessionIndex, 
            // SpanIndex,
        };
        use sp_runtime::{Perbill, Percent};
        use std::collections::{BTreeMap, HashSet};
        use frame_support::BoundedVec;

        type AccountId<T> = <T as frame_system::Config>::AccountId;
        type Balance<T> = <T as pallet_staking_async::Config>::CurrencyBalance;
        type StakingLedgerAsync<T> = pallet_staking_async::StakingLedger<T>;
        type NominationsAsync<T> = pallet_staking_async::Nominations<T>;
        type SpanRecordAsync<T> = pallet_staking_async::slashing::SpanRecord<Balance<T>>;
        type EraRewardPointsAsync<T> = pallet_staking_async::EraRewardPoints<T>;
        type RewardDestinationAsync<T> = pallet_staking_async::RewardDestination<AccountId<T>>;
        type ValidatorPrefsAsync = pallet_staking_async::ValidatorPrefs;
        type UnappliedSlashAsync<T> = pallet_staking_async::UnappliedSlash<T>;
        type SlashingSpansAsync = pallet_staking_async::slashing::SlashingSpans;
        type PagedExposureMetadataAsync<T> = sp_staking::PagedExposureMetadata<Balance<T>>;
        type ExposurePageAsync<T> = pallet_staking_async::BoundedExposurePage<T>;
        type AhStakingValues<T> = pallet_rc_migrator::staking::message::AhStakingValuesOf<T>;
        type ActiveEraInfoAsync = pallet_staking_async::ActiveEraInfo;
        type ForcingAsync = pallet_staking_async::Forcing;

    //     let mut expected_values_opt: Option<AhStakingValues<T>> = None;
    //     let mut expected_invulnerables: Vec<AccountId<T>> = Vec::new();
    //     let mut expected_bonded: BTreeMap<AccountId<T>, AccountId<T>> = BTreeMap::new();
    //     let mut expected_ledger: BTreeMap<AccountId<T>, StakingLedgerAsync<T>> = BTreeMap::new();
    //     let mut expected_payee: BTreeMap<AccountId<T>, RewardDestinationAsync<T>> = BTreeMap::new();
    //     let mut expected_validators: BTreeMap<AccountId<T>, ValidatorPrefsAsync> = BTreeMap::new();
    //     let mut expected_nominators: BTreeMap<AccountId<T>, NominationsAsync<T>> = BTreeMap::new();
    //     let mut expected_virtual_stakers: HashSet<AccountId<T>> = HashSet::new();
    //     let mut expected_eras_start_session_index: BTreeMap<EraIndex, SessionIndex> = BTreeMap::new();
    //     let mut expected_eras_stakers_overview: BTreeMap<(EraIndex, AccountId<T>), PagedExposureMetadataAsync<T>> = BTreeMap::new();
    //     let mut expected_eras_stakers_paged: BTreeMap<(EraIndex, AccountId<T>, Page), ExposurePageAsync<T>> = BTreeMap::new();
    //     let mut expected_claimed_rewards: BTreeMap<(EraIndex, AccountId<T>), Vec<Page>> = BTreeMap::new();
    //     let mut expected_eras_validator_prefs: BTreeMap<(EraIndex, AccountId<T>), ValidatorPrefsAsync> = BTreeMap::new();
    //     let mut expected_eras_validator_reward: BTreeMap<EraIndex, Balance<T>> = BTreeMap::new();
    //     let mut expected_eras_reward_points: BTreeMap<EraIndex, EraRewardPointsAsync<T>> = BTreeMap::new();
    //     let mut expected_eras_total_stake: BTreeMap<EraIndex, Balance<T>> = BTreeMap::new();
    //     let mut expected_unapplied_slashes: BTreeMap<(EraIndex, (AccountId<T>, Perbill, u32)), UnappliedSlashAsync<T>> = BTreeMap::new();
    //     let mut expected_bonded_eras: Vec<(EraIndex, SessionIndex)> = Vec::new();
    //     let mut expected_validator_slash_in_era: BTreeMap<(EraIndex, AccountId<T>), (Perbill, Balance<T>)> = BTreeMap::new();
    //     let mut expected_nominator_slash_in_era: BTreeMap<(EraIndex, AccountId<T>), Balance<T>> = BTreeMap::new();
    //     let mut expected_slashing_spans: BTreeMap<AccountId<T>, SlashingSpansAsync> = BTreeMap::new();
    // //     let mut expected_span_slash: BTreeMap<(AccountId<T>, SpanIndex), SpanRecordAsync<T>> = BTreeMap::new();

    //     for rc_message in rc_pre_payload {
    //         let ah_message = T::RcStakingMessage::intoAh(rc_message);
    //         use pallet_rc_migrator::staking::message::RcStakingMessage::*;
    //         match ah_message {
    //             Values(v) => expected_values_opt = Some(v),
    //             Invulnerables(inv) => expected_invulnerables = inv,
    //             Bonded { stash, controller } => { expected_bonded.insert(stash, controller); },
    //             Ledger { controller, ledger } => { expected_ledger.insert(controller, ledger); },
    //             Payee { stash, payment } => { expected_payee.insert(stash, payment); },
    //             Validators { stash, validators } => { expected_validators.insert(stash, validators); },
    //             Nominators { stash, nominations } => { expected_nominators.insert(stash, nominations); },
    //             VirtualStakers(staker) => { expected_virtual_stakers.insert(staker); },
    //             ErasStartSessionIndex { era, session } => { expected_eras_start_session_index.insert(era, session); },
    //             ErasStakersOverview { era, validator, exposure } => { expected_eras_stakers_overview.insert((era, validator.clone()), exposure); },
    //             ErasStakersPaged { era, validator, page, exposure } => { expected_eras_stakers_paged.insert((era, validator.clone(), page), exposure.into()); },
    //             ClaimedRewards { era, validator, rewards } => { expected_claimed_rewards.insert((era, validator.clone()), rewards); },
    //             ErasValidatorPrefs { era, validator, prefs } => { expected_eras_validator_prefs.insert((era, validator.clone()), prefs); },
    //             ErasValidatorReward { era, reward } => { expected_eras_validator_reward.insert(era, reward); },
    //             ErasRewardPoints { era, points } => { expected_eras_reward_points.insert(era, points); },
    //             ErasTotalStake { era, total_stake } => { expected_eras_total_stake.insert(era, total_stake); },
    //             UnappliedSlashes { era, slash } => {
    //                 let slash_map_key_tuple = (slash.validator.clone(), Perbill::from_percent(99), 9999u32);
    //                 expected_unapplied_slashes.insert((era, slash_map_key_tuple), slash);
    //             },
    //             BondedEras(be) => expected_bonded_eras = be,
    //             ValidatorSlashInEra { era, validator, slash } => { expected_validator_slash_in_era.insert((era, validator.clone()), slash); },
    //             NominatorSlashInEra { era, validator, slash } => { expected_nominator_slash_in_era.insert((era, validator.clone()), slash); },
    //             SlashingSpans { account, spans } => { expected_slashing_spans.insert(account, spans); },
    // //            SpanSlash { account, span, slash } => { expected_span_slash.insert((account, span), slash); },
    //             _ => todo!(), // Spanslash removes from master branch
    //         }
    //     }

    //     if let Some(values) = expected_values_opt {
    //         // "Assert storage 'StakingAsync::ValidatorCount::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::ValidatorCount::<T>::get(), values.validator_count, "StakingAsync::ValidatorCount mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::MinNominatorBond::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::MinNominatorBond::<T>::get(), values.min_nominator_bond, "StakingAsync::MinNominatorBond mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::MinValidatorBond::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::MinValidatorBond::<T>::get(), values.min_validator_bond, "StakingAsync::MinValidatorBond mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::MinimumActiveStake::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::MinimumActiveStake::<T>::get(), values.min_active_stake, "StakingAsync::MinimumActiveStake mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::MinCommission::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::MinCommission::<T>::get(), values.min_commission, "StakingAsync::MinCommission mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::MaxValidatorsCount::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::MaxValidatorsCount::<T>::get(), values.max_validators_count, "StakingAsync::MaxValidatorsCount mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::MaxNominatorsCount::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::MaxNominatorsCount::<T>::get(), values.max_nominators_count, "StakingAsync::MaxNominatorsCount mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::CurrentEra::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::CurrentEra::<T>::get(), values.current_era, "StakingAsync::CurrentEra mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::ActiveEra::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::ActiveEra::<T>::get(), values.active_era, "StakingAsync::ActiveEra mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::ForceEra::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::ForceEra::<T>::get(), values.force_era, "StakingAsync::ForceEra mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::MaxStakedRewards::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::MaxStakedRewards::<T>::get(), values.max_staked_rewards, "StakingAsync::MaxStakedRewards mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::SlashRewardFraction::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::SlashRewardFraction::<T>::get(), values.slash_reward_fraction, "StakingAsync::SlashRewardFraction mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::CanceledSlashPayout::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::CanceledSlashPayout::<T>::get(), values.canceled_slash_payout, "StakingAsync::CanceledSlashPayout mismatch on AH post-migration");
    //         // "Assert storage 'StakingAsync::ChillThreshold::ah_post::correct'"
    //         assert_eq!(pallet_staking_async::ChillThreshold::<T>::get(), values.chill_threshold, "StakingAsync::ChillThreshold mismatch on AH post-migration");
    //     }

    //     // "Assert storage 'StakingAsync::Invulnerables::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::Invulnerables::<T>::get().into_inner(), expected_invulnerables, "StakingAsync::Invulnerables mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::BondedEras::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::BondedEras::<T>::get().into_inner(), expected_bonded_eras, "StakingAsync::BondedEras mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::Bonded::ah_post::length'"
    //     assert_eq!(pallet_staking_async::Bonded::<T>::iter_keys().count(), expected_bonded.len(), "StakingAsync::Bonded map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::Bonded::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::Bonded::<T>::iter().collect::<BTreeMap<_,_>>(), expected_bonded, "StakingAsync::Bonded map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::Ledger::ah_post::length'"
    //     assert_eq!(pallet_staking_async::Ledger::<T>::iter_keys().count(), expected_ledger.len(), "StakingAsync::Ledger map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::Ledger::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::Ledger::<T>::iter().collect::<BTreeMap<_,_>>(), expected_ledger, "StakingAsync::Ledger map content mismatch on AH post-migration");
        
    //     // "Assert storage 'StakingAsync::Payee::ah_post::length'"
    //     assert_eq!(pallet_staking_async::Payee::<T>::iter_keys().count(), expected_payee.len(), "StakingAsync::Payee map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::Payee::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::Payee::<T>::iter().collect::<BTreeMap<_,_>>(), expected_payee, "StakingAsync::Payee map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::Validators::ah_post::length'"
    //     assert_eq!(pallet_staking_async::Validators::<T>::iter_keys().count(), expected_validators.len(), "StakingAsync::Validators map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::Validators::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::Validators::<T>::iter().collect::<BTreeMap<_,_>>(), expected_validators, "StakingAsync::Validators map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::Nominators::ah_post::length'"
    //     assert_eq!(pallet_staking_async::Nominators::<T>::iter_keys().count(), expected_nominators.len(), "StakingAsync::Nominators map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::Nominators::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::Nominators::<T>::iter().collect::<BTreeMap<_,_>>(), expected_nominators, "StakingAsync::Nominators map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::VirtualStakers::ah_post::length'"
    //     assert_eq!(pallet_staking_async::VirtualStakers::<T>::iter_keys().count(), expected_virtual_stakers.len(), "StakingAsync::VirtualStakers length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::VirtualStakers::ah_post::correct'"
    //     let current_virtual_stakers = pallet_staking_async::VirtualStakers::<T>::iter_keys().collect::<HashSet<_>>();
    //     assert_eq!(current_virtual_stakers, expected_virtual_stakers, "StakingAsync::VirtualStakers content mismatch on AH post-migration");
        
    //     // "Assert storage 'StakingAsync::ErasStartSessionIndex::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasStartSessionIndex::<T>::iter_keys().count(), expected_eras_start_session_index.len(), "StakingAsync::ErasStartSessionIndex map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasStartSessionIndex::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::ErasStartSessionIndex::<T>::iter().collect::<BTreeMap<_,_>>(), expected_eras_start_session_index, "StakingAsync::ErasStartSessionIndex map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::ErasStakersOverview::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasStakersOverview::<T>::iter_keys().count(), expected_eras_stakers_overview.len(), "StakingAsync::ErasStakersOverview map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasStakersOverview::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::ErasStakersOverview::<T>::iter().collect::<BTreeMap<_,_>>(), expected_eras_stakers_overview, "StakingAsync::ErasStakersOverview map content mismatch on AH post-migration");
        
    //     // "Assert storage 'StakingAsync::ErasStakersPaged::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasStakersPaged::<T>::iter_keys().count(), expected_eras_stakers_paged.len(), "StakingAsync::ErasStakersPaged map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasStakersPaged::ah_post::correct'"
    //     let current_eras_stakers_paged = pallet_staking_async::ErasStakersPaged::<T>::iter()
    //         .map(|(k, v_bounded)| (k, v_bounded.0))
    //         .collect::<BTreeMap<_,_>>();
    //     assert_eq!(current_eras_stakers_paged, expected_eras_stakers_paged, "StakingAsync::ErasStakersPaged map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::ErasClaimedRewards::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasClaimedRewards::<T>::iter_keys().count(), expected_claimed_rewards.len(), "StakingAsync::ErasClaimedRewards map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasClaimedRewards::ah_post::correct'"
    //     let current_claimed_rewards = pallet_staking_async::ErasClaimedRewards::<T>::iter()
    //         .map(|(k1, k2, v_weak_bounded)| ((k1, k2), v_weak_bounded.into_inner()))
    //         .collect::<BTreeMap<_,_>>();
    //     assert_eq!(current_claimed_rewards, expected_claimed_rewards, "StakingAsync::ErasClaimedRewards map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::ErasValidatorPrefs::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasValidatorPrefs::<T>::iter_keys().count(), expected_eras_validator_prefs.len(), "StakingAsync::ErasValidatorPrefs map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasValidatorPrefs::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::ErasValidatorPrefs::<T>::iter().collect::<BTreeMap<_,_>>(), expected_eras_validator_prefs, "StakingAsync::ErasValidatorPrefs map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::ErasValidatorReward::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasValidatorReward::<T>::iter_keys().count(), expected_eras_validator_reward.len(), "StakingAsync::ErasValidatorReward map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasValidatorReward::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::ErasValidatorReward::<T>::iter().collect::<BTreeMap<_,_>>(), expected_eras_validator_reward, "StakingAsync::ErasValidatorReward map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::ErasRewardPoints::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasRewardPoints::<T>::iter_keys().count(), expected_eras_reward_points.len(), "StakingAsync::ErasRewardPoints map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasRewardPoints::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::ErasRewardPoints::<T>::iter().collect::<BTreeMap<_,_>>(), expected_eras_reward_points, "StakingAsync::ErasRewardPoints map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::ErasTotalStake::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ErasTotalStake::<T>::iter_keys().count(), expected_eras_total_stake.len(), "StakingAsync::ErasTotalStake map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ErasTotalStake::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::ErasTotalStake::<T>::iter().collect::<BTreeMap<_,_>>(), expected_eras_total_stake, "StakingAsync::ErasTotalStake map content mismatch on AH post-migration");
        
    //     // "Assert storage 'StakingAsync::UnappliedSlashes::ah_post::length'"
    //     assert_eq!(pallet_staking_async::UnappliedSlashes::<T>::iter_keys().count(), expected_unapplied_slashes.len(), "StakingAsync::UnappliedSlashes map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::UnappliedSlashes::ah_post::correct'"
    //     let current_unapplied_slashes = pallet_staking_async::UnappliedSlashes::<T>::iter()
    //         .map(|(k1_era, k2_tuple, v_slash)| ((k1_era, k2_tuple), v_slash))
    //         .collect::<BTreeMap<_,_>>();
    //     assert_eq!(current_unapplied_slashes, expected_unapplied_slashes, "StakingAsync::UnappliedSlashes map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::ValidatorSlashInEra::ah_post::length'"
    //     assert_eq!(pallet_staking_async::ValidatorSlashInEra::<T>::iter_keys().count(), expected_validator_slash_in_era.len(), "StakingAsync::ValidatorSlashInEra map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::ValidatorSlashInEra::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::ValidatorSlashInEra::<T>::iter().collect::<BTreeMap<_,_>>(), expected_validator_slash_in_era, "StakingAsync::ValidatorSlashInEra map content mismatch on AH post-migration");

    //     // "Assert storage 'StakingAsync::NominatorSlashInEra::ah_post::length'"
    //     assert_eq!(pallet_staking_async::NominatorSlashInEra::<T>::iter_keys().count(), expected_nominator_slash_in_era.len(), "StakingAsync::NominatorSlashInEra map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::NominatorSlashInEra::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::NominatorSlashInEra::<T>::iter().collect::<BTreeMap<_,_>>(), expected_nominator_slash_in_era, "StakingAsync::NominatorSlashInEra map content mismatch on AH post-migration");

    // //     // "Assert storage 'StakingAsync::SlashingSpans::ah_post::length'"
    // //     assert_eq!(pallet_staking_async::SlashingSpans::<T>::iter_keys().count(), expected_slashing_spans.len(), "StakingAsync::SlashingSpans map length mismatch on AH post-migration");
    // //     // "Assert storage 'StakingAsync::SlashingSpans::ah_post::correct'"
    // //     assert_eq!(pallet_staking_async::SlashingSpans::<T>::iter().collect::<BTreeMap<_,_>>(), expected_slashing_spans, "StakingAsync::SlashingSpans map content mismatch on AH post-migration");
        
    //     // "Assert storage 'StakingAsync::SpanSlash::ah_post::length'"
    //     assert_eq!(pallet_staking_async::SpanSlash::<T>::iter_keys().count(), expected_span_slash.len(), "StakingAsync::SpanSlash map length mismatch on AH post-migration");
    //     // "Assert storage 'StakingAsync::SpanSlash::ah_post::correct'"
    //     assert_eq!(pallet_staking_async::SpanSlash::<T>::iter().collect::<BTreeMap<_,_>>(), expected_span_slash, "StakingAsync::SpanSlash map content mismatch on AH post-migration");
    }
}
