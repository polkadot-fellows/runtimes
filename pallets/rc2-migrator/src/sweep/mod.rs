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

//! The Sweep stage sends balances from listed Relay Chain pallet accounts to Asset Hub.
//!
//! The accounts stage leaves the pots in [`Config::SweptAccounts`] for Sweep. Sweep burns each
//! pot's balance on the Relay Chain and teleports the total to [`Config::SweepBeneficiary`] on
//! Asset Hub. If a pot cannot be swept, its changes are rolled back and [`Event::PotNotSwept`]
//! is emitted. The other pots are still processed.
//!
//! Sweep attempts the whole list in one block. If sending the teleport fails, the entire stage
//! is rolled back and retried in the next block.

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
	/// Attempt to burn the balance of each pot. Return the total burned for the caller to record
	/// and teleport.
	///
	/// Each pot is processed in its own storage transaction. A failure leaves the pot unchanged
	/// and emits [`Event::PotNotSwept`]; the other pots are still processed.
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

	/// Burn the pot's entire balance and return the amount. First release holds and reserves and
	/// remove locks and freezes.
	///
	/// The empty account is removed unless it still has a consumer reference, like a validator's
	/// account in the accounts stage, or a provider reference, like the on-demand pallet's pot.
	fn burn_pot(pot: &T::AccountId) -> Result<u128, DispatchError> {
		// TODO(ahm-v2): decide how to handle a pot's reserves, holds, locks and freezes. For now,
		// Sweep releases or removes them and sends the full balance to the beneficiary. The
		// accounts stage instead sends reserves without a deposit record to the Coretime chain as
		// holds for governance to review.
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

	/// Reactivate the amount recorded by the treasury after burning its pot's balance.
	///
	/// The treasury marks its pot, the free balance less the existential deposit, as inactive
	/// issuance and records the amount in `Deactivated`. Inactive issuance is excluded from the
	/// active issuance used to calculate referendum support. When the pot changes, the treasury
	/// hook reactivates the old amount and deactivates the new one. Clearing the record prevents
	/// the hook from reactivating it again. The recorded amount can differ from the burned balance,
	/// which may include the existential deposit.
	fn reactivate_treasury() {
		let deactivated = pallet_treasury::Deactivated::<T>::take();
		<NativeCurrency<T> as Unbalanced<T::AccountId>>::reactivate(deactivated);
	}
}
