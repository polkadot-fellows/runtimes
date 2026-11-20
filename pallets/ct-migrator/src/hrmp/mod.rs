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

//! HRMP stage, receiving side: HRMP deposit in the relay chain are received.

#[cfg(test)]
mod tests;

use crate::{Config, Event, HoldReason, Pallet, LOG_TARGET};
use alloc::vec::Vec;
use hrmp_primitives::ReceiveMigratedDeposits;
use migrator_types::{DepositKey, DepositSide, PortableHrmpChannelRecord};
use sp_runtime::DispatchResult;

impl<T: Config> Pallet<T> {
	/// Hold a batch of records, each in a transaction of its own. A failed record is logged.
	pub fn do_receive_hrmp_deposits(deposits: Vec<PortableHrmpChannelRecord>) {
		let (count_good, count_bad) = Self::receive_batch(
			deposits,
			Self::do_receive_hrmp_deposit, // receive_batch rolls back the state on error
			|()| (),
			|record, e| {
				log::error!(
					target: LOG_TARGET,
					"HRMP deposits of {:?} not held: {e:?}",
					record.channel,
				);
			},
		);
		Self::deposit_event(Event::HrmpDepositsReceived { count_good, count_bad });
	}

	/// Take over each side's deposit of the record, skipping a side that pays nothing.
	///
	/// Not transactional: on error, storage may be left dirty and the caller must roll it back.
	fn do_receive_hrmp_deposit(record: &PortableHrmpChannelRecord) -> DispatchResult {
		if record.sender_deposit != 0 {
			let key = DepositKey { channel: record.channel, side: DepositSide::Sender };
			Self::do_receive_hrmp_side(key, record.sender_deposit)?;
		}
		if record.recipient_deposit != 0 {
			let key = DepositKey { channel: record.channel, side: DepositSide::Recipient };
			Self::do_receive_hrmp_side(key, record.recipient_deposit)?;
		}
		Ok(())
	}

	/// The accounts stage brought the balance, under the migration hold `HrmpDeposit` of the
	/// para's sibling account. The local HRMP deposit pallet moves it from there to its own hold
	/// so that the side holds `amount`.
	///
	/// Idempotent: the side is set rather than added to, so a deposit delivered twice is held
	/// once, and what a side holds in excess goes back under the migration hold.
	fn do_receive_hrmp_side(key: DepositKey, amount: u128) -> DispatchResult {
		let source: T::RuntimeHoldReason = HoldReason::HrmpDeposit.into();
		T::HrmpDeposits::receive_deposit(key, amount, &source)
	}
}
