// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Fast unstake migration logic.

use crate::*;
use frame_election_provider_support::SortedListProvider;
use pallet_rc_migrator::staking::bags_list::{alias, BagsListMigrator, GenericBagsListMessage};

impl<T: Config> Pallet<T> {
	pub fn do_receive_bags_list_messages(
		messages: Vec<RcBagsListMessage<T>>,
	) -> Result<(), Error<T>> {
		let (mut good, bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} BagsListMessages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::BagsList,
			count: messages.len() as u32,
		});

		// Collect all nodes with translated accounts and their scores
		let mut nodes_to_insert: Vec<(T::AccountId, T::Score)> = Vec::new();

		for message in messages {
			match message {
				RcBagsListMessage::Node { id, node } => {
					let translated_id = Self::translate_account_rc_to_ah(id);
					nodes_to_insert.push((translated_id, node.score));
					good += 1;
				},
				RcBagsListMessage::Bag { .. } => {
					// Bags will be automatically created when nodes are inserted
					// TODO: Should I increment "good" or not? We are handling the message, but we
					// are not processing it since the bag will be recreated automatically when
					// nodes are inserted.
					good += 1;
				},
			}
		}

		// Now rebuild the bags list structure properly using the pallet's methods
		if !nodes_to_insert.is_empty() {
			Self::rebuild_bags_list(nodes_to_insert)?;
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::BagsList,
			count_good: good as u32,
			count_bad: bad as u32,
		});

		Ok(())
	}

	/// Rebuild the bags list structure using the pallet's proper methods
	/// This ensures correct sorting and prev/next relationships after account translation
	fn rebuild_bags_list(nodes: Vec<(T::AccountId, T::Score)>) -> Result<(), Error<T>> {
		log::info!(target: LOG_TARGET, "Rebuilding bags list with {} nodes", nodes.len());

		let results: Result<Vec<_>, _> = nodes
			.into_iter()
			.map(|(account, score)| {
				<pallet_bags_list::Pallet<T, pallet_bags_list::Instance1> as SortedListProvider<T::AccountId>>::on_insert(account.clone(), score)
					.map_err(|_| {
						log::error!(target: LOG_TARGET, "Failed to insert account {:?} with score {:?}", account, score);
						Error::<T>::FailedToInsertIntoBagsList
					})
					.map(|_| {
						log::debug!(target: LOG_TARGET, "Inserted account {:?} with score {:?}", account, score);
					})
			})
			.collect();

		results?;
		log::info!(target: LOG_TARGET, "Successfully rebuilt bags list structure");
		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for BagsListMigrator<T> {
	type RcPrePayload = Vec<GenericBagsListMessage<T::AccountId, T::Score>>;
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// Assert storage "VoterList::ListNodes::ah_pre::empty"
		assert!(
			alias::ListNodes::<T>::iter().next().is_none(),
			"VoterList::ListNodes::ah_pre::empty"
		);

		// Assert storage "VoterList::ListBags::ah_pre::empty"
		assert!(
			alias::ListBags::<T>::iter().next().is_none(),
			"VoterList::ListBags::ah_pre::empty"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		assert!(!rc_pre_payload.is_empty(), "RC pre-payload should not be empty during post_check");

		let rc_pre_translated: Vec<GenericBagsListMessage<T::AccountId, T::Score>> = rc_pre_payload
			.into_iter()
			.map(|message| {
				match message {
					GenericBagsListMessage::Node { id, node } => {
						let translated_id = Pallet::<T>::translate_account_rc_to_ah(id);
						let translated_node_id = Pallet::<T>::translate_account_rc_to_ah(node.id);
						let translated_prev = node
							.prev
							.map(|account| Pallet::<T>::translate_account_rc_to_ah(account));
						let translated_next = node
							.next
							.map(|account| Pallet::<T>::translate_account_rc_to_ah(account));

						GenericBagsListMessage::Node {
							id: translated_id,
							node: alias::Node {
								id: translated_node_id,
								prev: translated_prev,
								next: translated_next,
								bag_upper: node.bag_upper,
								score: node.score,
							},
						}
					},
					GenericBagsListMessage::Bag { score, bag } => {
						// Directly translate all AccountId fields - no need for encode/decode
						// cycles
						let translated_head = bag
							.head
							.map(|account| Pallet::<T>::translate_account_rc_to_ah(account));
						let translated_tail = bag
							.tail
							.map(|account| Pallet::<T>::translate_account_rc_to_ah(account));

						GenericBagsListMessage::Bag {
							score,
							bag: alias::Bag { head: translated_head, tail: translated_tail },
						}
					},
				}
			})
			.collect();

		let mut ah_messages = Vec::new();

		// Collect current state
		for (id, node) in alias::ListNodes::<T>::iter() {
			ah_messages.push(GenericBagsListMessage::Node {
				id: id.clone(),
				node: alias::Node {
					id: node.id,
					prev: node.prev,
					next: node.next,
					bag_upper: node.bag_upper,
					score: node.score,
				},
			});
		}

		for (score, bag) in alias::ListBags::<T>::iter() {
			ah_messages.push(GenericBagsListMessage::Bag {
				score,
				bag: alias::Bag { head: bag.head, tail: bag.tail },
			});
		}

		// Assert storage "VoterList::ListBags::ah_post::length"
		// Assert storage "VoterList::ListBags::ah_post::length"
		assert_eq!(
			rc_pre_translated.len(), ah_messages.len(),
			"Bags list length mismatch: Asset Hub data length differs from original Relay Chain data"
		);

		// Assert storage "VoterList::ListNodes::ah_post::correct"
		// Assert storage "VoterList::ListNodes::ah_post::consistent"
		// Assert storage "VoterList::ListBags::ah_post::correct"
		// Assert storage "VoterList::ListBags::ah_post::consistent"
		assert_eq!(
			rc_pre_translated, ah_messages,
			"Bags list data mismatch: Asset Hub data differs from original Relay Chain data"
		);
	}
}
