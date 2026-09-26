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

//! HRMP stage: sends the Coretime chain a record of every HRMP deposit, so it can hold each one
//! against its channel and side once the migration is done.
//!
//! The records themselves stay here, deposits included. The relay chain routes every HRMP message
//! through `HrmpChannels` and completes an open handshake at a session boundary from
//! `HrmpOpenChannelRequests`, and after the migration it still decides every deposit: it asks the
//! Coretime chain to hold or release the amounts these records name.
//!
//! The deposits themselves travel via the accounts stage: they arrive as holds on the paras'
//! *sibling sovereign* accounts, and the receiving side re-attributes them to the HRMP pallet as
//! each record lands.

use crate::*;
use polkadot_parachain_primitives::primitives::IsSystem;
use runtime_parachains::{
	configuration,
	hrmp::{
		HrmpChannels, HrmpCloseChannelRequestsList, HrmpOpenChannelRequests,
		HrmpOpenChannelRequestsList,
	},
};

pub struct HrmpMigrator<T>(PhantomData<T>);

impl<T: Config> HrmpMigrator<T> {
	/// Close every channel with a close queued for the next session boundary, now.
	///
	/// No new close can be asked for once the migration has started, but one asked for before it
	/// would otherwise be enacted mid-migration, refunding against reserves the accounts stage has
	/// moved. Processed here, just before the accounts stage reads the reserves, the refunds land
	/// on this chain like any other.
	pub fn process_queued_closes() -> Result<(), Error<T>> {
		let count = HrmpCloseChannelRequestsList::<T>::decode_len().unwrap_or(0) as u32;
		if count == 0 {
			return Ok(());
		}
		runtime_parachains::hrmp::Pallet::<T>::force_process_hrmp_close(
			frame_system::RawOrigin::Root.into(),
			count,
		)
		.map_err(|_| Error::<T>::HrmpClosesFailed)?;
		Pallet::<T>::deposit_event(Event::HrmpClosesProcessed { count });
		Ok(())
	}

	/// What the recipient of an open-channel request reserved on accepting it.
	///
	/// `hrmp` does not record it on the request: `accept_open_channel` reserves the configured
	/// recipient deposit, unless either end is a system chain.
	pub fn request_recipient_deposit(
		id: &HrmpChannelId,
		request: &runtime_parachains::hrmp::HrmpOpenChannelRequest,
	) -> u128 {
		let system = id.sender.is_system() || id.recipient.is_system();
		if request.confirmed && !system {
			configuration::ActiveConfig::<T>::get().hrmp_recipient_deposit
		} else {
			0
		}
	}

	/// Send every pending open-channel request to the Coretime chain.
	///
	/// One-shot, called by `HrmpInit`; the request count is small (dozens). The caller wraps
	/// this in a storage transaction so a failed send rolls everything back for a retry.
	pub fn copy_open_requests() -> Result<(), Error<T>> {
		let mut batch = Vec::new();
		for id in HrmpOpenChannelRequestsList::<T>::get() {
			let Some(request) = HrmpOpenChannelRequests::<T>::get(&id) else { continue };
			let recipient_deposit = Self::request_recipient_deposit(&id, &request);
			batch.push(migrator_types::PortableHrmpRequest {
				sender: id.sender.into(),
				recipient: id.recipient.into(),
				confirmed: request.confirmed,
				sender_deposit: request.sender_deposit,
				max_message_size: request.max_message_size,
				max_capacity: request.max_capacity,
				max_total_size: request.max_total_size,
				recipient_deposit,
			});
		}

		let count = batch.len() as u32;
		while !batch.is_empty() {
			let rest = batch.split_off(batch.len().min(MAX_RECORDS_PER_XCM as usize));
			Pallet::<T>::send_hrmp_requests(core::mem::replace(&mut batch, rest))?;
		}
		Pallet::<T>::deposit_event(Event::HrmpRequestsSent { count });
		Ok(())
	}

	/// Send HRMP channel records until the per-block limit is reached.
	///
	/// Returns the cursor to continue from on the next block, or `None` once the map is
	/// exhausted. The caller wraps this in a storage transaction; an `Err` rolls back the whole
	/// block's writes.
	pub fn migrate_many(
		last_key: Option<HrmpChannelId>,
	) -> Result<Option<HrmpChannelId>, Error<T>> {
		// Get iterator starting after last processed key.
		let iter = match &last_key {
			Some(last_key) => HrmpChannels::<T>::iter_from_key(last_key),
			None => HrmpChannels::<T>::iter(),
		};

		Pallet::<T>::drain_records(
			iter,
			|channel_id, channel| {
				Ok(Some(PortableHrmpChannel {
					sender: channel_id.sender.into(),
					recipient: channel_id.recipient.into(),
					max_capacity: channel.max_capacity,
					max_total_size: channel.max_total_size,
					max_message_size: channel.max_message_size,
					sender_deposit: channel.sender_deposit,
					recipient_deposit: channel.recipient_deposit,
				}))
			},
			Pallet::<T>::send_hrmp,
		)
	}
}
