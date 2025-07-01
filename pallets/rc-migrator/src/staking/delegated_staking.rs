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

//! Migrator for pallet-delegated-staking.

use crate::*;
use types::AccountIdOf;

/// Stage of the delegated-staking pallet migration.
#[derive(Encode, Decode, Clone, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
#[cfg_attr(feature = "stable2503", derive(DecodeWithMemTracking))]
pub enum DelegatedStakingStage<AccountId> {
	Delegators(Option<AccountId>),
	Agents(Option<AccountId>),
	Finished,
}

/// Message that is being sent to the AH Migrator.
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
#[cfg_attr(feature = "stable2503", derive(DecodeWithMemTracking))]
pub enum RcDelegatedStakingMessage<AccountId, Balance> {
	Delegators {
		delegator: AccountId,
		agent: AccountId,
		amount: Balance,
	},
	Agents {
		agent: AccountId,
		payee: AccountId,
		total_delegated: Balance,
		unclaimed_withdrawals: Balance,
		pending_slash: Balance,
	},
}

pub type RcDelegatedStakingMessageOf<T> = RcDelegatedStakingMessage<AccountIdOf<T>, BalanceOf<T>>;

pub mod alias {
	use super::*;

	// From https://github.com/paritytech/polkadot-sdk/blob/0447d26148ef5b97f40fc01bce2d5156ab335eca/substrate/frame/delegated-staking/src/types.rs#L35
	#[derive(Default, Encode, Clone, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "stable2503", derive(DecodeWithMemTracking))]
	pub struct Delegation<AccountId, Balance> {
		/// The target of delegation.
		pub agent: AccountId,
		/// The amount delegated.
		pub amount: Balance,
	}

	// From https://github.com/paritytech/polkadot-sdk/blob/0447d26148ef5b97f40fc01bce2d5156ab335eca/substrate/frame/delegated-staking/src/types.rs#L95
	#[derive(Default, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	#[cfg_attr(feature = "stable2503", derive(DecodeWithMemTracking))]
	pub struct AgentLedger<AccountId, Balance> {
		/// Where the reward should be paid out.
		pub payee: AccountId,
		/// Sum of all delegated funds to this `Agent`.
		pub total_delegated: Balance,
		/// Funds that are withdrawn from core staking but not released to delegator/s. It is a
		/// subset of `total_delegated` and can never be greater than it.
		pub unclaimed_withdrawals: Balance,
		/// Slashes that are not yet applied. This affects the effective balance of the `Agent`.
		pub pending_slash: Balance,
	}

	/// Alias for private item [`pallet_delegated_staking::Delegations`].
	///
	/// Source: https://github.com/paritytech/polkadot-sdk/blob/0447d26148ef5b97f40fc01bce2d5156ab335eca/substrate/frame/delegated-staking/src/lib.rs#L277
	#[frame_support::storage_alias(pallet_name)]
	pub type Delegations<T: pallet_delegated_staking::Config> = CountedStorageMap<
		pallet_delegated_staking::Pallet<T>,
		Twox64Concat,
		<T as frame_system::Config>::AccountId,
		Delegation<<T as frame_system::Config>::AccountId, BalanceOf<T>>,
		OptionQuery,
	>;

	/// Alias for private item [`pallet_delegated_staking::Agents`].
	///
	/// Source: https://github.com/paritytech/polkadot-sdk/blob/0447d26148ef5b97f40fc01bce2d5156ab335eca/substrate/frame/delegated-staking/src/lib.rs#L282
	#[frame_support::storage_alias(pallet_name)]
	pub type AgentLedgers<T: pallet_delegated_staking::Config> = CountedStorageMap<
		pallet_delegated_staking::Pallet<T>,
		Twox64Concat,
		<T as frame_system::Config>::AccountId,
		AgentLedger<<T as frame_system::Config>::AccountId, BalanceOf<T>>,
		OptionQuery,
	>;
}

pub struct DelegatedStakingMigrator<T> {
	_phantom: sp_std::marker::PhantomData<T>,
}

impl<T: Config> PalletMigration for DelegatedStakingMigrator<T> {
	type Key = DelegatedStakingStage<AccountIdOf<T>>;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut last_key = last_key.unwrap_or(DelegatedStakingStage::Delegators(None));
		let mut messages = XcmBatchAndMeter::<
			RcDelegatedStakingMessage<AccountIdOf<T>, BalanceOf<T>>,
		>::new_from_config::<T>();

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
			if T::MaxAhWeight::get().any_lt(T::AhWeightInfo::receive_delegated_staking_messages(
				(messages.len() + 1) as u32,
			)) {
				log::info!("AH weight limit reached at batch length {}, stopping", messages.len());
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			last_key = match last_key {
				DelegatedStakingStage::Delegators(last_key) => {
					let mut delegators_iter = if let Some(last_key) = last_key.clone() {
						alias::Delegations::<T>::iter_from(alias::Delegations::<T>::hashed_key_for(
							last_key,
						))
					} else {
						alias::Delegations::<T>::iter()
					};
					match delegators_iter.next() {
						Some((key, value)) => {
							alias::Delegations::<T>::remove(&key);
							messages.push(RcDelegatedStakingMessage::Delegators {
								delegator: key.clone(),
								agent: value.agent,
								amount: value.amount,
							});
							DelegatedStakingStage::Delegators(Some(key))
						},
						None => DelegatedStakingStage::Agents(None),
					}
				},
				DelegatedStakingStage::Agents(last_key) => {
					let mut agents_iter = if let Some(last_key) = last_key.clone() {
						alias::AgentLedgers::<T>::iter_from(
							alias::AgentLedgers::<T>::hashed_key_for(last_key),
						)
					} else {
						alias::AgentLedgers::<T>::iter()
					};
					match agents_iter.next() {
						Some((key, value)) => {
							alias::AgentLedgers::<T>::remove(&key);
							messages.push(RcDelegatedStakingMessage::Agents {
								agent: key.clone(),
								payee: value.payee,
								total_delegated: value.total_delegated,
								unclaimed_withdrawals: value.unclaimed_withdrawals,
								pending_slash: value.pending_slash,
							});
							DelegatedStakingStage::Agents(Some(key))
						},
						None => DelegatedStakingStage::Finished,
					}
				},
				DelegatedStakingStage::Finished => {
					break;
				},
			};
		}

		if messages.len() > 0 {
			Pallet::<T>::send_chunked_xcm_and_track(
				messages,
				|messages| types::AhMigratorCall::<T>::ReceiveDelegatedStakingMessages { messages },
				|len| T::AhWeightInfo::receive_delegated_staking_messages(len),
			)?;
		}

		if last_key == DelegatedStakingStage::Finished {
			Ok(None)
		} else {
			Ok(Some(last_key))
		}
	}
}
