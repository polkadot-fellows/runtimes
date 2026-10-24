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

//! Accounts stage, receiving side: each account the relay chain burned is minted here and its
//! holds placed through the fungible APIs. An account that fails is rolled back and parked in
//! `FailedAccounts`; the rest of the batch continues.

#[cfg(test)]
mod tests;

use crate::{
	BalanceOf, Config, CtMintedTotal, Event, FailedAccounts, HoldReason, Pallet, LOG_TARGET,
};
use alloc::vec::Vec;
use frame_support::{
	defensive, defensive_assert,
	traits::{
		fungible::{Inspect, Mutate, MutateHold},
		tokens::{Fortitude, Preservation},
	},
};
use migrator_types::PortableAccount;
use sp_runtime::{traits::Zero, DispatchError, Saturating};

pub type PortableAccountOf<T> =
	PortableAccount<<T as frame_system::Config>::AccountId, BalanceOf<T>>;

impl<T: Config> Pallet<T> {
	/// Mint a batch of accounts, each in a transaction of its own. What was minted accrues to
	/// `CtMintedTotal`; a failed account is parked in `FailedAccounts`.
	pub fn do_receive_accounts(accounts: Vec<PortableAccountOf<T>>) {
		let mut minted: BalanceOf<T> = Zero::zero();
		let (count_good, count_bad) = Self::receive_batch(
			accounts,
			Self::do_receive_account,
			|amount| minted = minted.saturating_add(amount),
			|account, e| {
				log::error!(target: LOG_TARGET, "Account {:?} not minted: {e:?}", account.who);
				Self::park(account);
			},
		);
		if !minted.is_zero() {
			CtMintedTotal::<T>::mutate(|t| *t = t.saturating_add(minted));
		}
		Self::deposit_event(Event::AccountsReceived { count_good, count_bad });
	}

	/// Mint one account and place its holds. Returns the amount minted. The existential deposit
	/// stays free; a hold takes what is left up to its amount.
	fn do_receive_account(account: &PortableAccountOf<T>) -> Result<BalanceOf<T>, DispatchError> {
		let who = &account.who;
		let held = account
			.holds
			.iter()
			.fold(BalanceOf::<T>::zero(), |sum, hold| sum.saturating_add(hold.amount));
		let total = account.free.saturating_add(held);

		let minted = T::Currency::mint_into(who, total)?;
		defensive_assert!(minted == total, "minted what the relay chain burned");

		for hold in &account.holds {
			let holdable =
				T::Currency::reducible_balance(who, Preservation::Preserve, Fortitude::Force);
			let amount = hold.amount.min(holdable);
			defensive_assert!(amount == hold.amount, "the hold is placed whole");
			if amount.is_zero() {
				continue;
			}
			let reason: T::RuntimeHoldReason = HoldReason::from(hold.reason).into();
			T::Currency::hold(&reason, who, amount)?;
		}

		Ok(minted)
	}

	/// Park a failed account. A child sovereign and a sibling-format account of the same para
	/// land on one address, so a second failure merges into the first.
	fn park(account: PortableAccountOf<T>) {
		let who = account.who.clone();
		let Some(mut parked) = FailedAccounts::<T>::get(&who) else {
			FailedAccounts::<T>::insert(&who, account);
			return;
		};
		defensive!("the same account parked twice", who);
		parked.free = parked.free.saturating_add(account.free);
		for hold in account.holds {
			match parked.holds.iter_mut().find(|h| h.reason == hold.reason) {
				Some(same) => same.amount = same.amount.saturating_add(hold.amount),
				// one entry per reason, so this fits
				None => {
					let _ = parked.holds.try_push(hold);
				},
			}
		}
		FailedAccounts::<T>::insert(&who, parked);
	}
}
