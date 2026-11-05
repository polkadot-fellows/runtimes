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
use core::marker::PhantomData;
use migrator_types::{ChannelId, DepositKey, DepositSide, PortableHrmpDeposit};
use polkadot_parachain_primitives::primitives::{HrmpChannelId, IsSystem};
use runtime_parachains::{configuration, hrmp};
use sp_runtime::traits::Zero;

/// What one block of the stage read.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct BlockHrmpDeposits {
	/// The deposits to hold on the Coretime chain.
	pub(crate) deposits: Vec<PortableHrmpDeposit>,
	/// Where the next block continues from; `None` once every channel has been visited.
	pub(crate) last_key: Option<HrmpChannelId>,
}

pub(crate) struct HrmpMigrator<T>(PhantomData<T>);

impl<T: Config> HrmpMigrator<T> {
	/// The deposits of every open-channel request.
	///
	/// Runs once, in the `HrmpInit` block. Unbounded: the requests are bounded by the paras'
	/// open-request limits.
	pub(crate) fn request_deposits() -> Vec<PortableHrmpDeposit> {
		let recipient_deposit = Self::recipient_deposit();
		hrmp::HrmpOpenChannelRequests::<T>::iter()
			.flat_map(|(id, request)| Self::of_request(&id, &request, recipient_deposit))
			.collect()
	}

	/// Read the channels after `last_key` until the block's [`Limits`] of deposits is reached,
	/// and return the deposits. Writes nothing.
	pub(crate) fn migrate_many(last_key: Option<HrmpChannelId>) -> BlockHrmpDeposits {
		let mut iter = match last_key {
			Some(last_key) => hrmp::HrmpChannels::<T>::iter_from_key(last_key),
			None => hrmp::HrmpChannels::<T>::iter(),
		};

		let mut out = BlockHrmpDeposits::default();
		while out.deposits.len() < Limits::<T>::get().items_per_block() as usize {
			let Some((id, channel)) = iter.next() else {
				out.last_key = None;
				return out;
			};
			if !Self::is_closing(&id) {
				out.deposits.extend(Self::of_channel(&id, &channel));
			}
			out.last_key = Some(id);
		}
		out
	}

	/// Whether `id` closes at the next session boundary, after the migration. Its deposits are
	/// refunded by the accounts stage.
	pub(crate) fn is_closing(id: &HrmpChannelId) -> bool {
		hrmp::HrmpCloseChannelRequests::<T>::contains_key(id)
	}

	/// The deposits recorded for an open channel.
	pub(crate) fn of_channel(
		id: &HrmpChannelId,
		channel: &hrmp::HrmpChannel,
	) -> impl Iterator<Item = PortableHrmpDeposit> {
		Self::deposits(id, channel.sender_deposit, channel.recipient_deposit)
	}

	/// What a recipient reserves on accepting an open-channel request: the active configuration's
	/// value. Read once per block, the configuration is large.
	pub(crate) fn recipient_deposit() -> u128 {
		configuration::ActiveConfig::<T>::get().hrmp_recipient_deposit
	}

	/// The deposits taken for an open-channel request. The recipient's is reserved on acceptance
	/// at [`Self::recipient_deposit`], unless either end is a system chain, and is recorded
	/// nowhere until the channel opens.
	pub(crate) fn of_request(
		id: &HrmpChannelId,
		request: &hrmp::HrmpOpenChannelRequest,
		recipient_deposit: u128,
	) -> impl Iterator<Item = PortableHrmpDeposit> {
		let system = id.sender.is_system() || id.recipient.is_system();
		let recipient_deposit = if request.confirmed && !system { recipient_deposit } else { 0 };
		Self::deposits(id, request.sender_deposit, recipient_deposit)
	}

	/// One deposit per side that pays something.
	fn deposits(
		id: &HrmpChannelId,
		sender: u128,
		recipient: u128,
	) -> impl Iterator<Item = PortableHrmpDeposit> {
		let channel = ChannelId { sender: id.sender.into(), recipient: id.recipient.into() };
		[(DepositSide::Sender, sender), (DepositSide::Recipient, recipient)]
			.into_iter()
			.filter(|(_, amount)| !amount.is_zero())
			.map(move |(side, amount)| PortableHrmpDeposit {
				key: DepositKey { channel, side },
				amount,
			})
	}
}
