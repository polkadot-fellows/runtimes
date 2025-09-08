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

//! Nomination pools data migrator module.

use crate::{types::*, *};

type I = pallet_bags_list::Instance1;

#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum BagsListStage<AccountId, Score> {
	ListNodes(Option<AccountId>),
	ListBags(Option<Score>),
	Finished,
}

pub type BagsListStageOf<T> = BagsListStage<
	<T as frame_system::Config>::AccountId,
	<T as pallet_bags_list::Config<I>>::Score,
>;

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	RuntimeDebugNoBound,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
)]
pub enum PortableBagsListMessage {
	Node { id: AccountId32, node: PortableNode },
	Bag { score: u64, bag: PortableBag },
}

#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
)]
pub struct PortableNode {
	pub id: AccountId32,
	pub prev: Option<AccountId32>,
	pub next: Option<AccountId32>,
	pub bag_upper: u64,
	pub score: u64,
}

impl<T: Config> IntoPortable for pallet_bags_list::Node<T, I> {
	type Portable = PortableNode;

	fn into_portable(self) -> Self::Portable {
		PortableNode {
			id: self.id,
			prev: self.prev,
			next: self.next,
			bag_upper: self.bag_upper,
			score: self.score,
		}
	}
}

impl<T: Config> From<PortableNode> for pallet_bags_list::Node<T, I> {
	fn from(node: PortableNode) -> Self {
		pallet_bags_list::Node {
			id: node.id,
			prev: node.prev,
			next: node.next,
			bag_upper: node.bag_upper,
			score: node.score,
			_phantom: Default::default(),
		}
	}
}

#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
)]
pub struct PortableBag {
	pub head: Option<AccountId32>,
	pub tail: Option<AccountId32>,
	pub bag_upper: u64,
}

impl<T: Config> IntoPortable for pallet_bags_list::Bag<T, I> {
	type Portable = PortableBag;

	fn into_portable(self) -> Self::Portable {
		PortableBag { head: self.head, tail: self.tail, bag_upper: self.bag_upper }
	}
}

impl<T: Config> From<PortableBag> for pallet_bags_list::Bag<T, I> {
	fn from(bag: PortableBag) -> Self {
		pallet_bags_list::Bag {
			head: bag.head,
			tail: bag.tail,
			bag_upper: bag.bag_upper,
			_phantom: Default::default(),
		}
	}
}

pub struct BagsListMigrator<T> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for BagsListMigrator<T> {
	type Key = BagsListStageOf<T>;
	type Error = Error<T>;

	fn migrate_many(
		current_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut inner_key = current_key.unwrap_or(BagsListStage::ListNodes(None));
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
			if T::MaxAhWeight::get()
				.any_lt(T::AhWeightInfo::receive_bags_list_messages((messages.len() + 1) as u32))
			{
				log::info!("AH weight limit reached at batch length {}, stopping", messages.len());
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > MAX_ITEMS_PER_BLOCK {
				log::info!(
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					messages.len()
				);
				break;
			}

			if messages.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					messages.batch_count()
				);
				break;
			}

			inner_key = match inner_key {
				BagsListStage::ListNodes(next) => {
					let mut iter = match next {
						Some(next) => pallet_bags_list::ListNodes::<T, I>::iter_from(
							pallet_bags_list::ListNodes::<T, I>::hashed_key_for(next),
						),
						None => pallet_bags_list::ListNodes::<T, I>::iter(),
					};

					match iter.next() {
						Some((id, node)) => {
							pallet_bags_list::ListNodes::<T, I>::remove(&id);
							messages.push(PortableBagsListMessage::Node {
								id: id.clone(),
								node: node.into_portable(),
							});
							BagsListStage::ListNodes(Some(id))
						},
						None => BagsListStage::ListBags(None),
					}
				},
				BagsListStage::ListBags(next) => {
					let mut iter = match next {
						Some(next) => pallet_bags_list::ListBags::<T, I>::iter_from(
							pallet_bags_list::ListBags::<T, I>::hashed_key_for(next),
						),
						None => pallet_bags_list::ListBags::<T, I>::iter(),
					};

					match iter.next() {
						Some((score, bag)) => {
							pallet_bags_list::ListBags::<T, I>::remove(&score);
							messages.push(PortableBagsListMessage::Bag {
								score: score.clone(),
								bag: bag.into_portable(),
							});
							BagsListStage::ListBags(Some(score))
						},
						None => BagsListStage::Finished,
					}
				},
				BagsListStage::Finished => {
					break;
				},
			}
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(messages, |messages| {
				types::AhMigratorCall::<T>::ReceiveBagsListMessages { messages }
			})?;
		}

		if inner_key == BagsListStage::Finished {
			Ok(None)
		} else {
			Ok(Some(inner_key))
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for BagsListMigrator<T> {
	type RcPrePayload = Vec<PortableBagsListMessage>;

	fn pre_check() -> Self::RcPrePayload {
		let mut messages = Vec::new();

		// Collect ListNodes
		for (id, node) in pallet_bags_list::ListNodes::<T, I>::iter() {
			messages
				.push(PortableBagsListMessage::Node { id: id.clone(), node: node.into_portable() });
		}

		// Collect ListBags
		for (score, bag) in pallet_bags_list::ListBags::<T, I>::iter() {
			messages.push(PortableBagsListMessage::Bag { score, bag: bag.into_portable() });
		}

		messages
	}

	fn post_check(_: Self::RcPrePayload) {
		// Assert storage "VoterList::ListNodes::rc_post::empty"
		assert!(
			pallet_bags_list::ListNodes::<T, I>::iter().next().is_none(),
			"VoterList::ListNodes::rc_post::empty"
		);
		// Assert storage "VoterList::ListBags::rc_post::empty
		assert!(
			pallet_bags_list::ListBags::<T, I>::iter().next().is_none(),
			"VoterList::ListBags::rc_post::empty"
		);

		log::info!("All bags list data successfully migrated and cleared from the Relay Chain.");
	}
}
