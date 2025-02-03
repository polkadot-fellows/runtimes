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

	pub fn do_process_vesting_schedule(message: RcVestingSchedule<T>) -> Result<(), Error<T>> {
		let mut schedules = pallet_vesting::Vesting::<T>::get(&message.who).unwrap_or_default();

		if !schedules.is_empty() {
			log::warn!(target: LOG_TARGET, "Merging with existing vesting schedule for {:?}", message.who);
		}

		let all_schedules =
			schedules.into_iter().chain(message.schedules.into_iter()).collect::<Vec<_>>();
		let bounded_schedule = BoundedVec::<_, _>::truncate_from(all_schedules);
		let truncated = all_schedules.len() - bounded_schedule.len();

		if truncated == 0 {
			pallet_vesting::Vesting::<T>::insert(&message.who, bounded_schedule);
			log::debug!(target: LOG_TARGET, "Integrated vesting schedule for {:?}", message.who);
		} else {
			log::error!(target: LOG_TARGET, "Truncated {} vesting schedules for {:?}", truncated, message.who);

			// TODO what do? should we create a storage item and insert the truncated ones?
			// Nobody seems to use the Vesting pallet on AH yet, but we cannot be sure that there
			// won't be a truncatenation.
		}

		Ok(())
	}
}
