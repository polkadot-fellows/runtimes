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
use pallet_rc_migrator::vesting::{RcVestingSchedule, VestingMigrator};

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
	pub fn do_process_vesting_schedule(message: RcVestingSchedule<T>) -> Result<(), Error<T>> {
		let mut ah_schedules = pallet_vesting::Vesting::<T>::get(&message.who).unwrap_or_default();

		if !ah_schedules.is_empty() {
			defensive!("We disabled vesting, looks like someone used it. Manually verify this and then remove this defensive assert.");
		}

		for schedule in message.schedules {
			ah_schedules
				.try_push(schedule)
				.defensive()
				.map_err(|_| Error::<T>::FailedToIntegrateVestingSchedule)?;
		}

		pallet_vesting::Vesting::<T>::insert(&message.who, &ah_schedules);
		log::warn!(target: LOG_TARGET, "Integrated vesting schedule for {:?}, len {}", message.who, ah_schedules.len());

		Ok(())
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

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for VestingMigrator<T> {
	type RcPrePayload = Vec<RcVestingSchedule<T>>;
	type AhPrePayload = Vec<RcVestingSchedule<T>>;

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		pallet_vesting::Vesting::<T>::iter()
			.map(|(who, schedules)| RcVestingSchedule { who, schedules })
			.collect()
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, ah_pre_payload: Self::AhPrePayload) {
		use std::collections::BTreeMap;

		let all_pre: BTreeMap<_, _> = rc_pre_payload
			.into_iter()
			.chain(ah_pre_payload.into_iter())
			.map(|RcVestingSchedule { who, schedules }| (who.clone(), schedules))
			.collect();

		let all_post: BTreeMap<_, _> = pallet_vesting::Vesting::<T>::iter()
			.map(|(who, schedules)| (who, schedules))
			.collect();

		assert_eq!(all_pre, all_post, "RC and AH vesting schedules are present");
	}
}
