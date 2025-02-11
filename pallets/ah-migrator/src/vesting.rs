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

use super::*;

impl<T: Config> Pallet<T> {
	pub fn do_receive_vesting_schedules(
		messages: Vec<RcVestingSchedule<T>>,
	) -> Result<(), Error<T>> {
		alias::StorageVersion::<T>::put(alias::Releases::V1);
		log::info!(target: LOG_TARGET, "Integrating {} vesting schedules", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Vesting,
			count: messages.len() as u32,
		});
		let (mut count_good, mut count_bad) = (0, 0);

		for message in messages {
			match Self::do_process_vesting_schedule(message) {
				Ok(()) => count_good += 1,
				Err(e) => {
					count_bad += 1;
					log::error!(target: LOG_TARGET, "Error while integrating vesting: {:?}", e);
				},
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Vesting,
			count_good,
			count_bad,
		});

		Ok(())
	}

	/// Integrate vesting schedules.
	///
	/// May merges them with pre-existing schedules if there is not enough space.
	pub fn do_process_vesting_schedule(message: RcVestingSchedule<T>) -> Result<(), Error<T>> {
		let ah_schedules = pallet_vesting::Vesting::<T>::get(&message.who).unwrap_or_default();

		if !ah_schedules.is_empty() {
			log::warn!(target: LOG_TARGET, "Merging with existing vesting schedule for {:?}", message.who);
		}

		let all_schedules = ah_schedules
			.into_iter()
			.chain(message.schedules.into_iter())
			.collect::<Vec<_>>();
		let (bounded, truncated) = all_schedules
			.split_at(T::MAX_VESTING_SCHEDULES.min(all_schedules.len() as u32) as usize);
		let bounded_schedules = BoundedVec::<_, _>::truncate_from(bounded.to_vec());

		if truncated.is_empty() {
			pallet_vesting::Vesting::<T>::insert(&message.who, bounded_schedules);
			log::debug!(target: LOG_TARGET, "Integrated vesting schedule for {:?}", message.who);
			return Ok(());
		}

		log::error!(target: LOG_TARGET, "Truncated {} vesting schedules for {:?}", truncated.len(), message.who);
		pallet_vesting::Vesting::<T>::insert(&message.who, &bounded_schedules);

		for truncate in truncated {
			let len = pallet_vesting::Vesting::<T>::get(&message.who).unwrap_or_default().len();
			let last_index = len.checked_sub(1).ok_or(Error::<T>::Unreachable)? as u32;
			let second_last_index = len.checked_sub(2).ok_or(Error::<T>::Unreachable)? as u32;
			Self::merge_schedules(message.who.clone(), second_last_index, last_index)?;

			// Now we have at least one slot free that we can use to insert into:
			defensive_assert!(
				pallet_vesting::Vesting::<T>::get(&message.who).unwrap_or_default().len() <
					bounded_schedules.len()
			);
			// Insert the new schedule into the free slot:
			let mut schedules = pallet_vesting::Vesting::<T>::get(&message.who).unwrap_or_default();
			schedules.try_push(*truncate).map_err(|_| Error::<T>::Unreachable)?;
			pallet_vesting::Vesting::<T>::insert(&message.who, &schedules);
		}

		Ok(())
	}

	/// Merges two vesting schedules.
	///
	/// This function makes use of an pallet-vesting call, which is not entirely clean, but our best
	/// bet since otherwise it requires big code duplication. However, nobody is currently using the
	/// Vesting pallet on AH, and I recon it unlikely that someone would create a lot of vesting
	/// schedules just to have them merged (which they can also do on their own).
	pub fn merge_schedules(
		who: T::AccountId,
		schedule1_index: u32,
		schedule2_index: u32,
	) -> Result<(), Error<T>> {
		// Pretend to be the account:
		let origin: <T as frame_system::Config>::RuntimeOrigin =
			frame_system::RawOrigin::Signed(who.clone()).into();

		// NOTE: the pallet macro should already add a transaction, but am not 100% sure:
		let res = frame_support::storage::transactional::with_storage_layer(|| {
			pallet_vesting::Pallet::<T>::merge_schedules(origin, schedule1_index, schedule2_index)
		});

		let Err(err) = res else {
			return Ok(());
		};

		defensive!("Failed to merge vesting schedules: {:?}", err);
		// This is an important error, so we emit an event to monitor if it happens in production:
		let err_index = err.using_encoded(|e| e.get(0).copied());
		Self::deposit_event(Event::FailedToMergeVestingSchedules {
			who,
			schedule1: schedule1_index,
			schedule2: schedule2_index,
			pallet_vesting_error_index: err_index,
		});
		Err(Error::<T>::FailedToMergeVestingSchedules)
	}
}

pub mod alias {
	use super::*;

	#[frame_support::storage_alias(pallet_name)]
	pub type StorageVersion<T: pallet_vesting::Config> =
		StorageValue<pallet_vesting::Pallet<T>, Releases, ValueQuery>;

	#[derive(
		Encode, Decode, Clone, Copy, PartialEq, Eq, RuntimeDebug, MaxEncodedLen, Default, TypeInfo,
	)]
	pub enum Releases {
		#[default]
		V0,
		V1,
	}
}
