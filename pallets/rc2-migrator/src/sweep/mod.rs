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

//! The Sweep stage: empty the chain's pots.
//!
//! A pot ([`Config::SweptAccounts`]) belongs to a pallet, not to anyone who could claim it on
//! Asset Hub, so the accounts stage leaves it alone. Sweep burns each one whole here, and the
//! stage machine teleports the total to [`Config::SweepBeneficiary`] on Asset Hub.
//!
//! Runs in a single block: the pot list is a handful of accounts.

#[cfg(test)]
mod tests;

use crate::{accounts::AccountsMigrator, Config, Error, Event, Pallet, LOG_TARGET};
use core::marker::PhantomData;
use frame_support::{
	defensive_assert, ensure,
	storage::with_storage_layer,
	traits::{
		fungible::{Mutate, Unbalanced},
		tokens::{Fortitude, Precision, Preservation},
		Get, ReservableCurrency,
	},
};
use sp_runtime::{traits::Zero, DispatchError};

type NativeCurrency<T> = pallet_balances::Pallet<T>;

pub struct SweepMigrator<T>(PhantomData<T>);

impl<T: Config> SweepMigrator<T> {
	/// Burn every pot that holds something. Returns the total burned, for the caller to book and
	/// teleport.
	///
	/// Each pot is burned in a storage transaction of its own, so one that fails is left as it
	/// was, reported with [`Event::PotNotSwept`], and the others go on.
	pub fn burn_pots() -> u128 {
		let mut total = 0u128;
		for pot in T::SweptAccounts::get() {
			match with_storage_layer(|| Self::burn_pot(&pot)) {
				Ok(amount) if amount.is_zero() => (),
				Ok(amount) => {
					total = total.saturating_add(amount);
					Pallet::<T>::deposit_event(Event::PotSwept { who: pot, amount });
				},
				Err(error) => {
					log::error!(target: LOG_TARGET, "Pot {pot:?} not swept: {error:?}");
					Pallet::<T>::deposit_event(Event::PotNotSwept { who: pot, error });
				},
			}
		}
		total
	}

	/// Burn everything `pot` holds and return the amount. Holds, reserves, locks and freezes are
	/// released first, so whatever form the balance is in, all of it is burned.
	///
	/// The account is reaped, unless something still references it once everything is released.
	/// Then it stays as a zero-balance record, like a validator's account in the accounts stage.
	fn burn_pot(pot: &T::AccountId) -> Result<u128, DispatchError> {
		// TODO(ahm-v2): decide how a pot's held and frozen balance (reserves, holds, locks and
		// freezes) should be handled. Picked for now: release all of it and teleport it to the
		// beneficiary with the free balance. The accounts stage instead parks reserve that no
		// deposit record explains on the Coretime chain, for governance to look at.
		// release everything, so the whole balance is free
		AccountsMigrator::<T>::release_holds(pot)?;
		let reserved = frame_system::Account::<T>::get(pot).data.reserved;
		let not_released = NativeCurrency::<T>::unreserve(pot, reserved);
		ensure!(not_released.is_zero(), Error::<T>::FailedToWithdrawAccount);
		AccountsMigrator::<T>::release_locks(pot)?;

		// burn; a release may have dropped a consumer ref, so read the account again
		let account = frame_system::Account::<T>::get(pot);
		defensive_assert!(
			account.data.reserved.is_zero() && account.data.frozen.is_zero(),
			"everything was released"
		);
		let total = account.data.free;
		if total.is_zero() {
			// nothing to burn
		} else if account.consumers != 0 {
			AccountsMigrator::<T>::drain_to_shell(pot, total);
		} else {
			NativeCurrency::<T>::burn_from(
				pot,
				total,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite,
			)?;
		}

		// the pot is empty now
		if *pot == pallet_treasury::Pallet::<T>::account_id() {
			Self::reactivate_treasury();
		}
		Ok(total)
	}

	/// Undo the treasury's inactive-issuance bookkeeping, now that its pot is burned.
	///
	/// The treasury marks its pot as inactive issuance, so that it does not count towards
	/// governance turnout, and records the amount it marked in `Deactivated`. Its hook keeps the
	/// two in step every block by reactivating the recorded amount and deactivating the pot again.
	/// Reactivating the recorded amount here and clearing the record leaves that hook nothing to
	/// undo: it finds an empty pot and nothing recorded. Reactivating the burned amount instead
	/// would be undone twice, once here and once by the hook.
	fn reactivate_treasury() {
		let deactivated = pallet_treasury::Deactivated::<T>::take();
		<NativeCurrency<T> as Unbalanced<T::AccountId>>::reactivate(deactivated);
	}
}
