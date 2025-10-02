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
use pallet_rc_migrator::{
	staking::bags_list::{BagsListMigrator, PortableBagsListMessage},
	types::SortByEncoded,
};

type I = pallet_bags_list::Instance1;

impl<T: Config> Pallet<T> {
	pub fn do_receive_bags_list_messages(
		messages: Vec<PortableBagsListMessage>,
	) -> Result<(), Error<T>> {
		let (mut good, mut bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} BagsListMessages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::BagsList,
			count: messages.len() as u32,
		});

		// Lock bags-list to prevent any score updates during migration
		if !messages.is_empty() && pallet_bags_list::ListNodes::<T, I>::count() == 0 {
			log::info!(target: LOG_TARGET, "Locking bags-list for migration");
			<pallet_bags_list::Pallet<T, I> as SortedListProvider<T::AccountId>>::lock();
		}

		// Use direct translation instead of rebuilding to preserve exact structure.
		// Rebuilding with SortedListProvider::on_insert changes the insertion order within bags
		// (nodes are added to tail), creating different prev/next relationships even with
		// identical scores. This breaks post-check validation which expects structural match.
		for message in messages {
			match Self::do_receive_bags_list_message(message) {
				Ok(_) => good += 1,
				Err(_) => bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::BagsList,
			count_good: good as u32,
			count_bad: bad as u32,
		});

		Ok(())
	}

	pub fn do_receive_bags_list_message(message: PortableBagsListMessage) -> Result<(), Error<T>> {
		match message {
			PortableBagsListMessage::Node { id, node } => {
				let translated_id = Self::translate_account_rc_to_ah(id);
				debug_assert!(!pallet_bags_list::ListNodes::<T, I>::contains_key(&translated_id));

				// Translate all AccountId fields in the node structure
				let translated_node = pallet_bags_list::Node {
					id: Self::translate_account_rc_to_ah(node.id),
					prev: node.prev.map(Self::translate_account_rc_to_ah),
					next: node.next.map(Self::translate_account_rc_to_ah),
					bag_upper: node.bag_upper,
					score: node.score,
					_phantom: Default::default(),
				};

				pallet_bags_list::ListNodes::<T, I>::insert(&translated_id, &translated_node);
				log::debug!(target: LOG_TARGET, "Integrating BagsListNode: {:?}", &translated_id);
			},
			PortableBagsListMessage::Bag { score, bag } => {
				debug_assert!(!pallet_bags_list::ListBags::<T, I>::contains_key(score));

				// Translate all AccountId fields in the bag structure
				let translated_bag = pallet_bags_list::Bag {
					head: bag.head.map(Self::translate_account_rc_to_ah),
					tail: bag.tail.map(Self::translate_account_rc_to_ah),
					bag_upper: bag.bag_upper,
					_phantom: Default::default(),
				};

				pallet_bags_list::ListBags::<T, I>::insert(score, &translated_bag);
				log::debug!(target: LOG_TARGET, "Integrating BagsListBag: {:?}", &score);
			},
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for BagsListMigrator<T> {
	type RcPrePayload = Vec<PortableBagsListMessage>;
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// Assert storage "VoterList::ListNodes::ah_pre::empty"
		assert!(
			pallet_bags_list::ListNodes::<T, I>::iter().next().is_none(),
			"VoterList::ListNodes::ah_pre::empty"
		);

		// Assert storage "VoterList::ListBags::ah_pre::empty"
		assert!(
			pallet_bags_list::ListBags::<T, I>::iter().next().is_none(),
			"VoterList::ListBags::ah_pre::empty"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		assert!(!rc_pre_payload.is_empty(), "RC pre-payload should not be empty during post_check");

		let mut rc_pre_translated: Vec<PortableBagsListMessage> = rc_pre_payload
			.into_iter()
			.map(|message| {
				match message {
					PortableBagsListMessage::Node { id, node } => {
						let translated_id = Pallet::<T>::translate_account_rc_to_ah(id);
						let translated_node_id = Pallet::<T>::translate_account_rc_to_ah(node.id);
						let translated_prev =
							node.prev.map(Pallet::<T>::translate_account_rc_to_ah);
						let translated_next =
							node.next.map(Pallet::<T>::translate_account_rc_to_ah);

						PortableBagsListMessage::Node {
							id: translated_id,
							node: pallet_rc_migrator::staking::bags_list::PortableNode {
								id: translated_node_id,
								prev: translated_prev,
								next: translated_next,
								bag_upper: node.bag_upper,
								score: node.score,
							},
						}
					},
					PortableBagsListMessage::Bag { score, bag } => {
						// Directly translate all AccountId fields - no need for encode/decode
						// cycles
						let translated_head = bag.head.map(Pallet::<T>::translate_account_rc_to_ah);
						let translated_tail = bag.tail.map(Pallet::<T>::translate_account_rc_to_ah);

						PortableBagsListMessage::Bag {
							score,
							bag: pallet_rc_migrator::staking::bags_list::PortableBag {
								head: translated_head,
								tail: translated_tail,
								bag_upper: bag.bag_upper,
							},
						}
					},
				}
			})
			.collect();
		rc_pre_translated.sort_by_encoded();
		assert!(
			!rc_pre_translated.is_empty(),
			"RC pre-payload should not be empty during post_check"
		);

		let mut ah_messages = Vec::new();

		// Collect current state
		for (id, node) in pallet_bags_list::ListNodes::<T, I>::iter() {
			ah_messages.push(PortableBagsListMessage::Node {
				id: id.clone(),
				node: pallet_rc_migrator::staking::bags_list::PortableNode {
					id: node.id,
					prev: node.prev,
					next: node.next,
					bag_upper: node.bag_upper,
					score: node.score,
				},
			});
		}

		for (score, bag) in pallet_bags_list::ListBags::<T, I>::iter() {
			ah_messages.push(PortableBagsListMessage::Bag {
				score,
				bag: pallet_rc_migrator::staking::bags_list::PortableBag {
					head: bag.head,
					tail: bag.tail,
					bag_upper: bag.bag_upper,
				},
			});
		}
		ah_messages.sort_by_encoded();

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

		// Enhanced diagnostic logging before assertion
		if rc_pre_translated != ah_messages {
			log::error!(
				target: LOG_TARGET,
				"Bags list mismatch detected. Total items - RC: {}, AH: {}",
				rc_pre_translated.len(),
				ah_messages.len()
			);

			let mut node_score_mismatches = 0;
			let mut node_structure_mismatches = 0;
			let mut bag_mismatches = 0;
			let mut first_10_logged = 0;

			for (idx, (rc_msg, ah_msg)) in
				rc_pre_translated.iter().zip(ah_messages.iter()).enumerate()
			{
				match (rc_msg, ah_msg) {
					(
						PortableBagsListMessage::Node { id: rc_id, node: rc_node },
						PortableBagsListMessage::Node { id: ah_id, node: ah_node },
					) =>
						if rc_id != ah_id || rc_node.id != ah_node.id {
							node_structure_mismatches += 1;
							if first_10_logged < 10 {
								log::error!(
									target: LOG_TARGET,
									"Node ID mismatch at index {}: RC={:?}, AH={:?}",
									idx,
									rc_id,
									ah_id
								);
								first_10_logged += 1;
							}
						} else if rc_node.score != ah_node.score {
							node_score_mismatches += 1;
							if first_10_logged < 10 {
								log::error!(
									target: LOG_TARGET,
									"Score mismatch for node {:?} at index {}: RC_score={}, AH_score={}, diff={}",
									rc_id,
									idx,
									rc_node.score,
									ah_node.score,
									(rc_node.score as i128) - (ah_node.score as i128)
								);
								first_10_logged += 1;
							}
						} else if rc_node.prev != ah_node.prev || rc_node.next != ah_node.next {
							node_structure_mismatches += 1;
							if first_10_logged < 10 {
								log::error!(
									target: LOG_TARGET,
									"Structure mismatch for node {:?} at index {}: RC(prev={:?}, next={:?}), AH(prev={:?}, next={:?})",
									rc_id,
									idx,
									rc_node.prev,
									rc_node.next,
									ah_node.prev,
									ah_node.next
								);
								first_10_logged += 1;
							}
						} else if rc_node.bag_upper != ah_node.bag_upper {
							node_structure_mismatches += 1;
							if first_10_logged < 10 {
								log::error!(
									target: LOG_TARGET,
									"Bag upper mismatch for node {:?} at index {}: RC={}, AH={}",
									rc_id,
									idx,
									rc_node.bag_upper,
									ah_node.bag_upper
								);
								first_10_logged += 1;
							}
						},
					(
						PortableBagsListMessage::Bag { score: rc_score, bag: rc_bag },
						PortableBagsListMessage::Bag { score: ah_score, bag: ah_bag },
					) => {
						if rc_score != ah_score ||
							rc_bag.head != ah_bag.head ||
							rc_bag.tail != ah_bag.tail ||
							rc_bag.bag_upper != ah_bag.bag_upper
						{
							bag_mismatches += 1;
							if first_10_logged < 10 {
								log::error!(
									target: LOG_TARGET,
									"Bag mismatch at index {}: RC(score={}, head={:?}, tail={:?}), AH(score={}, head={:?}, tail={:?})",
									idx,
									rc_score,
									rc_bag.head,
									rc_bag.tail,
									ah_score,
									ah_bag.head,
									ah_bag.tail
								);
								first_10_logged += 1;
							}
						}
					},
					_ => {
						log::error!(
							target: LOG_TARGET,
							"Type mismatch at index {}: RC and AH have different message types",
							idx
						);
					},
				}
			}

			log::error!(
				target: LOG_TARGET,
				"Mismatch summary: {} node score mismatches, {} node structure mismatches, {} bag mismatches",
				node_score_mismatches,
				node_structure_mismatches,
				bag_mismatches
			);

			panic!(
				"Bags list data mismatch: {} node score mismatches, {} node structure mismatches, {} bag mismatches. See logs for details.",
				node_score_mismatches,
				node_structure_mismatches,
				bag_mismatches
			);
		}

		assert_eq!(
			rc_pre_translated, ah_messages,
			"Bags list data mismatch: Asset Hub data differs from original Relay Chain data"
		);

		// Run bags-list pallet integrity check
		#[cfg(feature = "try-runtime")]
		<pallet_bags_list::Pallet<T, I> as frame_election_provider_support::SortedListProvider<
			T::AccountId,
		>>::try_state()
		.expect("Bags list integrity check failed");
	}
}
