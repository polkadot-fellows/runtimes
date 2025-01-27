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
use pallet_nomination_pools::BondedPoolInner;
use sp_runtime::Saturating;
use frame_support::traits::DefensiveSaturating;
use sp_runtime::traits::CheckedSub;
use sp_runtime::traits::One;

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
				pallet_nomination_pools::BondedPools::<T>::insert(pool.0, Self::rc_to_ah_bonded_pool(pool.1));
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
			RcNomPoolsMessage::ReversePoolIdLookup { lookups } => {
				log::debug!("Received NomPoolsReversePoolIdLookup: {:?}", &lookups.0);
				pallet_nomination_pools::ReversePoolIdLookup::<T>::insert(lookups.0, lookups.1);
				Ok(())
			},
			RcNomPoolsMessage::ClaimPermissions { perms } => {
				log::debug!("Received NomPoolsClaimPermissions: {:?}", &perms.0);
				pallet_nomination_pools::ClaimPermissions::<T>::insert(perms.0, perms.1);
				Ok(())
			},
		}
	}

	/// Translate a bonded RC pool to an AH one.
	pub fn rc_to_ah_bonded_pool(mut pool: BondedPoolInner<T>) -> BondedPoolInner<T> {
		log::info!("Translating BondedPool: {:?}", &pool);
		if let Some(ref mut throttle_from) = pool.commission.throttle_from {
			// Plus one here to be safe for the pool member just in case that the pool operator
			// would like to enact commission rate changes immediately.
			*throttle_from = Self::rc_to_ah_timepoint(*throttle_from).saturating_add(One::one());
		}
		if let Some(ref mut change_rate) = pool.commission.change_rate {
			// We cannot assume how this conversion works, but adding one will ensure that we err on
			// the side of pool-member safety in case of rounding.
			change_rate.min_delay = T::RcToProxyDelay::convert(change_rate.min_delay).saturating_add(One::one());
		}
		
		pool
	}

	/// Convert an absolute RC timepoint to an AH one.
	///
	///  This works by re-anchoring the timepoint to 
	pub fn rc_to_ah_timepoint(rc_timepoint: BlockNumberFor<T>) -> BlockNumberFor<T> {
		let rc_now = T::RcBlockNumberProvider::current_block_number();
		let ah_now = frame_system::Pallet::<T>::block_number();

		if let Some(rc_since) = rc_now.checked_sub(&rc_timepoint) {
			ah_now.saturating_sub(T::RcToProxyDelay::convert(rc_since)) // TODO rename
		} else {
			ah_now.saturating_add(T::RcToProxyDelay::convert(rc_timepoint.defensive_saturating_sub(rc_now)))
		}
	}
}
