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

use crate::{types::*, *};

impl<T: Config> Pallet<T> {
	pub fn do_receive_nom_pools_messages(
		messages: Vec<RcNomPoolsMessage<T>>,
	) -> Result<(), Error<T>> {
		for message in messages {
			Self::do_receive_nom_pools_message(message)?;
		}
		Ok(())
	}

	pub fn do_receive_nom_pools_message(
		message: RcNomPoolsMessage<T>,
	) -> Result<(), Error<T>> {
		match message {
			RcNomPoolsMessage::StorageValues { values } => {
				pallet_rc_migrator::staking::nom_pools::NomPoolsMigrator::<T>::put_values(values);
				//Self::deposit_event(Event::NomPoolsStoragesProcessed);
				log::info!("Received NomPoolsStorageValues");
				Ok(())
			},
			RcNomPoolsMessage::PoolMembers { member } => {
				debug_assert!(!pallet_nomination_pools::PoolMembers::<T>::contains_key(&member.0));
				pallet_nomination_pools::PoolMembers::<T>::insert(member.0, member.1);
				log::info!("Received NomPoolsPoolMembers");
				Ok(())
			},
			_ => {
				defensive!("Unknown message type");
				Ok(())
			}
		}
	}
}
