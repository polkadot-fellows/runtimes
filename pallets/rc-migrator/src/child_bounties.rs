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

use crate::{types::DefensiveTruncateInto, *};
use pallet_bounties::BountyIndex;
use pallet_child_bounties::{ChildBounty, ChildBountyStatus};

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
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	TypeInfo,
	RuntimeDebugNoBound,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
)]
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
		description: BoundedVec<u8, ConstU32<17000>>, // 16 KiB on Polkadot
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

impl<T: Config> PalletMigration for ChildBountiesMigrator<T> {
	type Key = ChildBountiesStage;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut last_key = last_key.unwrap_or_default();
		let mut messages = XcmBatchAndMeter::new_from_config::<T>();

		log::info!(target: LOG_TARGET, "Migrating ChildBounties at stage {:?} with weight limit {:?}", &last_key, &weight_counter.limit());

		loop {
			if weight_counter
				.try_consume(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))
				.is_err() || weight_counter.try_consume(messages.consume_weight()).is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					messages.len()
				);
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if T::MaxAhWeight::get()
				.any_lt(T::AhWeightInfo::receive_child_bounties_messages(messages.len() + 1))
			{
				log::info!(
					target: LOG_TARGET,
					"AH weight limit reached at batch length {}, stopping",
					messages.len()
				);
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > MAX_ITEMS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
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

			last_key = match last_key {
				ChildBountiesStage::ChildBountyCount => {
					// Check if exists to make it idempotent.
					if pallet_child_bounties::ChildBountyCount::<T>::exists() {
						let count = pallet_child_bounties::ChildBountyCount::<T>::take();
						messages.push(PortableChildBountiesMessage::ChildBountyCount(count));
					}

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
							pallet_child_bounties::ParentChildBounties::<T>::remove(key);
							messages.push(PortableChildBountiesMessage::ParentChildBounties(
								key, value,
							));
							ChildBountiesStage::ParentChildBounties { parent_id: Some(key) }
						},
						None => ChildBountiesStage::ParentTotalChildBounties { parent_id: None },
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
							pallet_child_bounties::ParentTotalChildBounties::<T>::remove(key);
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
								parent_id, child_id,
							),
						)
					} else {
						pallet_child_bounties::ChildBounties::<T>::iter()
					};

					match iter.next() {
						Some((parent_id, child_id, child_bounty)) => {
							pallet_child_bounties::ChildBounties::<T>::remove(parent_id, child_id);
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
								parent_id, child_id,
							),
						)
					} else {
						pallet_child_bounties::ChildBountyDescriptionsV1::<T>::iter()
					};

					match iter.next() {
						Some((parent_id, child_id, description)) => {
							pallet_child_bounties::ChildBountyDescriptionsV1::<T>::remove(
								parent_id, child_id,
							);
							let description = description.into_inner().defensive_truncate_into();

							messages.push(
								PortableChildBountiesMessage::ChildBountyDescriptionsV1 {
									parent_id,
									child_id,
									description,
								},
							);
							ChildBountiesStage::ChildBountyDescriptionsV1 {
								ids: Some((parent_id, child_id)),
							}
						},
						None => ChildBountiesStage::V0ToV1ChildBountyIds { child_id: None },
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
							pallet_child_bounties::V0ToV1ChildBountyIds::<T>::remove(v0_child_id);
							messages.push(PortableChildBountiesMessage::V0ToV1ChildBountyIds {
								v0_child_id,
								parent_id,
								v1_child_id,
							});
							ChildBountiesStage::V0ToV1ChildBountyIds { child_id: Some(v1_child_id) }
						},
						None => ChildBountiesStage::ChildrenCuratorFees { child_id: None },
					}
				},
				ChildBountiesStage::ChildrenCuratorFees { child_id } => {
					let mut iter = match child_id {
						Some(child_id) =>
							pallet_child_bounties::ChildrenCuratorFees::<T>::iter_from(
								pallet_child_bounties::ChildrenCuratorFees::<T>::hashed_key_for(
									child_id,
								),
							),
						None => pallet_child_bounties::ChildrenCuratorFees::<T>::iter(),
					};

					match iter.next() {
						Some((child_id, amount)) => {
							pallet_child_bounties::ChildrenCuratorFees::<T>::remove(child_id);
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

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(messages, |messages| {
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

// ChildBounty: RC -> Portable
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

// ChildBounty: Portable -> AH
impl<BlockNumber: From<u32>, Balance: From<u128>> From<PortableChildBounty>
	for ChildBounty<AccountId32, Balance, BlockNumber>
{
	fn from(portable: PortableChildBounty) -> Self {
		ChildBounty {
			parent_bounty: portable.parent_bounty,
			value: portable.value.into(),
			fee: portable.fee.into(),
			curator_deposit: portable.curator_deposit.into(),
			status: portable.status.into(),
		}
	}
}

impl PortableChildBounty {
	/// Apply an account translation function to all accounts.
	pub fn translate_accounts(
		self,
		translate_account: impl Fn(AccountId32) -> AccountId32,
	) -> Self {
		PortableChildBounty { status: self.status.translate_accounts(translate_account), ..self }
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

// ChildBountyStatus: RC -> Portable
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

// ChildBountyStatus: Portable -> AH
impl<BlockNumber: From<u32>> From<PortableChildBountyStatus>
	for ChildBountyStatus<AccountId32, BlockNumber>
{
	fn from(portable: PortableChildBountyStatus) -> Self {
		match portable {
			PortableChildBountyStatus::Added => ChildBountyStatus::Added,
			PortableChildBountyStatus::CuratorProposed { curator } =>
				ChildBountyStatus::CuratorProposed { curator },
			PortableChildBountyStatus::Active { curator } => ChildBountyStatus::Active { curator },
			PortableChildBountyStatus::PendingPayout { curator, beneficiary, unlock_at } =>
				ChildBountyStatus::PendingPayout {
					curator,
					beneficiary,
					unlock_at: unlock_at.into(),
				},
		}
	}
}

impl PortableChildBountyStatus {
	/// Apply an account translation function to all accounts.
	pub fn translate_accounts(
		self,
		translate_account: impl Fn(AccountId32) -> AccountId32,
	) -> Self {
		match self {
			PortableChildBountyStatus::Added => PortableChildBountyStatus::Added,
			PortableChildBountyStatus::CuratorProposed { curator } =>
				PortableChildBountyStatus::CuratorProposed { curator: translate_account(curator) },
			PortableChildBountyStatus::Active { curator } =>
				PortableChildBountyStatus::Active { curator: translate_account(curator) },
			PortableChildBountyStatus::PendingPayout { curator, beneficiary, unlock_at } =>
				PortableChildBountyStatus::PendingPayout {
					curator: translate_account(curator),
					beneficiary: translate_account(beneficiary),
					unlock_at,
				},
		}
	}
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RcData {
	pub child_bounty_count: u32,
	pub parent_child_bounties: Vec<(u32, u32)>,
	pub parent_total_child_bounties: Vec<(u32, u32)>,
	pub child_bounties: Vec<(u32, u32, PortableChildBounty)>,
	pub child_bounty_descriptions_v1: Vec<(u32, u32, Vec<u8>)>,
	pub v0_to_v1_child_bounty_ids: Vec<(u32, (u32, u32))>,
	pub children_curator_fees: Vec<(u32, u128)>,
}

#[cfg(feature = "std")]
pub struct ChildBountiesMigratedCorrectly<T>(PhantomData<T>);

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for ChildBountiesMigratedCorrectly<T> {
	type RcPrePayload = RcData;

	fn pre_check() -> Self::RcPrePayload {
		use pallet_child_bounties::*;

		RcData {
			child_bounty_count: ChildBountyCount::<T>::get(),
			parent_child_bounties: ParentChildBounties::<T>::iter().collect(),
			parent_total_child_bounties: ParentTotalChildBounties::<T>::iter().collect(),
			child_bounties: ChildBounties::<T>::iter()
				.map(|(p, c, b)| (p, c, b.into_portable()))
				.collect(),
			child_bounty_descriptions_v1: ChildBountyDescriptionsV1::<T>::iter()
				.map(|(p, c, d)| (p, c, d.into_inner()))
				.collect(),
			v0_to_v1_child_bounty_ids: V0ToV1ChildBountyIds::<T>::iter().collect(),
			children_curator_fees: ChildrenCuratorFees::<T>::iter().collect(),
		}
	}

	fn post_check(_rc_pre_payload: Self::RcPrePayload) {
		use pallet_child_bounties::*;

		assert_eq!(ChildBountyCount::<T>::get(), 0);
		assert_eq!(ParentChildBounties::<T>::iter().count(), 0);
		assert_eq!(ParentTotalChildBounties::<T>::iter().count(), 0);
		assert_eq!(ChildBounties::<T>::iter().count(), 0);
		assert_eq!(ChildBountyDescriptionsV1::<T>::iter().count(), 0);
		assert_eq!(V0ToV1ChildBountyIds::<T>::iter().count(), 0);
		assert_eq!(ChildrenCuratorFees::<T>::iter().count(), 0);
	}
}
