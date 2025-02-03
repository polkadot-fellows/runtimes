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
		if pallet_vesting::Vesting::<T>::contains_key(&message.who) {
			defensive!("Vesting schedule for {:?} already exists", message.who);
			return Err(Error::<T>::InsertConflict);
		}

		pallet_vesting::Vesting::<T>::insert(&message.who, message.schedules);
		log::debug!(target: LOG_TARGET, "Integrated vesting schedule for {:?}", message.who);

		Ok(())
	}
}
