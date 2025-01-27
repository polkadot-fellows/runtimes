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
		let (mut good, mut bad) = (0, 0);
		log::info!("Received {} NomPoolsMessages", messages.len());
		Self::deposit_event(Event::NomPoolsMessagesBatchReceived { count: messages.len() as u32 });

		for message in messages {
			if Self::do_receive_nom_pools_message(message).is_ok() {
				good += 1;
			} else {
				bad += 1;
			}
		}

		Self::deposit_event(Event::NomPoolsMessagesBatchProcessed {
			count_good: good as u32,
			count_bad: bad as u32,
		});
		Ok(())
	}

	pub fn do_receive_nom_pools_message(message: RcNomPoolsMessage<T>) -> Result<(), ()> {
		match message {
			RcNomPoolsMessage::StorageValues { values } => {
				pallet_rc_migrator::staking::nom_pools::NomPoolsMigrator::<T>::put_values(values);
				log::debug!("Received NomPoolsStorageValues");
				Ok(())
			},
			RcNomPoolsMessage::PoolMembers { member } => {
				debug_assert!(!pallet_nomination_pools::PoolMembers::<T>::contains_key(&member.0));
				log::debug!("Received NomPoolsPoolMember: {:?}", &member.0);
				pallet_nomination_pools::PoolMembers::<T>::insert(member.0, member.1);
				Ok(())
			},
			RcNomPoolsMessage::BondedPools { pool } => {
				debug_assert!(!pallet_nomination_pools::BondedPools::<T>::contains_key(&pool.0));
				log::debug!("Received NomPoolsBondedPool: {}", &pool.0);
				pallet_nomination_pools::BondedPools::<T>::insert(pool.0, pool.1);
				Ok(())
			},
			RcNomPoolsMessage::RewardPools { rewards } => {
				log::debug!("Received NomPoolsRewardPool: {:?}", &rewards.0);
				// Not sure if it is the best to use the alias here, but it is the easiest...
				pallet_rc_migrator::staking::nom_pools_alias::RewardPools::<T>::insert(
					rewards.0, rewards.1,
				);
				Ok(())
			},
			RcNomPoolsMessage::SubPoolsStorage { sub_pools } => {
				log::debug!("Received NomPoolsSubPoolsStorage: {:?}", &sub_pools.0);
				pallet_rc_migrator::staking::nom_pools_alias::SubPoolsStorage::<T>::insert(
					sub_pools.0,
					sub_pools.1,
				);
				Ok(())
			},
			RcNomPoolsMessage::Metadata { meta } => {
				log::debug!("Received NomPoolsMetadata: {:?}", &meta.0);
				pallet_nomination_pools::Metadata::<T>::insert(meta.0, meta.1);
				Ok(())
			},
			_ => {
				defensive!("Unknown message type");
				Err(())
			},
		}
	}
}
