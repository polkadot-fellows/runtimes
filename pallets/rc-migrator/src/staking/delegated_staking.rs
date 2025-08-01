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
use types::{AccountIdOf, RcMigrationCheck};

/// Stage of the delegated-staking pallet migration.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	PartialEq,
	Eq,
)]
pub enum DelegatedStakingStage<AccountId> {
	Delegators(Option<AccountId>),
	Agents(Option<AccountId>),
	Finished,
}

/// Message that is being sent to the AH Migrator.
#[derive(Encode, Decode, DecodeWithMemTracking, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub enum PortableDelegatedStakingMessage {
	Delegators {
		delegator: AccountId32,
		agent: AccountId32,
		amount: u128,
	},
	Agents {
		agent: AccountId32,
		payee: AccountId32,
		total_delegated: u128,
		unclaimed_withdrawals: u128,
		pending_slash: u128,
	},
}

pub struct DelegatedStakingMigrator<T>(core::marker::PhantomData<T>);

impl<T: Config> PalletMigration for DelegatedStakingMigrator<T> {
	type Key = DelegatedStakingStage<AccountIdOf<T>>;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut last_key = last_key.unwrap_or(DelegatedStakingStage::Delegators(None));
		let mut messages =
			XcmBatchAndMeter::<PortableDelegatedStakingMessage>::new_from_config::<T>();

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
			if messages.len() > 10_000 {
				log::warn!(target: LOG_TARGET, "Weight allowed very big batch, stopping");
				break;
			}

			last_key = match last_key {
				DelegatedStakingStage::Delegators(last_key) => {
					let mut delegators_iter = if let Some(last_key) = last_key.clone() {
						pallet_delegated_staking::Delegators::<T>::iter_from(
							pallet_delegated_staking::Delegators::<T>::hashed_key_for(last_key),
						)
					} else {
						pallet_delegated_staking::Delegators::<T>::iter()
					};
					match delegators_iter.next() {
						Some((key, value)) => {
							pallet_delegated_staking::Delegators::<T>::remove(&key);
							messages.push(PortableDelegatedStakingMessage::Delegators {
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
						pallet_delegated_staking::Agents::<T>::iter_from(
							pallet_delegated_staking::Agents::<T>::hashed_key_for(last_key),
						)
					} else {
						pallet_delegated_staking::Agents::<T>::iter()
					};
					match agents_iter.next() {
						Some((key, value)) => {
							pallet_delegated_staking::Agents::<T>::remove(&key);
							messages.push(PortableDelegatedStakingMessage::Agents {
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
			Pallet::<T>::send_chunked_xcm_and_track(messages, |messages| {
				types::AhMigratorCall::<T>::ReceiveDelegatedStakingMessages { messages }
			})?;
		}

		if last_key == DelegatedStakingStage::Finished {
			Ok(None)
		} else {
			Ok(Some(last_key))
		}
	}
}

pub mod test {
	use super::*;

	// Delegation used in delegators storage item
	#[derive(Debug, PartialEq, Eq, Clone)]
	pub struct RcDelegation {
		pub delegator: AccountId32,
		pub agent: AccountId32,
		pub amount: u128,
	}

	// AgentLedger used in Agents storage item
	#[derive(Debug, PartialEq, Eq, Clone)]
	pub struct RcAgentLedger {
		pub agent: AccountId32,
		pub payee: AccountId32,
		pub total_delegated: u128,
		pub unclaimed_withdrawals: u128,
		pub pending_slash: u128,
	}
}

#[cfg(feature = "std")]
impl<T: Config> RcMigrationCheck for DelegatedStakingMigrator<T> {
	type RcPrePayload = (Vec<test::RcDelegation>, Vec<test::RcAgentLedger>);

	fn pre_check() -> Self::RcPrePayload {
		let mut delegators = Vec::new();
		let mut agent_ledgers = Vec::new();

		for (delegator, delegation) in pallet_delegated_staking::Delegators::<T>::iter() {
			delegators.push(test::RcDelegation {
				delegator: delegator.clone(),
				agent: delegation.agent.clone(),
				amount: delegation.amount,
			});
		}

		for (agent, agent_ledger) in pallet_delegated_staking::Agents::<T>::iter() {
			agent_ledgers.push(test::RcAgentLedger {
				agent: agent.clone(),
				payee: agent_ledger.payee.clone(),
				total_delegated: agent_ledger.total_delegated,
				unclaimed_withdrawals: agent_ledger.unclaimed_withdrawals,
				pending_slash: agent_ledger.pending_slash,
			});
		}

		(delegators, agent_ledgers)
	}

	fn post_check(_: Self::RcPrePayload) {
		// Assert storage "Delegators::rc_post::empty"
		assert!(
			pallet_delegated_staking::Delegators::<T>::iter().next().is_none(),
			"No delegators should exist on the Relay Chain after migration"
		);

		// Assert storage "Agents::rc_post::empty"
		assert!(
			pallet_delegated_staking::Agents::<T>::iter().next().is_none(),
			"No agent ledgers should exist on the Relay Chain after migration"
		);
	}
}
