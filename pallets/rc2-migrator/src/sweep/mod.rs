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
//! Asset Hub. Burning a pot cannot fail, so every pot ends empty.
//!
//! Sweep handles the whole list in one block. If sending the teleport fails, the entire stage
//! is rolled back and retried in the next block.

#[cfg(test)]
mod tests;

use crate::{Config, Event, Pallet};
use core::marker::PhantomData;
use frame_support::traits::{fungible::Inspect, Get};
use sp_runtime::traits::Zero;

type NativeCurrency<T> = pallet_balances::Pallet<T>;

pub struct SweepMigrator<T>(PhantomData<T>);

impl<T: Config> SweepMigrator<T> {
	/// Burn every pot. Return the total burned for the caller to record and teleport.
	pub fn burn_pots() -> u128 {
		T::SweptAccounts::get()
			.iter()
			.map(Self::burn_pot)
			.fold(0u128, |total, burned| total.saturating_add(burned))
	}

	/// Burn everything `pot` holds and return the amount.
	///
	/// The pot is emptied by writing its storage directly, not through the balances pallet. The
	/// balances pallet refuses to release a hold if the free balance would stay below the
	/// existential deposit, and destroys a balance below it as dust without this pallet recording
	/// it. Writing the storage directly cannot fail and creates no dust.
	///
	/// The empty account is removed, unless another pallet still references it or provides for
	/// it, as the on-demand pallet does for its pot. It then stays as a record with no balance.
	fn burn_pot(pot: &T::AccountId) -> u128 {
		if !frame_system::Account::<T>::contains_key(pot) {
			return 0;
		}
		// All of a pot goes to the beneficiary: its reserves, holds, locks and freezes are burned
		// with its free balance. They do not go to the Coretime chain as holds, as an account's
		// deposits do in the accounts stage.

		// The reference counts below follow the balances pallet's current rules. This brings an
		// account it has not touched since those rules came in up to date. It cannot fail.
		NativeCurrency::<T>::ensure_upgraded(pot);
		let balance = frame_system::Account::<T>::get(pot).data;
		let total = balance.free.saturating_add(balance.reserved);
		// The balances pallet holds a provider reference while the free balance is at least the
		// existential deposit, and one consumer reference while anything is reserved or frozen.
		let balances_provides =
			balance.free >= <NativeCurrency<T> as Inspect<T::AccountId>>::minimum_balance();
		let balances_consumes = !balance.reserved.is_zero() || !balance.frozen.is_zero();

		// empty the pot and its balance records, and burn what it held
		frame_system::Account::<T>::mutate(pot, |account| {
			account.data.free = Zero::zero();
			account.data.reserved = Zero::zero();
			account.data.frozen = Zero::zero();
		});
		pallet_balances::Holds::<T>::remove(pot);
		pallet_balances::Reserves::<T>::remove(pot);
		pallet_balances::Locks::<T>::remove(pot);
		pallet_balances::Freezes::<T>::remove(pot);
		pallet_balances::TotalIssuance::<T>::mutate(|issuance| {
			*issuance = issuance.saturating_sub(total)
		});

		// drop the balances pallet's references; dropping the last provider removes the account
		if balances_consumes {
			frame_system::Pallet::<T>::dec_consumers(pot);
		}
		if balances_provides {
			// fails only if another pallet still references the account, which then stays
			let _ = frame_system::Pallet::<T>::dec_providers(pot);
		}

		Pallet::<T>::deposit_event(Event::PotSwept { who: pot.clone(), amount: total });
		total
	}
}
