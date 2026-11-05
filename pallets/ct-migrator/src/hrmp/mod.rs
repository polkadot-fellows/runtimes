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

//! HRMP stage, receiving side: each deposit the relay chain records is held for its channel side
//! by `Config::HrmpDeposits`, out of the `HrmpDeposit` hold the accounts stage placed on the
//! paying para. A deposit that fails is rolled back, its hold in place, and logged; the rest of
//! the batch continues. The relay chain keeps the records, so what is held here is checked
//! against them.

#[cfg(test)]
mod tests;

use crate::{BalanceOf, Config, Event, HoldReason, Pallet, LOG_TARGET};
use alloc::vec::Vec;
use frame_support::traits::{
	fungible::{InspectHold, MutateHold},
	tokens::Precision,
};
use hrmp_primitives::ReceiveMigratedDeposits;
use migrator_types::{sibling_account, PortableHrmpDeposit};
use sp_runtime::{traits::Zero, DispatchResult, SaturatedConversion};

impl<T: Config> Pallet<T> {
	/// Hold a batch of deposits, each in a transaction of its own. A failed deposit is logged.
	pub fn do_receive_hrmp_deposits(deposits: Vec<PortableHrmpDeposit>) {
		let (count_good, count_bad) = Self::receive_batch(
			deposits,
			Self::do_receive_hrmp_deposit,
			|()| (),
			|deposit, e| {
				log::error!(
					target: LOG_TARGET,
					"HRMP deposit {:?} of {} not held: {e:?}",
					deposit.key,
					deposit.amount,
				);
			},
		);
		Self::deposit_event(Event::HrmpDepositsReceived { count_good, count_bad });
	}

	/// Release up to the deposit from the paying para's `HrmpDeposit` hold, and hand the deposit
	/// to `Config::HrmpDeposits`, which holds as much of it as the para can cover.
	fn do_receive_hrmp_deposit(deposit: &PortableHrmpDeposit) -> DispatchResult {
		let who = sibling_account::<T::AccountId>(deposit.key.para());
		let reason: T::RuntimeHoldReason = HoldReason::HrmpDeposit.into();
		let amount: BalanceOf<T> = deposit.amount.saturated_into();
		let migrated = T::Currency::balance_on_hold(&reason, &who).min(amount);
		if !migrated.is_zero() {
			T::Currency::release(&reason, &who, migrated, Precision::Exact)?;
		}
		T::HrmpDeposits::receive_deposit(deposit.key, deposit.amount)
	}
}
