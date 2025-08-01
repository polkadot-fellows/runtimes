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
use sp_staking::EraIndex;

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
pub enum FastUnstakeStage<AccountId> {
	StorageValues,
	Queue(Option<AccountId>),
	Finished,
}

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
pub enum PortableFastUnstakeMessage {
	StorageValues { values: PortableFastUnstakeStorageValues },
	Queue { member: (AccountId32, u128) },
}

/// All the `StorageValues` from the fast unstake pallet.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	RuntimeDebugNoBound,
)]
pub struct PortableFastUnstakeStorageValues {
	pub head: Option<PortableUnstakeRequest>,
	pub eras_to_check_per_block: Option<u32>,
}

impl<T: Config> FastUnstakeMigrator<T> {
	pub fn take_values() -> PortableFastUnstakeStorageValues {
		PortableFastUnstakeStorageValues {
			head: pallet_fast_unstake::Head::<T>::take().map(IntoPortable::into_portable),
			eras_to_check_per_block: pallet_fast_unstake::ErasToCheckPerBlock::<T>::exists()
				.then(pallet_fast_unstake::ErasToCheckPerBlock::<T>::take),
		}
	}
}

impl<T: pallet_fast_unstake::Config> FastUnstakeMigrator<T>
where
	<<T as pallet_fast_unstake::Config>::Currency as frame_support::traits::Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance: From<u128>,
	<T as frame_system::Config>::AccountId: From<AccountId32>,
{
	pub fn put_values(values: PortableFastUnstakeStorageValues) {
		values
			.head
			.map(Into::<pallet_fast_unstake::types::UnstakeRequest<_>>::into)
			.map(pallet_fast_unstake::Head::<T>::put);
		values
			.eras_to_check_per_block
			.map(pallet_fast_unstake::ErasToCheckPerBlock::<T>::put);
	}
}

impl PortableFastUnstakeStorageValues {
	pub fn translate_accounts(self, function: impl Fn(AccountId32) -> AccountId32) -> Self {
		let head = self.head.map(|mut request| {
			request.stashes.iter_mut().for_each(|(account, _)| {
				*account = function(account.clone());
			});
			request
		});

		Self { head, eras_to_check_per_block: self.eras_to_check_per_block }
	}
}

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	MaxEncodedLen,
	TypeInfo,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	RuntimeDebugNoBound,
)]
pub struct PortableUnstakeRequest {
	pub stashes: BoundedVec<(AccountId32, u128), ConstU32<20>>, // Fast Unstake batch size = 16
	pub checked: BoundedVec<EraIndex, ConstU32<30>>,            // Bonding duration + 1 = 29
}

// RC -> Portable
impl<T: Config> IntoPortable for pallet_fast_unstake::types::UnstakeRequest<T> {
	type Portable = PortableUnstakeRequest;

	fn into_portable(self) -> Self::Portable {
		PortableUnstakeRequest {
			stashes: self.stashes.into_inner().defensive_truncate_into(),
			checked: self.checked.into_inner().defensive_truncate_into(),
		}
	}
}

// Portable -> AH
impl<T: pallet_fast_unstake::Config> From<PortableUnstakeRequest>
	for pallet_fast_unstake::types::UnstakeRequest<T>
where
	<<T as pallet_fast_unstake::Config>::Currency as frame_support::traits::Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance: From<u128>,
	<T as frame_system::Config>::AccountId: From<AccountId32>,
{
	fn from(request: PortableUnstakeRequest) -> Self {
		pallet_fast_unstake::types::UnstakeRequest {
			stashes: request
				.stashes
				.into_iter()
				.map(|(a, v)| (a.into(), v.into()))
				.collect::<Vec<_>>()
				.defensive_truncate_into(),
			checked: request.checked.into_inner().defensive_truncate_into(),
		}
	}
}

impl PortableFastUnstakeStorageValues {
	pub fn is_empty(&self) -> bool {
		self.head.is_none() && self.eras_to_check_per_block.is_none()
	}
}

pub struct FastUnstakeMigrator<T> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for FastUnstakeMigrator<T> {
	type Key = FastUnstakeStage<T::AccountId>;
	type Error = Error<T>;

	fn migrate_many(
		current_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut inner_key = current_key.unwrap_or(FastUnstakeStage::StorageValues);
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
				.any_lt(T::AhWeightInfo::receive_fast_unstake_messages((messages.len() + 1) as u32))
			{
				log::info!("AH weight limit reached at batch length {}, stopping", messages.len());
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
				FastUnstakeStage::StorageValues => {
					let values = Self::take_values();
					if !values.is_empty() {
						messages.push(PortableFastUnstakeMessage::StorageValues { values });
					} else {
						log::info!(
							target: LOG_TARGET,
							"Fast unstake storage values are empty. Skipping fast unstake values \
								migration.",
						);
					}
					FastUnstakeStage::Queue(None)
				},
				FastUnstakeStage::Queue(queue_iter) => {
					let mut new_queue_iter = match queue_iter.clone() {
						Some(queue_iter) => pallet_fast_unstake::Queue::<T>::iter_from(
							pallet_fast_unstake::Queue::<T>::hashed_key_for(queue_iter),
						),
						None => pallet_fast_unstake::Queue::<T>::iter(),
					};

					match new_queue_iter.next() {
						Some((key, member)) => {
							pallet_fast_unstake::Queue::<T>::remove(&key);
							messages.push(PortableFastUnstakeMessage::Queue {
								member: (key.clone(), member),
							});
							FastUnstakeStage::Queue(Some(key))
						},
						None => FastUnstakeStage::Finished,
					}
				},
				FastUnstakeStage::Finished => {
					break;
				},
			}
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(messages, |messages| {
				types::AhMigratorCall::<T>::ReceiveFastUnstakeMessages { messages }
			})?;
		}

		if inner_key == FastUnstakeStage::Finished {
			Ok(None)
		} else {
			Ok(Some(inner_key))
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for FastUnstakeMigrator<T> {
	type RcPrePayload = (Vec<(T::AccountId, u128)>, u32); // (queue, eras_to_check)

	fn pre_check() -> Self::RcPrePayload {
		let queue: Vec<_> = pallet_fast_unstake::Queue::<T>::iter().collect();
		let eras_to_check = pallet_fast_unstake::ErasToCheckPerBlock::<T>::get();

		assert!(
			pallet_fast_unstake::Head::<T>::get().is_none(),
			"Staking Heads must be empty on the relay chain before the migration"
		);

		(queue, eras_to_check)
	}

	fn post_check(_: Self::RcPrePayload) {
		// RC post: Ensure that entries have been deleted
		assert!(
			pallet_fast_unstake::Head::<T>::get().is_none(),
			"Assert storage 'FastUnstake::Head::rc_post::empty'"
		);
		assert!(
			pallet_fast_unstake::Queue::<T>::iter().next().is_none(),
			"Assert storage 'FastUnstake::Queue::rc_post::empty'"
		);
		assert!(
			pallet_fast_unstake::ErasToCheckPerBlock::<T>::get() == 0,
			"Assert storage 'FastUnstake::ErasToCheckPerBlock::rc_post::empty'"
		);
	}
}
