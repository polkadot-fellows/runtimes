// Copyright (C) Polkadot Fellows.
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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

#![doc = include_str!("hrmp.md")]

#[cfg(test)]
mod tests;

use crate::{Config, Limits};
use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_support::traits::Get;
use migrator_types::{ChannelId, PortableHrmpChannelRecord};
use polkadot_parachain_primitives::primitives::{HrmpChannelId, IsSystem};
use runtime_parachains::{configuration, hrmp};
use scale_info::TypeInfo;

/// A record the HRMP stage visits, and so its cursor.
///
/// The open requests go first, then the channels.
#[derive(
	Encode, Decode, DecodeWithMemTracking, MaxEncodedLen, Clone, PartialEq, Eq, Debug, TypeInfo,
)]
pub enum HrmpKey {
	/// An open-channel request, in `HrmpOpenChannelRequests`.
	Request(HrmpChannelId),
	/// An open channel, in `HrmpChannels`.
	Channel(HrmpChannelId),
}

/// The channel records one block read.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BlockHrmpChannels {
	/// The records whose deposits the Coretime chain is to hold.
	pub channels: Vec<PortableHrmpChannelRecord>,
	/// The records read, a request, or a channel and its close request, for the block's weight.
	pub records: u32,
	/// Where the next block continues from; `None` once every record has been visited.
	pub last_key: Option<HrmpKey>,
}

pub struct HrmpMigrator<T>(PhantomData<T>);

impl<T: Config> HrmpMigrator<T> {
	/// Read the deposits of the records after `last_key`, the open requests then the channels,
	/// up to [`Config::HrmpRecordsPerBlock`] of them and the block's [`Limits`] in items.
	/// Writes nothing.
	pub fn migrate_many(last_key: Option<HrmpKey>) -> BlockHrmpChannels {
		let (requests, channels) = match last_key {
			None =>
				(Some(hrmp::HrmpOpenChannelRequests::<T>::iter()), hrmp::HrmpChannels::<T>::iter()),
			Some(HrmpKey::Request(key)) => (
				Some(hrmp::HrmpOpenChannelRequests::<T>::iter_from_key(key)),
				hrmp::HrmpChannels::<T>::iter(),
			),
			Some(HrmpKey::Channel(key)) => (None, hrmp::HrmpChannels::<T>::iter_from_key(key)),
		};

		let recipient_deposit = requests
			.as_ref()
			.map_or(0, |_| configuration::ActiveConfig::<T>::get().hrmp_recipient_deposit);

		let mut records = requests
			.into_iter()
			.flatten()
			.map(|(id, request)| {
				let recipient =
					if Self::recipient_reserved(&id, &request) { recipient_deposit } else { 0 };
				let record = Self::record(&id, request.sender_deposit, recipient);
				(HrmpKey::Request(id), record)
			})
			.chain(channels.map(|(id, channel)| {
				let record = if Self::is_closing(&id) {
					None
				} else {
					Self::record(&id, channel.sender_deposit, channel.recipient_deposit)
				};
				(HrmpKey::Channel(id), record)
			}));

		let items_limit = Limits::<T>::get().items_per_block() as usize;
		let mut out = BlockHrmpChannels::default();
		for _ in 0..T::HrmpRecordsPerBlock::get() {
			// a record adds at most one item
			if out.channels.len() >= items_limit {
				return out;
			}
			let Some((key, record)) = records.next() else {
				out.last_key = None;
				return out;
			};
			out.records += 1;
			out.channels.extend(record);
			out.last_key = Some(key);
		}
		out
	}

	/// Whether the recipient of request `id` has reserved its deposit: on confirmation, unless
	/// either end is a system chain. It is recorded nowhere until the channel opens.
	pub fn recipient_reserved(id: &HrmpChannelId, request: &hrmp::HrmpOpenChannelRequest) -> bool {
		let system = id.sender.is_system() || id.recipient.is_system();
		request.confirmed && !system
	}

	/// Whether `id` closes at the next session boundary, after the migration. Its deposits are
	/// refunded by the accounts stage.
	pub fn is_closing(id: &HrmpChannelId) -> bool {
		hrmp::HrmpCloseChannelRequests::<T>::contains_key(id)
	}

	/// The record of `id`'s deposits, unless neither side pays anything.
	fn record(
		id: &HrmpChannelId,
		sender_deposit: u128,
		recipient_deposit: u128,
	) -> Option<PortableHrmpChannelRecord> {
		if sender_deposit == 0 && recipient_deposit == 0 {
			return None;
		}
		Some(PortableHrmpChannelRecord {
			channel: ChannelId { sender: id.sender.into(), recipient: id.recipient.into() },
			sender_deposit,
			recipient_deposit,
		})
	}
}
