// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use pallet_rc_migrator::staking::delegated_staking::{
	test, DelegatedStakingMigrator, PortableDelegatedStakingMessage,
};

impl<T: Config> Pallet<T> {
	pub fn translate_delegated_staking_message(
		message: PortableDelegatedStakingMessage,
	) -> PortableDelegatedStakingMessage {
		match message {
			PortableDelegatedStakingMessage::Delegators { delegator, agent, amount } =>
				PortableDelegatedStakingMessage::Delegators {
					delegator: Self::translate_account_rc_to_ah(delegator),
					agent: Self::translate_account_rc_to_ah(agent),
					amount,
				},
			PortableDelegatedStakingMessage::Agents {
				agent,
				payee,
				total_delegated,
				unclaimed_withdrawals,
				pending_slash,
			} => PortableDelegatedStakingMessage::Agents {
				agent: Self::translate_account_rc_to_ah(agent),
				payee: Self::translate_account_rc_to_ah(payee),
				total_delegated,
				unclaimed_withdrawals,
				pending_slash,
			},
		}
	}

	pub fn do_receive_delegated_staking_messages(
		messages: Vec<PortableDelegatedStakingMessage>,
	) -> DispatchResult {
		log::info!(target: LOG_TARGET, "Processing {} delegated staking messages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::DelegatedStaking,
			count: messages.len() as u32,
		});

		let (mut count_good, mut count_bad) = (0, 0);

		for message in messages {
			let translated_message = Self::translate_delegated_staking_message(message);
			match Self::do_process_delegated_staking_message(translated_message) {
				Ok(()) => count_good += 1,
				Err(_) => count_bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::DelegatedStaking,
			count_good,
			count_bad,
		});
		log::info!(target: LOG_TARGET, "Processed {} delegated staking messages", count_good);

		Ok(())
	}

	fn do_process_delegated_staking_message(
		message: PortableDelegatedStakingMessage,
	) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Processing delegated staking message: {:?}", message);

		match message {
			PortableDelegatedStakingMessage::Delegators { delegator, agent, amount } => {
				let delegation = pallet_delegated_staking::types::Delegation { agent, amount };
				pallet_delegated_staking::Delegators::<T>::insert(delegator, delegation);
			},
			PortableDelegatedStakingMessage::Agents {
				agent,
				payee,
				total_delegated,
				unclaimed_withdrawals,
				pending_slash,
			} => {
				let agent_ledger = pallet_delegated_staking::types::AgentLedger {
					payee,
					total_delegated,
					unclaimed_withdrawals,
					pending_slash,
				};
				pallet_delegated_staking::Agents::<T>::insert(agent, agent_ledger);
			},
		}

		Ok(())
	}
}

impl<T: Config> crate::types::AhMigrationCheck for DelegatedStakingMigrator<T> {
	type RcPrePayload = (Vec<test::RcDelegation>, Vec<test::RcAgentLedger>);
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// Assert storage "Delegations::ah_pre::empty"
		assert!(
			pallet_delegated_staking::Delegators::<T>::iter().next().is_none(),
			"No delegations should exist on the Asset Hub before migration"
		);

		// Assert storage "Agents::ah_pre::empty"
		assert!(
			pallet_delegated_staking::Agents::<T>::iter().next().is_none(),
			"No agent ledgers should exist on the Asset Hub before migration"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		let (delegations, agent_ledgers) = rc_pre_payload;

		// Assert storage "Delegations::ah_post::correct"
		assert_eq!(
			delegations.len(),
			pallet_delegated_staking::Delegators::<T>::iter().count(),
			"Number of delegations on Asset Hub after migration should be the same as on the Relay Chain before migration"
		);

		// Assert storage "Agents::ah_post::correct"
		assert_eq!(
			agent_ledgers.len(),
			pallet_delegated_staking::Agents::<T>::iter().count(),
			"Number of agent ledgers on Asset Hub after migration should be the same as on the Relay Chain before migration"
		);

		// Assert storage "Delegations::ah_post::correct"
		for delegation in delegations {
			let translated_delegator =
				Pallet::<T>::translate_account_rc_to_ah(delegation.delegator.clone());
			let translated_agent =
				Pallet::<T>::translate_account_rc_to_ah(delegation.agent.clone());

			let ah_delegation_maybe =
				pallet_delegated_staking::Delegators::<T>::get(&translated_delegator);
			assert!(
				ah_delegation_maybe.is_some(),
				"Delegation for delegator {:?} should exist on the Asset Hub after migration",
				translated_delegator
			);
			let ah_delegation = ah_delegation_maybe.unwrap();
			assert_eq!(
				ah_delegation.agent,
				translated_agent,
				"Agent for delegation of delegator {:?} should be the same on the Asset Hub after migration",
				translated_delegator
			);
			assert_eq!(
				ah_delegation.amount,
				delegation.amount,
				"Amount for delegation of delegator {:?} should be the same on the Asset Hub after migration",
				translated_delegator
			);
		}

		// Assert storage "Agents::ah_post::correct"
		for agent_ledger in agent_ledgers {
			let translated_agent =
				Pallet::<T>::translate_account_rc_to_ah(agent_ledger.agent.clone());
			let translated_payee =
				Pallet::<T>::translate_account_rc_to_ah(agent_ledger.payee.clone());

			let ah_agent_ledger_maybe =
				pallet_delegated_staking::Agents::<T>::get(&translated_agent);
			assert!(
				ah_agent_ledger_maybe.is_some(),
				"Agent ledger for agent {:?} should exist on the Asset Hub after migration",
				translated_agent
			);
			let ah_agent_ledger = ah_agent_ledger_maybe.unwrap();
			assert_eq!(
				ah_agent_ledger.payee,
				translated_payee,
				"Payee for agent ledger of agent {:?} should be the same on the Asset Hub after migration",
				translated_agent
			);
			assert_eq!(
				ah_agent_ledger.total_delegated,
				agent_ledger.total_delegated,
				"Total delegated for agent ledger of agent {:?} should be the same on the Asset Hub after migration",
				translated_agent
			);
			assert_eq!(
				ah_agent_ledger.unclaimed_withdrawals,
				agent_ledger.unclaimed_withdrawals,
				"Unclaimed withdrawals for agent ledger of agent {:?} should be the same on the Asset Hub after migration",
				translated_agent
			);
			assert_eq!(
				ah_agent_ledger.pending_slash,
				agent_ledger.pending_slash,
				"Pending slash for agent ledger of agent {:?} should be the same on the Asset Hub after migration",
				translated_agent
			);
		}
	}
}
