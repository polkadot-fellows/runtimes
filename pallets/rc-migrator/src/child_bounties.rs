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

use crate::*;
use pallet_bounties::{Bounty, BountyIndex};
use pallet_child_bounties::ChildBounty;

pub type BalanceOf<T, I = ()> = pallet_treasury::BalanceOf<T, I>;

/// Stages that the `ChildBountiesMigrator` will go through in linear order.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub enum ChildBountiesStage {
	#[default]
	ChildBountyCount,
	ParentChildBounties {
		parent_id: Option<BountyIndex>,
	},
	// Not yet available in 2409, TODO https://github.com/polkadot-fellows/runtimes/pull/606
	/*ParentTotalChildBounties {
		parent_id: Option<BountyIndex>,
	},*/
	ChildBounties {
		ids: Option<(BountyIndex, BountyIndex)>,
	},
	ChildBountyDescriptionsV1 {
		ids: Option<(BountyIndex, BountyIndex)>,
	},
	/*V0ToV1ChildBountyIds {
		child_id: Option<BountyIndex>,
	},*/
	ChildrenCuratorFees {
		child_id: Option<BountyIndex>,
	},
	Finished,
}

/// Child bounties data message to migrate some data from RC to AH.
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub enum RcChildBountiesMessage<AccountId, Balance, BlockNumber> {
	ChildBountyCount(BountyIndex),
	ParentChildBounties(BountyIndex, u32),
	ParentTotalChildBounties(BountyIndex, u32),
	ChildBounty {
		parent_id: BountyIndex,
		child_id: BountyIndex,
		child_bounty: ChildBounty<AccountId, Balance, BlockNumber>,
	},
	ChildBountyDescriptionsV1 {
		parent_id: BountyIndex,
		child_id: BountyIndex,
		description: Vec<u8>,
	},
	/*V0ToV1ChildBountyIds {
		v0_child_id: BountyIndex,
		parent_id: BountyIndex,
		v1_child_id: BountyIndex,
	},*/
	ChildrenCuratorFees {
		child_id: BountyIndex,
		amount: Balance,
	},
}

pub type RcChildBountiesMessageOf<T> =
	RcChildBountiesMessage<<T as frame_system::Config>::AccountId, BalanceOf<T>, BlockNumberFor<T>>;

pub struct ChildBountiesMigrator<T> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for ChildBountiesMigrator<T> {
	type Key = ChildBountiesStage;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut last_key = last_key.unwrap_or_default();
		let mut messages = Vec::new();

		log::info!(target: LOG_TARGET, "Migrating ChildBounties at stage {:?} with weight limit {:?}", &last_key, &weight_counter.limit());

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
				log::warn!(target: LOG_TARGET, "Weight allowed very big batch, stopping");
				break;
			}

			last_key = match last_key {
				ChildBountiesStage::ChildBountyCount => {
					let count = pallet_child_bounties::ChildBountyCount::<T>::take();
					log::debug!(target: LOG_TARGET, "Migration ChildBountyCount {:?}", &count);
					messages.push(RcChildBountiesMessage::ChildBountyCount(count));
					ChildBountiesStage::ParentChildBounties { parent_id: None }
				},
				ChildBountiesStage::ParentChildBounties { parent_id } => {
					let mut iter = if let Some(parent_id) = parent_id {
						pallet_child_bounties::ParentChildBounties::<T>::iter_from_key(parent_id)
					} else {
						pallet_child_bounties::ParentChildBounties::<T>::iter()
					};

					match iter.next() {
						Some((key, value)) => {
							log::debug!(target: LOG_TARGET, "Migration ParentChildBounties key {:?}", &key);
							pallet_child_bounties::ParentChildBounties::<T>::remove(&key);
							messages.push(RcChildBountiesMessage::ParentChildBounties(key, value));
							ChildBountiesStage::ParentChildBounties { parent_id: Some(key) }
						},
						None => ChildBountiesStage::ChildBounties { ids: None },
					}
				},
				ChildBountiesStage::ChildBounties { ids } => {
					let mut iter = if let Some((parent_id, child_id)) = ids {
						pallet_child_bounties::ChildBounties::<T>::iter_from(
							pallet_child_bounties::ChildBounties::<T>::hashed_key_for(
								&parent_id, &child_id,
							),
						)
					} else {
						pallet_child_bounties::ChildBounties::<T>::iter()
					};

					match iter.next() {
						Some((parent_id, child_id, child_bounty)) => {
							log::debug!(target: LOG_TARGET, "Migration ChildBounties key {:?}", &parent_id);
							pallet_child_bounties::ChildBounties::<T>::remove(
								&parent_id, &child_id,
							);
							messages.push(RcChildBountiesMessage::ChildBounty {
								parent_id,
								child_id,
								child_bounty,
							});
							ChildBountiesStage::ChildBounties { ids: Some((parent_id, child_id)) }
						},
						None => ChildBountiesStage::ChildBountyDescriptionsV1 { ids: None },
					}
				},
				ChildBountiesStage::ChildBountyDescriptionsV1 { ids } => {
					let mut iter = if let Some((_parent_id, child_id)) = ids {
						// TODO should be V1 after https://github.com/polkadot-fellows/runtimes/pull/606
						pallet_child_bounties::ChildBountyDescriptions::<T>::iter_from(
							pallet_child_bounties::ChildBountyDescriptions::<T>::hashed_key_for(
								&child_id,
							),
						)
					} else {
						pallet_child_bounties::ChildBountyDescriptions::<T>::iter()
					};

					match iter.next() {
						Some((child_id, description)) => {
							log::debug!(target: LOG_TARGET, "Migration ChildBountyDescriptionsV1 key {:?}", &child_id);
							pallet_child_bounties::ChildBountyDescriptions::<T>::remove(&child_id);
							messages.push(RcChildBountiesMessage::ChildBountyDescriptionsV1 {
								parent_id: 0,
								child_id,
								description: description.into_inner(),
							});
							ChildBountiesStage::ChildBountyDescriptionsV1 {
								ids: Some((0, child_id)),
							} // TODO
						},
						None => ChildBountiesStage::ChildrenCuratorFees { child_id: None },
					}
				},
				ChildBountiesStage::ChildrenCuratorFees { child_id } => {
					let mut iter = match child_id {
						Some(child_id) =>
							pallet_child_bounties::ChildrenCuratorFees::<T>::iter_from(
								pallet_child_bounties::ChildrenCuratorFees::<T>::hashed_key_for(
									&child_id,
								),
							),
						None => pallet_child_bounties::ChildrenCuratorFees::<T>::iter(),
					};

					match iter.next() {
						Some((child_id, amount)) => {
							log::debug!(target: LOG_TARGET, "Migration ChildrenCuratorFees key {:?}", &child_id);
							pallet_child_bounties::ChildrenCuratorFees::<T>::remove(&child_id);
							messages.push(RcChildBountiesMessage::ChildrenCuratorFees {
								child_id,
								amount,
							});
							ChildBountiesStage::ChildrenCuratorFees { child_id: Some(child_id) }
						},
						None => ChildBountiesStage::Finished,
					}
				},
				ChildBountiesStage::Finished => {
					break;
				},
			};
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm(messages, |messages| {
				types::AhMigratorCall::<T>::ReceiveChildBountiesMessages { messages }
			})?;
		}

		if last_key == ChildBountiesStage::Finished {
			log::info!(target: LOG_TARGET, "ChildBounties migration finished");
			Ok(None)
		} else {
			log::info!(
				target: LOG_TARGET,
				"ChildBounties migration iteration stopped at {:?}",
				&last_key
			);
			Ok(Some(last_key))
		}
	}
}

pub mod alias {
	use super::*;
	use pallet_child_bounties::ChildBountyStatus;

	pub type BalanceOf<T> = pallet_treasury::BalanceOf<T>;
	// TODO swap with treasury BNP after https://github.com/polkadot-fellows/runtimes/pull/606
	pub type BlockNumberFor<T> = frame_system::pallet_prelude::BlockNumberFor<T>;
	pub type ChildBountyOf<T> =
		ChildBounty<<T as frame_system::Config>::AccountId, BalanceOf<T>, BlockNumberFor<T>>;

	// Copied forom https://github.com/paritytech/polkadot-sdk/blob/a8722784fb36e13c811605bd5631d78643273e24/substrate/frame/child-bounties/src/lib.rs#L99-L111
	#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct ChildBounty<AccountId, Balance, BlockNumber> {
		/// The parent of this child-bounty.
		pub parent_bounty: BountyIndex,
		/// The (total) amount that should be paid if this child-bounty is rewarded.
		pub value: Balance,
		/// The child bounty curator fee.
		pub fee: Balance,
		/// The deposit of child-bounty curator.
		pub curator_deposit: Balance,
		/// The status of this child-bounty.
		pub status: ChildBountyStatus<AccountId, BlockNumber>,
	}
}
