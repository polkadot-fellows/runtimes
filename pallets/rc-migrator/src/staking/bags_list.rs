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

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum BagsListStage<AccountId, Score> {
	ListNodes(Option<AccountId>),
	ListBags(Option<Score>),
	Finished,
}

pub type BagsListStageOf<T> = BagsListStage<<T as frame_system::Config>::AccountId, <T as pallet_bags_list::Config>::Score>;

#[derive(
	Encode,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	RuntimeDebugNoBound,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
)]
#[codec(mel_bound(T: Config))]
#[scale_info(skip_type_params(T))]
pub enum RcBagsListMessage<T: pallet_bags_list::Config> {
	Node { node: alias::NodeOf<T> },
	Bag { bag: alias::BagOf<T> },
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
		let mut messages = Vec::new();

		loop {
			if weight_counter
				.try_consume(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))
				.is_err()
			{
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
				BagsListStage::ListNodes(next) => {
					let mut iter = match next {
						Some(next) => alias::ListNodes::<T>::iter_from(alias::ListNodes::<T>::hashed_key_for(next)),
						None => alias::ListNodes::<T>::iter(),
					};
					
					match iter.next() {
						Some((key, node)) => {
							alias::ListNodes::<T>::remove(&key);
							messages.push(RcBagsListMessage::Node { node });
							BagsListStage::ListNodes(Some(key))
						},
						None => BagsListStage::ListBags(None),
					}
				},
				BagsListStage::ListBags(next) => {
					let mut iter = match next {
						Some(next) => alias::ListBags::<T>::iter_from(alias::ListBags::<T>::hashed_key_for(next)),
						None => alias::ListBags::<T>::iter(),
					};

					match iter.next() {
						Some((key, bag)) => {
							alias::ListBags::<T>::remove(&key);
							messages.push(RcBagsListMessage::Bag { bag });
							BagsListStage::ListBags(Some(key))
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
			Pallet::<T>::send_chunked_xcm(messages, |messages| {
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

pub mod alias {
	use super::*;

	// From https://github.com/paritytech/polkadot-sdk/blob/7ecf3f757a5d6f622309cea7f788e8a547a5dce8/substrate/frame/bags-list/src/list/mod.rs#L818-L830 minus all the stuff that we don't need
	#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, RuntimeDebug)]
	pub struct Node<AccountId, Score> {
		pub id: AccountId,
		pub prev: Option<AccountId>,
		pub next: Option<AccountId>,
		pub bag_upper: Score,
		pub score: Score,
	}
	pub type NodeOf<T> = Node<<T as frame_system::Config>::AccountId, <T as pallet_bags_list::Config>::Score>;

	// From https://github.com/paritytech/polkadot-sdk/blob/7ecf3f757a5d6f622309cea7f788e8a547a5dce8/substrate/frame/bags-list/src/list/mod.rs#L622-L630
	#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq, RuntimeDebug)]
	pub struct Bag<AccountId> {
		pub head: Option<AccountId>,
		pub tail: Option<AccountId>,
	}
	pub type BagOf<T> = Bag<<T as frame_system::Config>::AccountId>;

	// From https://github.com/paritytech/polkadot-sdk/blob/6c3219ebe9231a0305f53c7b33cb558d46058062/substrate/frame/bags-list/src/lib.rs#L255-L257
	#[frame_support::storage_alias(pallet_name)]
	pub type ListNodes<T: Config> =
		CountedStorageMap<pallet_bags_list::Pallet<T, ()>, Twox64Concat, <T as frame_system::Config>::AccountId, NodeOf<T>>;

	// From https://github.com/paritytech/polkadot-sdk/blob/6c3219ebe9231a0305f53c7b33cb558d46058062/substrate/frame/bags-list/src/lib.rs#L262-L264
	#[frame_support::storage_alias(pallet_name)]
	pub type ListBags<T: Config> =
		StorageMap<pallet_bags_list::Pallet<T, ()>, Twox64Concat, <T as pallet_bags_list::Config>::Score, BagOf<T>>;
}
