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
	alias as delegated_staking_alias, RcDelegatedStakingMessage, RcDelegatedStakingMessageOf,
};

impl<T: Config> Pallet<T> {
	pub fn do_receive_delegated_staking_messages(
		messages: Vec<RcDelegatedStakingMessageOf<T>>,
	) -> DispatchResult {
		log::info!(target: LOG_TARGET, "Processing {} delegated staking messages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::DelegatedStaking,
			count: messages.len() as u32,
		});

		let (mut count_good, mut count_bad) = (0, 0);

		for message in messages {
			match Self::do_process_delegated_staking_message(message) {
				Ok(()) => count_good += 1,
				Err(_) => count_bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Treasury,
			count_good,
			count_bad,
		});
		log::info!(target: LOG_TARGET, "Processed {} delegated staking messages", count_good);

		Ok(())
	}

	fn do_process_delegated_staking_message(
		message: RcDelegatedStakingMessageOf<T>,
	) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Processing delegated staking message: {:?}", message);

		match message {
			RcDelegatedStakingMessage::Delegators { delegator, agent, amount } => {
				let delegation = delegated_staking_alias::Delegation { agent, amount };
				delegated_staking_alias::Delegations::<T>::insert(delegator, delegation);
			},
			RcDelegatedStakingMessage::Agents {
				agent,
				payee,
				total_delegated,
				unclaimed_withdrawals,
				pending_slash,
			} => {
				let agent_ledger = delegated_staking_alias::AgentLedger {
					payee,
					total_delegated,
					unclaimed_withdrawals,
					pending_slash,
				};
				delegated_staking_alias::AgentLedgers::<T>::insert(agent, agent_ledger);
			},
		}

		Ok(())
	}
}
