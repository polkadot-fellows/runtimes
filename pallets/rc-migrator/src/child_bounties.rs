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
use pallet_child_bounties::ChildBountyStatus;
use sp_runtime::traits::BlockNumberProvider;

pub type BalanceOf<T, I = ()> = pallet_treasury::BalanceOf<T, I>;

/// Stages that the `ChildBountiesMigrator` will go through in linear order.
#[derive(
	Encode,
	Decode,
	Clone,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	DecodeWithMemTracking,
	PartialEq,
	Eq,
)]
pub enum ChildBountiesStage {
	#[default]
	ChildBountyCount,
	ParentChildBounties {
		parent_id: Option<BountyIndex>,
	},
	ParentTotalChildBounties {
		parent_id: Option<BountyIndex>,
	},
	ChildBounties {
		ids: Option<(BountyIndex, BountyIndex)>,
	},
	ChildBountyDescriptionsV1 {
		ids: Option<(BountyIndex, BountyIndex)>,
	},
	V0ToV1ChildBountyIds {
		child_id: Option<BountyIndex>,
	},
	ChildrenCuratorFees {
		child_id: Option<BountyIndex>,
	},
	Finished,
}

/// Child bounties data message to migrate some data from RC to AH.
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq, DecodeWithMemTracking)]
pub enum PortableChildBountiesMessage {
	ChildBountyCount(BountyIndex),
	ParentChildBounties(BountyIndex, u32),
	ParentTotalChildBounties(BountyIndex, u32),
	ChildBounty {
		parent_id: BountyIndex,
		child_id: BountyIndex,
		child_bounty: PortableChildBounty,
	},
	ChildBountyDescriptionsV1 {
		parent_id: BountyIndex,
		child_id: BountyIndex,
		description: Vec<u8>,
	},
	V0ToV1ChildBountyIds {
		v0_child_id: BountyIndex,
		parent_id: BountyIndex,
		v1_child_id: BountyIndex,
	},
	ChildrenCuratorFees {
		child_id: BountyIndex,
		amount: u128,
	},
}

pub struct ChildBountiesMigrator<T> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for ChildBountiesMigrator<T>
where
	<<T as pallet_treasury::Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber: Into<u32>
{
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
			if messages.len() > 1_000 {
				log::warn!(target: LOG_TARGET, "Weight allowed very big batch, stopping");
				break;
			}

			last_key = match last_key {
				ChildBountiesStage::ChildBountyCount => {
					let count = pallet_child_bounties::ChildBountyCount::<T>::take();
					messages.push(PortableChildBountiesMessage::ChildBountyCount(count));

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
							pallet_child_bounties::ParentChildBounties::<T>::remove(&key);
							messages.push(PortableChildBountiesMessage::ParentChildBounties(
								key, value,
							));
							ChildBountiesStage::ParentChildBounties { parent_id: Some(key) }
						},
						None => ChildBountiesStage::ChildBounties { ids: None },
					}
				},
				ChildBountiesStage::ParentTotalChildBounties { parent_id } => {
					let mut iter = if let Some(parent_id) = parent_id {
						pallet_child_bounties::ParentTotalChildBounties::<T>::iter_from_key(
							parent_id,
						)
					} else {
						pallet_child_bounties::ParentTotalChildBounties::<T>::iter()
					};

					match iter.next() {
						Some((key, value)) => {
							pallet_child_bounties::ParentTotalChildBounties::<T>::remove(&key);
							messages.push(PortableChildBountiesMessage::ParentTotalChildBounties(
								key, value,
							));
							ChildBountiesStage::ParentTotalChildBounties { parent_id: Some(key) }
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
							pallet_child_bounties::ChildBounties::<T>::remove(
								&parent_id, &child_id,
							);
							messages.push(PortableChildBountiesMessage::ChildBounty {
								parent_id,
								child_id,
								child_bounty: child_bounty.into_portable(),
							});
							ChildBountiesStage::ChildBounties { ids: Some((parent_id, child_id)) }
						},
						None => ChildBountiesStage::ChildBountyDescriptionsV1 { ids: None },
					}
				},
				ChildBountiesStage::ChildBountyDescriptionsV1 { ids } => {
					let mut iter = if let Some((parent_id, child_id)) = ids {
						pallet_child_bounties::ChildBountyDescriptionsV1::<T>::iter_from(
							pallet_child_bounties::ChildBountyDescriptionsV1::<T>::hashed_key_for(
								&parent_id, &child_id,
							),
						)
					} else {
						pallet_child_bounties::ChildBountyDescriptionsV1::<T>::iter()
					};

					match iter.next() {
						Some((parent_id, child_id, description)) => {
							pallet_child_bounties::ChildBountyDescriptionsV1::<T>::remove(
								&parent_id, &child_id,
							);
							messages.push(
								PortableChildBountiesMessage::ChildBountyDescriptionsV1 {
									parent_id,
									child_id,
									description: description.into_inner(),
								},
							);
							ChildBountiesStage::ChildBountyDescriptionsV1 {
								ids: Some((parent_id, child_id)),
							}
						},
						None => ChildBountiesStage::ChildrenCuratorFees { child_id: None },
					}
				},
				ChildBountiesStage::V0ToV1ChildBountyIds { child_id } => {
					let mut iter = if let Some(child_id) = child_id {
						pallet_child_bounties::V0ToV1ChildBountyIds::<T>::iter_from_key(child_id)
					} else {
						pallet_child_bounties::V0ToV1ChildBountyIds::<T>::iter()
					};

					match iter.next() {
						Some((v0_child_id, (parent_id, v1_child_id))) => {
							pallet_child_bounties::V0ToV1ChildBountyIds::<T>::remove(&v0_child_id);
							messages.push(PortableChildBountiesMessage::V0ToV1ChildBountyIds {
								v0_child_id,
								parent_id,
								v1_child_id,
							});
							ChildBountiesStage::V0ToV1ChildBountyIds { child_id: Some(v1_child_id) }
						},
						None => ChildBountiesStage::Finished,
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
							pallet_child_bounties::ChildrenCuratorFees::<T>::remove(&child_id);
							messages.push(PortableChildBountiesMessage::ChildrenCuratorFees {
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

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct PortableChildBounty {
	pub parent_bounty: BountyIndex,
	pub value: u128,
	pub fee: u128,
	pub curator_deposit: u128,
	pub status: PortableChildBountyStatus,
}

// PortableChildBounty: RC -> Portable
impl<BlockNumber: Into<u32>> IntoPortable for ChildBounty<AccountId32, u128, BlockNumber> {
	type Portable = PortableChildBounty;

	fn into_portable(self) -> Self::Portable {
		PortableChildBounty {
			parent_bounty: self.parent_bounty,
			value: self.value,
			fee: self.fee,
			curator_deposit: self.curator_deposit,
			status: self.status.into_portable(),
		}
	}
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum PortableChildBountyStatus {
	Added,
	CuratorProposed { curator: AccountId32 },
	Active { curator: AccountId32 },
	PendingPayout { curator: AccountId32, beneficiary: AccountId32, unlock_at: u32 },
}

// PortableChildBountyStatus: RC -> Portable
impl<BlockNumber: Into<u32>> IntoPortable for ChildBountyStatus<AccountId32, BlockNumber> {
	type Portable = PortableChildBountyStatus;

	fn into_portable(self) -> Self::Portable {
		use PortableChildBountyStatus::*;

		match self {
			ChildBountyStatus::Added => Added,
			ChildBountyStatus::CuratorProposed { curator } => CuratorProposed { curator },
			ChildBountyStatus::Active { curator } => Active { curator },
			ChildBountyStatus::PendingPayout { curator, beneficiary, unlock_at } =>
				PendingPayout { curator, beneficiary, unlock_at: unlock_at.into() },
		}
	}
}
