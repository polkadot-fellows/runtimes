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

pub use crate::staking::message::{
	AhEquivalentStakingMessageOf, RcStakingMessage, RcStakingMessageOf,
};
use crate::{staking::IntoAh, *};
use codec::{EncodeLike, HasCompact};
use core::fmt::Debug;
pub use frame_election_provider_support::PageIndex;
use frame_support::traits::DefensiveTruncateInto;
use pallet_staking::{
	slashing::{SlashingSpans, SpanIndex, SpanRecord},
	ActiveEraInfo, EraRewardPoints, Forcing, Nominations, RewardDestination, StakingLedger,
	ValidatorPrefs,
};
use sp_runtime::{Perbill, Percent};
use sp_staking::{EraIndex, ExposurePage, Page, PagedExposureMetadata, SessionIndex};

pub struct StakingMigrator<T> {
	_phantom: PhantomData<T>,
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Default,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum StakingStage<AccountId> {
	#[default]
	Values,
	Invulnerables,
	Bonded(Option<AccountId>),
	Ledger(Option<AccountId>),
	Payee(Option<AccountId>),
	Validators(Option<AccountId>),
	Nominators(Option<AccountId>),
	VirtualStakers(Option<AccountId>),
	ErasStartSessionIndex(Option<EraIndex>),
	ErasStakersOverview(Option<(EraIndex, AccountId)>),
	ErasStakersPaged(Option<(EraIndex, AccountId, Page)>),
	ClaimedRewards(Option<(EraIndex, AccountId)>),
	ErasValidatorPrefs(Option<(EraIndex, AccountId)>),
	ErasValidatorReward(Option<EraIndex>),
	ErasRewardPoints(Option<EraIndex>),
	ErasTotalStake(Option<EraIndex>),
	UnappliedSlashes(Option<EraIndex>),
	BondedEras,
	ValidatorSlashInEra(Option<(EraIndex, AccountId)>),
	NominatorSlashInEra(Option<(EraIndex, AccountId)>),
	SlashingSpans(Option<AccountId>),
	SpanSlash(Option<(AccountId, SpanIndex)>),
	Finished,
}

pub type StakingStageOf<T> = StakingStage<<T as frame_system::Config>::AccountId>;

pub type BalanceOf<T> = <T as pallet_staking::Config>::CurrencyBalance;
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

impl<T: Config> PalletMigration for StakingMigrator<T> {
	type Key = StakingStageOf<T>;
	type Error = Error<T>;

	fn migrate_many(
		current_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut inner_key = current_key.unwrap_or_default();
		let mut messages = XcmBatchAndMeter::new_from_config::<T>();

		loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err() ||
				weight_counter.try_consume(messages.consume_weight()).is_err()
			{
				log::info!("RC weight limit reached at batch length {}, stopping", messages.len());
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > 10_000 {
				log::warn!("Weight allowed very big batch, stopping");
				break;
			}

			inner_key = match inner_key {
				StakingStage::Values => {
					let values = Self::take_values();
					messages.push(RcStakingMessage::Values(values));
					StakingStage::Invulnerables
				},
				StakingStage::Invulnerables => {
					let invulnerables = pallet_staking::Invulnerables::<T>::take();
					messages.push(RcStakingMessage::Invulnerables(invulnerables));
					StakingStage::Bonded(None)
				},
				StakingStage::Bonded(who) => {
					let mut iter = if let Some(who) = who {
						pallet_staking::Bonded::<T>::iter_from_key(who)
					} else {
						pallet_staking::Bonded::<T>::iter()
					};

					match iter.next() {
						Some((stash, controller)) => {
							pallet_staking::Bonded::<T>::remove(&stash);
							messages.push(RcStakingMessage::Bonded {
								stash: stash.clone(),
								controller,
							});
							StakingStage::Bonded(Some(stash))
						},
						None => StakingStage::Ledger(None),
					}
				},
				StakingStage::Ledger(who) => {
					let mut iter = if let Some(who) = who {
						pallet_staking::Ledger::<T>::iter_from_key(who)
					} else {
						pallet_staking::Ledger::<T>::iter()
					};

					match iter.next() {
						Some((controller, ledger)) => {
							pallet_staking::Ledger::<T>::remove(&controller);
							messages.push(RcStakingMessage::Ledger {
								controller: controller.clone(),
								ledger,
							});
							StakingStage::Ledger(Some(controller))
						},
						None => StakingStage::Payee(None),
					}
				},
				StakingStage::Payee(who) => {
					let mut iter = if let Some(who) = who {
						pallet_staking::Payee::<T>::iter_from_key(who)
					} else {
						pallet_staking::Payee::<T>::iter()
					};

					match iter.next() {
						Some((stash, payment)) => {
							pallet_staking::Payee::<T>::remove(&stash);
							messages
								.push(RcStakingMessage::Payee { stash: stash.clone(), payment });
							StakingStage::Payee(Some(stash))
						},
						None => StakingStage::Validators(None),
					}
				},
				StakingStage::Validators(who) => {
					let mut iter = if let Some(who) = who {
						pallet_staking::Validators::<T>::iter_from(
							pallet_staking::Validators::<T>::hashed_key_for(who),
						)
					} else {
						pallet_staking::Validators::<T>::iter()
					};

					match iter.next() {
						Some((stash, validators)) => {
							pallet_staking::Validators::<T>::remove(&stash);
							messages.push(RcStakingMessage::Validators {
								stash: stash.clone(),
								validators,
							});
							StakingStage::Validators(Some(stash))
						},
						None => StakingStage::Nominators(None),
					}
				},
				StakingStage::Nominators(who) => {
					let mut iter = if let Some(who) = who {
						pallet_staking::Nominators::<T>::iter_from(
							pallet_staking::Nominators::<T>::hashed_key_for(who),
						)
					} else {
						pallet_staking::Nominators::<T>::iter()
					};

					match iter.next() {
						Some((stash, nominations)) => {
							pallet_staking::Nominators::<T>::remove(&stash);
							messages.push(RcStakingMessage::Nominators {
								stash: stash.clone(),
								nominations,
							});
							StakingStage::Nominators(Some(stash))
						},
						None => StakingStage::VirtualStakers(None),
					}
				},
				StakingStage::VirtualStakers(who) => {
					let mut iter = if let Some(who) = who {
						pallet_staking::VirtualStakers::<T>::iter_from(
							// Counted maps dont have the convenience function here
							pallet_staking::VirtualStakers::<T>::hashed_key_for(who),
						)
					} else {
						pallet_staking::VirtualStakers::<T>::iter()
					};

					match iter.next() {
						Some((staker, ())) => {
							pallet_staking::VirtualStakers::<T>::remove(&staker);
							messages.push(RcStakingMessage::VirtualStakers(staker.clone()));
							StakingStage::VirtualStakers(Some(staker))
						},
						None => StakingStage::ErasStartSessionIndex(None),
					}
				},
				StakingStage::ErasStartSessionIndex(who) => {
					let mut iter = if let Some(who) = who {
						pallet_staking::ErasStartSessionIndex::<T>::iter_from_key(who)
					} else {
						pallet_staking::ErasStartSessionIndex::<T>::iter()
					};

					match iter.next() {
						Some((era, session)) => {
							pallet_staking::ErasStartSessionIndex::<T>::remove(&era);
							messages.push(RcStakingMessage::ErasStartSessionIndex { era, session });
							StakingStage::ErasStartSessionIndex(Some(era))
						},
						None => StakingStage::ErasStakersOverview(None),
					}
				},
				StakingStage::ErasStakersOverview(progress) => {
					let mut iter = if let Some(progress) = progress {
						pallet_staking::ErasStakersOverview::<T>::iter_from(
							pallet_staking::ErasStakersOverview::<T>::hashed_key_for(
								progress.0, progress.1,
							),
						)
					} else {
						pallet_staking::ErasStakersOverview::<T>::iter()
					};

					match iter.next() {
						Some((era, validator, exposure)) => {
							pallet_staking::ErasStakersOverview::<T>::remove(&era, &validator);
							messages.push(RcStakingMessage::ErasStakersOverview {
								era,
								validator: validator.clone(),
								exposure,
							});
							StakingStage::ErasStakersOverview(Some((era, validator)))
						},
						None => StakingStage::ErasStakersPaged(None),
					}
				},
				StakingStage::ErasStakersPaged(progress) => {
					let mut iter = if let Some(progress) = progress {
						pallet_staking::ErasStakersPaged::<T>::iter_from(
							pallet_staking::ErasStakersPaged::<T>::hashed_key_for(progress),
						)
					} else {
						pallet_staking::ErasStakersPaged::<T>::iter()
					};

					match iter.next() {
						Some(((era, validator, page), exposure)) => {
							pallet_staking::ErasStakersPaged::<T>::remove((
								&era, &validator, &page,
							));
							messages.push(RcStakingMessage::ErasStakersPaged {
								era,
								validator: validator.clone(),
								page,
								exposure,
							});
							StakingStage::ErasStakersPaged(Some((era, validator, page)))
						},
						None => StakingStage::ClaimedRewards(None),
					}
				},
				StakingStage::ClaimedRewards(progress) => {
					let mut iter = if let Some(progress) = progress {
						pallet_staking::ClaimedRewards::<T>::iter_from(
							pallet_staking::ClaimedRewards::<T>::hashed_key_for(
								progress.0, progress.1,
							),
						)
					} else {
						pallet_staking::ClaimedRewards::<T>::iter()
					};

					match iter.next() {
						Some((era, validator, rewards)) => {
							pallet_staking::ClaimedRewards::<T>::remove(&era, &validator);
							messages.push(RcStakingMessage::ClaimedRewards {
								era,
								validator: validator.clone(),
								rewards,
							});
							StakingStage::ClaimedRewards(Some((era, validator)))
						},
						None => StakingStage::ErasValidatorPrefs(None),
					}
				},
				StakingStage::ErasValidatorPrefs(progress) => {
					let mut iter = if let Some(progress) = progress {
						pallet_staking::ErasValidatorPrefs::<T>::iter_from(
							pallet_staking::ErasValidatorPrefs::<T>::hashed_key_for(
								progress.0, progress.1,
							),
						)
					} else {
						pallet_staking::ErasValidatorPrefs::<T>::iter()
					};

					match iter.next() {
						Some((era, validator, prefs)) => {
							pallet_staking::ErasValidatorPrefs::<T>::remove(&era, &validator);
							messages.push(RcStakingMessage::ErasValidatorPrefs {
								era,
								validator: validator.clone(),
								prefs,
							});
							StakingStage::ErasValidatorPrefs(Some((era, validator)))
						},
						None => StakingStage::ErasValidatorReward(None),
					}
				},
				StakingStage::ErasValidatorReward(era) => {
					let mut iter = resume::<pallet_staking::ErasValidatorReward<T>, _, _>(era);

					match iter.next() {
						Some((era, reward)) => {
							pallet_staking::ErasValidatorReward::<T>::remove(&era);
							messages.push(RcStakingMessage::ErasValidatorReward { era, reward });
							StakingStage::ErasValidatorReward(Some(era))
						},
						None => StakingStage::ErasRewardPoints(None),
					}
				},
				StakingStage::ErasRewardPoints(era) => {
					let mut iter = resume::<pallet_staking::ErasRewardPoints<T>, _, _>(era);

					match iter.next() {
						Some((era, points)) => {
							pallet_staking::ErasRewardPoints::<T>::remove(&era);
							messages.push(RcStakingMessage::ErasRewardPoints { era, points });
							StakingStage::ErasRewardPoints(Some(era))
						},
						None => StakingStage::ErasTotalStake(None),
					}
				},
				StakingStage::ErasTotalStake(era) => {
					let mut iter = resume::<pallet_staking::ErasTotalStake<T>, _, _>(era);

					match iter.next() {
						Some((era, total_stake)) => {
							pallet_staking::ErasTotalStake::<T>::remove(&era);
							messages.push(RcStakingMessage::ErasTotalStake { era, total_stake });
							StakingStage::ErasTotalStake(Some(era))
						},
						None => StakingStage::UnappliedSlashes(None),
					}
				},
				StakingStage::UnappliedSlashes(era) => {
					let mut iter = resume::<pallet_staking::UnappliedSlashes<T>, _, _>(era);

					match iter.next() {
						Some((era, slashes)) => {
							pallet_staking::UnappliedSlashes::<T>::remove(&era);

							if slashes.len() > 1000 {
								defensive!("Lots of unapplied slashes for era, this is odd");
							}

							// Translate according to https://github.com/paritytech/polkadot-sdk/blob/43ea306f6307dff908551cb91099ef6268502ee0/substrate/frame/staking/src/migrations.rs#L94-L108
							for slash in slashes.into_iter().take(1000) {
								// First 1000 slashes should be enough, just to avoid unbound loop
								messages.push(RcStakingMessage::UnappliedSlashes { era, slash });
							}
							StakingStage::UnappliedSlashes(Some(era))
						},
						None => StakingStage::BondedEras,
					}
				},
				StakingStage::BondedEras => {
					let bonded_eras = pallet_staking::BondedEras::<T>::take();
					messages.push(RcStakingMessage::BondedEras(bonded_eras));
					StakingStage::ValidatorSlashInEra(None)
				},
				StakingStage::ValidatorSlashInEra(next) => {
					let mut iter = if let Some(next) = next {
						pallet_staking::ValidatorSlashInEra::<T>::iter_from(
							pallet_staking::ValidatorSlashInEra::<T>::hashed_key_for(
								next.0, next.1,
							),
						)
					} else {
						pallet_staking::ValidatorSlashInEra::<T>::iter()
					};

					match iter.next() {
						Some((era, validator, slash)) => {
							pallet_staking::ValidatorSlashInEra::<T>::remove(&era, &validator);
							messages.push(RcStakingMessage::ValidatorSlashInEra {
								era,
								validator: validator.clone(),
								slash,
							});
							StakingStage::ValidatorSlashInEra(Some((era, validator)))
						},
						None => StakingStage::NominatorSlashInEra(None),
					}
				},
				StakingStage::NominatorSlashInEra(next) => {
					let mut iter = if let Some(next) = next {
						pallet_staking::NominatorSlashInEra::<T>::iter_from(
							pallet_staking::NominatorSlashInEra::<T>::hashed_key_for(
								next.0, next.1,
							),
						)
					} else {
						pallet_staking::NominatorSlashInEra::<T>::iter()
					};

					match iter.next() {
						Some((era, validator, slash)) => {
							pallet_staking::NominatorSlashInEra::<T>::remove(&era, &validator);
							messages.push(RcStakingMessage::NominatorSlashInEra {
								era,
								validator: validator.clone(),
								slash,
							});
							StakingStage::NominatorSlashInEra(Some((era, validator)))
						},
						None => StakingStage::SlashingSpans(None),
					}
				},
				StakingStage::SlashingSpans(account) => {
					let mut iter = resume::<pallet_staking::SlashingSpans<T>, _, _>(account);

					match iter.next() {
						Some((account, spans)) => {
							pallet_staking::SlashingSpans::<T>::remove(&account);
							messages.push(RcStakingMessage::SlashingSpans {
								account: account.clone(),
								spans,
							});
							StakingStage::SlashingSpans(Some(account))
						},
						None => StakingStage::SpanSlash(None),
					}
				},
				StakingStage::SpanSlash(next) => {
					let mut iter = resume::<pallet_staking::SpanSlash<T>, _, _>(next);

					match iter.next() {
						Some(((account, span), slash)) => {
							pallet_staking::SpanSlash::<T>::remove((&account, &span));
							messages.push(RcStakingMessage::SpanSlash {
								account: account.clone(),
								span,
								slash,
							});
							StakingStage::SpanSlash(Some((account, span)))
						},
						None => StakingStage::Finished,
					}
				},
				StakingStage::Finished => {
					break;
				},
			};
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm(
				messages,
				|messages| types::AhMigratorCall::<T>::ReceiveStakingMessages { messages },
				|_len| Weight::from_all(1),
			)?;
		}

		if inner_key == StakingStage::Finished {
			Ok(None)
		} else {
			Ok(Some(inner_key))
		}
	}
}

use codec::{FullCodec, FullEncode};
fn resume<Map: frame_support::IterableStorageMap<K, V>, K: FullEncode, V: FullCodec>(
	key: Option<K>,
) -> impl Iterator<Item = (K, V)> {
	if let Some(key) = key {
		Map::iter_from(Map::hashed_key_for(key))
	} else {
		Map::iter()
	}
}

// The payload that will be passed between pre and post migration checks
pub type RcPrePayload<T> = (
	// Values captured by `StakingMigrator::take_values()`.
	crate::staking::message::RcStakingValuesOf<T>,
	// Invulnerables.
	Vec<<T as frame_system::Config>::AccountId>,
	// Bonded map.
	Vec<(<T as frame_system::Config>::AccountId, <T as frame_system::Config>::AccountId)>,
	// Ledger map.
	Vec<(<T as frame_system::Config>::AccountId, pallet_staking::StakingLedger<T>)>,
	// Payee map.
	Vec<(
		<T as frame_system::Config>::AccountId,
		pallet_staking::RewardDestination<<T as frame_system::Config>::AccountId>,
	)>,
	// Validators map.
	Vec<(<T as frame_system::Config>::AccountId, pallet_staking::ValidatorPrefs)>,
	// Nominators map.
	Vec<(<T as frame_system::Config>::AccountId, pallet_staking::Nominations<T>)>,
	// VirtualStakers map.
	Vec<<T as frame_system::Config>::AccountId>,
	// ErasStartSessionIndex map.
	Vec<(sp_staking::EraIndex, sp_staking::SessionIndex)>,
	// ErasStakersOverview double map.
	Vec<(
		sp_staking::EraIndex,
		<T as frame_system::Config>::AccountId,
		sp_staking::PagedExposureMetadata<pallet_staking::BalanceOf<T>>,
	)>,
	// ErasStakersPaged N map.
	Vec<(
		(sp_staking::EraIndex, <T as frame_system::Config>::AccountId, sp_staking::Page),
		sp_staking::ExposurePage<<T as frame_system::Config>::AccountId, pallet_staking::BalanceOf<T>>,
	)>,
	// ClaimedRewards double map.
	Vec<(sp_staking::EraIndex, <T as frame_system::Config>::AccountId, Vec<sp_staking::Page>)>,
	// ErasValidatorPrefs double map.
	Vec<(
		sp_staking::EraIndex,
		<T as frame_system::Config>::AccountId,
		pallet_staking::ValidatorPrefs,
	)>,
	// ErasValidatorReward map.
	Vec<(sp_staking::EraIndex, pallet_staking::BalanceOf<T>)>,
	// ErasRewardPoints map.
	Vec<(
		sp_staking::EraIndex,
		pallet_staking::EraRewardPoints<<T as frame_system::Config>::AccountId>,
	)>,
	// ErasTotalStake map.
	Vec<(sp_staking::EraIndex, pallet_staking::BalanceOf<T>)>,
	// UnappliedSlashes map.
	Vec<(
		sp_staking::EraIndex,
		Vec<
			pallet_staking::UnappliedSlash<
				<T as frame_system::Config>::AccountId,
				pallet_staking::BalanceOf<T>,
			>,
		>,
	)>,
	// BondedEras.
	Vec<(sp_staking::EraIndex, sp_staking::SessionIndex)>,
	// ValidatorSlashInEra double map.
	Vec<(
		sp_staking::EraIndex,
		<T as frame_system::Config>::AccountId,
		(sp_runtime::Perbill, pallet_staking::BalanceOf<T>),
	)>,
	// NominatorSlashInEra double map.
	Vec<(
		sp_staking::EraIndex,
		<T as frame_system::Config>::AccountId,
		pallet_staking::BalanceOf<T>,
	)>,
	// SlashingSpans map.
	Vec<(<T as frame_system::Config>::AccountId, pallet_staking::slashing::SlashingSpans)>,
	// SpanSlash map.
	Vec<(
		(<T as frame_system::Config>::AccountId, pallet_staking::slashing::SpanIndex),
		pallet_staking::slashing::SpanRecord<pallet_staking::BalanceOf<T>>,
	)>,
);

#[cfg(all(feature = "std", feature = "ahm-staking-migration"))]
impl<T: Config> crate::types::RcMigrationCheck for StakingMigrator<T> {
	type RcPrePayload = RcPrePayload<T>;

    fn pre_check() -> Self::RcPrePayload {
        let staking_values = crate::staking::message::StakingValues {
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
        };

        let invulnerables = pallet_staking::Invulnerables::<T>::get();
        let bonded: Vec<_> = pallet_staking::Bonded::<T>::iter().collect();
        let ledgers: Vec<_> = pallet_staking::Ledger::<T>::iter().collect();
        let payees: Vec<_> = pallet_staking::Payee::<T>::iter().collect();
        let validators: Vec<_> = pallet_staking::Validators::<T>::iter().collect();
        let nominators: Vec<_> = pallet_staking::Nominators::<T>::iter().collect();
        let virtual_stakers: Vec<_> = pallet_staking::VirtualStakers::<T>::iter().map(|(k, _)| k).collect();
        let eras_start_session_index: Vec<_> =
            pallet_staking::ErasStartSessionIndex::<T>::iter().collect();
        let eras_stakers_overview: Vec<_> = pallet_staking::ErasStakersOverview::<T>::iter()
            .map(|(era, validator, exposure)| (era, validator, exposure))
            .collect();
        let eras_stakers_paged: Vec<_> = pallet_staking::ErasStakersPaged::<T>::iter().collect();
        let claimed_rewards: Vec<_> = pallet_staking::ClaimedRewards::<T>::iter()
            .map(|(era, validator, rewards)| (era, validator, rewards))
            .collect();
        let eras_validator_prefs: Vec<_> = pallet_staking::ErasValidatorPrefs::<T>::iter()
            .map(|(era, validator, prefs)| (era, validator, prefs))
            .collect();
        let eras_validator_reward: Vec<_> =
            pallet_staking::ErasValidatorReward::<T>::iter().collect();
        let eras_reward_points: Vec<_> = pallet_staking::ErasRewardPoints::<T>::iter().collect();
        let eras_total_stake: Vec<_> = pallet_staking::ErasTotalStake::<T>::iter().collect();
        let unapplied_slashes: Vec<_> = pallet_staking::UnappliedSlashes::<T>::iter().collect();
        let bonded_eras = pallet_staking::BondedEras::<T>::get();
        let validator_slash_in_era: Vec<_> =
            pallet_staking::ValidatorSlashInEra::<T>::iter()
            .map(|(era, validator, slash)| (era, validator, slash))
            .collect();
        let nominator_slash_in_era: Vec<_> =
            pallet_staking::NominatorSlashInEra::<T>::iter()
            .map(|(era, validator, slash)| (era, validator, slash))
            .collect();
        let slashing_spans: Vec<_> = pallet_staking::SlashingSpans::<T>::iter().collect();
        let span_slashes: Vec<_> = pallet_staking::SpanSlash::<T>::iter().collect();

        (
            staking_values,
            invulnerables,
            bonded,
            ledgers,
            payees,
            validators,
            nominators,
            virtual_stakers,
            eras_start_session_index,
            eras_stakers_overview,
            eras_stakers_paged,
            claimed_rewards,
            eras_validator_prefs,
            eras_validator_reward,
            eras_reward_points,
            eras_total_stake,
            unapplied_slashes,
            bonded_eras,
            validator_slash_in_era,
            nominator_slash_in_era,
            slashing_spans,
            span_slashes,
        )
    }

    fn post_check(_rc_pre_payload: Self::RcPrePayload) {
        // Assert storage 'Staking::ValidatorCount::rc_post::empty'
        assert_eq!(
            pallet_staking::ValidatorCount::<T>::get(),
            0,
            "ValidatorCount should be default on RC after migration"
        );
        // Assert storage 'Staking::MinimumValidatorCount::rc_post::empty'
        assert_eq!(
            pallet_staking::MinimumValidatorCount::<T>::get(),
            0,
            "MinimumValidatorCount should be default on RC after migration"
        );
        // Assert storage 'Staking::MinNominatorBond::rc_post::empty'
        assert_eq!(
            pallet_staking::MinNominatorBond::<T>::get(),
            Default::default(),
            "MinNominatorBond should be default on RC after migration"
        );
        // Assert storage 'Staking::MinValidatorBond::rc_post::empty'
        assert_eq!(
            pallet_staking::MinValidatorBond::<T>::get(),
            Default::default(),
            "MinValidatorBond should be default on RC after migration"
        );
        // Assert storage 'Staking::MinimumActiveStake::rc_post::empty'
        assert_eq!(
            pallet_staking::MinimumActiveStake::<T>::get(),
            Default::default(),
            "MinimumActiveStake should be default on RC after migration"
        );
        // Assert storage 'Staking::MinCommission::rc_post::empty'
        assert_eq!(
            pallet_staking::MinCommission::<T>::get(),
            Default::default(),
            "MinCommission should be default on RC after migration"
        );
        // Assert storage 'Staking::MaxValidatorsCount::rc_post::empty'
        assert_eq!(
            pallet_staking::MaxValidatorsCount::<T>::get(),
            None,
            "MaxValidatorsCount should be None on RC after migration"
        );
        // Assert storage 'Staking::MaxNominatorsCount::rc_post::empty'
        assert_eq!(
            pallet_staking::MaxNominatorsCount::<T>::get(),
            None,
            "MaxNominatorsCount should be None on RC after migration"
        );
        // Assert storage 'Staking::CurrentEra::rc_post::empty'
        assert_eq!(
            pallet_staking::CurrentEra::<T>::get(),
            None,
            "CurrentEra should be None on RC after migration"
        );
        // Assert storage 'Staking::ActiveEra::rc_post::empty'
        assert_eq!(
            pallet_staking::ActiveEra::<T>::get(),
            None,
            "ActiveEra should be None on RC after migration"
        );
        // Assert storage 'Staking::ForceEra::rc_post::empty'
        assert_eq!(
            pallet_staking::ForceEra::<T>::get(),
            Default::default(),
            "ForceEra should be default on RC after migration"
        );
        // Assert storage 'Staking::MaxStakedRewards::rc_post::empty'
        assert_eq!(
            pallet_staking::MaxStakedRewards::<T>::get(),
            None,
            "MaxStakedRewards should be None on RC after migration"
        );
        // Assert storage 'Staking::SlashRewardFraction::rc_post::empty'
        assert_eq!(
            pallet_staking::SlashRewardFraction::<T>::get(),
            Default::default(),
            "SlashRewardFraction should be default on RC after migration"
        );
        // Assert storage 'Staking::CanceledSlashPayout::rc_post::empty'
        assert_eq!(
            pallet_staking::CanceledSlashPayout::<T>::get(),
            Default::default(),
            "CanceledSlashPayout should be default on RC after migration"
        );
        // Assert storage 'Staking::CurrentPlannedSession::rc_post::empty'
        assert_eq!(
            pallet_staking::CurrentPlannedSession::<T>::get(),
            0,
            "CurrentPlannedSession should be default on RC after migration"
        );
        // Assert storage 'Staking::ChillThreshold::rc_post::empty'
        assert_eq!(
            pallet_staking::ChillThreshold::<T>::get(),
            None,
            "ChillThreshold should be None on RC after migration"
        );
        // Assert storage 'Staking::Invulnerables::rc_post::empty'
        assert!(
            pallet_staking::Invulnerables::<T>::get().is_empty(),
            "Invulnerables should be empty on RC after migration"
        );
        // Assert storage 'Staking::BondedEras::rc_post::empty'
        assert!(
            pallet_staking::BondedEras::<T>::get().is_empty(),
            "BondedEras should be empty on RC after migration"
        );
        // Assert storage 'Staking::Bonded::rc_post::empty'
        assert!(
            pallet_staking::Bonded::<T>::iter().next().is_none(),
            "Bonded map should be empty on RC after migration"
        );
        // Assert storage 'Staking::Ledger::rc_post::empty'
        assert!(
            pallet_staking::Ledger::<T>::iter().next().is_none(),
            "Ledger map should be empty on RC after migration"
        );
        // Assert storage 'Staking::Payee::rc_post::empty'
        assert!(
            pallet_staking::Payee::<T>::iter().next().is_none(),
            "Payee map should be empty on RC after migration"
        );
        // Assert storage 'Staking::Validators::rc_post::empty'
        assert!(
            pallet_staking::Validators::<T>::iter().next().is_none(),
            "Validators map should be empty on RC after migration"
        );
        // Assert storage 'Staking::Nominators::rc_post::empty'
        assert!(
            pallet_staking::Nominators::<T>::iter().next().is_none(),
            "Nominators map should be empty on RC after migration"
        );
        // Assert storage 'Staking::VirtualStakers::rc_post::empty'
        assert!(
            pallet_staking::VirtualStakers::<T>::iter().next().is_none(),
            "VirtualStakers map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ErasStartSessionIndex::rc_post::empty'
        assert!(
            pallet_staking::ErasStartSessionIndex::<T>::iter().next().is_none(),
            "ErasStartSessionIndex map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ErasStakersOverview::rc_post::empty'
        assert!(
            pallet_staking::ErasStakersOverview::<T>::iter().next().is_none(),
            "ErasStakersOverview map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ErasStakersPaged::rc_post::empty'
        assert!(
            pallet_staking::ErasStakersPaged::<T>::iter().next().is_none(),
            "ErasStakersPaged map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ClaimedRewards::rc_post::empty'
        assert!(
            pallet_staking::ClaimedRewards::<T>::iter().next().is_none(),
            "ClaimedRewards map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ErasValidatorPrefs::rc_post::empty'
        assert!(
            pallet_staking::ErasValidatorPrefs::<T>::iter().next().is_none(),
            "ErasValidatorPrefs map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ErasValidatorReward::rc_post::empty'
        assert!(
            pallet_staking::ErasValidatorReward::<T>::iter().next().is_none(),
            "ErasValidatorReward map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ErasRewardPoints::rc_post::empty'
        assert!(
            pallet_staking::ErasRewardPoints::<T>::iter().next().is_none(),
            "ErasRewardPoints map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ErasTotalStake::rc_post::empty'
        assert!(
            pallet_staking::ErasTotalStake::<T>::iter().next().is_none(),
            "ErasTotalStake map should be empty on RC after migration"
        );
        // Assert storage 'Staking::UnappliedSlashes::rc_post::empty'
        assert!(
            pallet_staking::UnappliedSlashes::<T>::iter().next().is_none(),
            "UnappliedSlashes map should be empty on RC after migration"
        );
        // Assert storage 'Staking::ValidatorSlashInEra::rc_post::empty'
        assert!(
            pallet_staking::ValidatorSlashInEra::<T>::iter().next().is_none(),
            "ValidatorSlashInEra map should be empty on RC after migration"
        );
        // Assert storage 'Staking::NominatorSlashInEra::rc_post::empty'
        assert!(
            pallet_staking::NominatorSlashInEra::<T>::iter().next().is_none(),
            "NominatorSlashInEra map should be empty on RC after migration"
        );
        // Assert storage 'Staking::SlashingSpans::rc_post::empty'
        assert!(
            pallet_staking::SlashingSpans::<T>::iter().next().is_none(),
            "SlashingSpans map should be empty on RC after migration"
        );
        // Assert storage 'Staking::SpanSlash::rc_post::empty'
        assert!(
            pallet_staking::SpanSlash::<T>::iter().next().is_none(),
            "SpanSlash map should be empty on RC after migration"
        );

		// -- Ensure deprecated storage items empty as well --

		// Assert storage 'Staking::ErasStakers::rc_post::empty'
        assert!(
            pallet_staking::ErasStakers::<T>::iter().next().is_none(),
            "ErasStakers map should be empty on RC after migration (deprecated item)"
        );
        // Assert storage 'Staking::ErasStakersClipped::rc_post::empty'
        assert!(
            pallet_staking::ErasStakersClipped::<T>::iter().next().is_none(),
            "ErasStakersClipped map should be empty on RC after migration (deprecated item)"
        );
    }
}
