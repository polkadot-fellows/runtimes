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

//! Fast unstake migration logic.

use crate::*;
use pallet_rc_migrator::staking::fast_unstake::{
	FastUnstakeMigrator, PortableFastUnstakeMessage,
};

impl<T: Config> Pallet<T> {
	pub fn do_receive_fast_unstake_messages(
		messages: Vec<PortableFastUnstakeMessage>,
	) -> Result<(), Error<T>> {
		let (mut good, mut bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} FastUnstakeMessages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::FastUnstake,
			count: messages.len() as u32,
		});

		for message in messages {
			match Self::do_receive_fast_unstake_message(message) {
				Ok(_) => good += 1,
				Err(_) => bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::FastUnstake,
			count_good: good as u32,
			count_bad: bad as u32,
		});

		Ok(())
	}

	pub fn do_receive_fast_unstake_message(
		message: PortableFastUnstakeMessage,
	) -> Result<(), Error<T>> {
		match message {
			PortableFastUnstakeMessage::StorageValues { values } => {
				let values = values.translate_accounts(Self::translate_account_rc_to_ah);
				FastUnstakeMigrator::<T>::put_values(values);
				log::debug!(target: LOG_TARGET, "Integrating FastUnstakeStorageValues");
			},
			PortableFastUnstakeMessage::Queue { member } => {
				let translated_member = (Self::translate_account_rc_to_ah(member.0), member.1);
				debug_assert!(!pallet_fast_unstake::Queue::<T>::contains_key(&translated_member.0));
				log::debug!(target: LOG_TARGET, "Integrating FastUnstakeQueueMember: {:?}", &translated_member.0);
				pallet_fast_unstake::Queue::<T>::insert(translated_member.0, translated_member.1);
			},
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for FastUnstakeMigrator<T> {
	type RcPrePayload = (Vec<(AccountId32, u128)>, u32); // (queue, eras_to_check)
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// AH pre: Verify no entries are present
		assert!(
			pallet_fast_unstake::Head::<T>::get().is_none(),
			"Assert storage 'FastUnstake::Head::ah_pre::empty'"
		);
		assert!(
			pallet_fast_unstake::Queue::<T>::iter().next().is_none(),
			"Assert storage 'FastUnstake::Queue::ah_pre::empty'"
		);
		assert!(
			pallet_fast_unstake::ErasToCheckPerBlock::<T>::get() == 0,
			"Assert storage 'FastUnstake::ErasToCheckPerBlock::ah_pre::empty'"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		let (queue, eras_to_check) = rc_pre_payload;

		// AH post: Verify entries are correctly migrated
		let ah_queue: Vec<_> = pallet_fast_unstake::Queue::<T>::iter().collect();
		let ah_eras_to_check = pallet_fast_unstake::ErasToCheckPerBlock::<T>::get();

		// Assert storage "FastUnstake::Head::ah_post::correct"
		// Assert storage "FastUnstake::Head::ah_post::consistent"
		// Assert storage "FastUnstake::Head::ah_post::length"
		assert!(
			pallet_fast_unstake::Head::<T>::get().is_none(),
			"Assert storage 'FastUnstake::Head::ah_post::correct'"
		);

		// Assert storage "FastUnstake::Queue::ah_post::length"
		assert_eq!(
			queue.len(),
			ah_queue.len(),
			"Assert storage 'FastUnstake::Queue::ah_post::length'"
		);
		// Assert storage "FastUnstake::Queue::ah_post::correct"
		// Assert storage "FastUnstake::Queue::ah_post::consistent"
		for (pre_entry, post_entry) in queue.iter().zip(ah_queue.iter()) {
			let translated_pre_entry =
				(Pallet::<T>::translate_account_rc_to_ah(pre_entry.0.clone()), pre_entry.1.clone());
			assert_eq!(
				translated_pre_entry, *post_entry,
				"Assert storage 'FastUnstake::Queue::ah_post::correct'"
			);
		}

		// Assert storage "FastUnstake::ErasToCheckPerBlock::ah_post::correct"
		// Assert storage "FastUnstake::ErasToCheckPerBlock::ah_post::consistent"
		// Assert storage "FastUnstake::ErasToCheckPerBlock::ah_post::length"
		assert_eq!(
			eras_to_check, ah_eras_to_check,
			"Assert storage 'FastUnstake::ErasToCheckPerBlock::ah_post::correct'"
		);
	}
}
