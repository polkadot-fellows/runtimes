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
//! Asset Hub.
//!
//! Sweep handles the whole list in one block. If sending the teleport fails, the entire stage
//! is rolled back and retried in the next block.

#[cfg(test)]
mod tests;

use crate::{Config, Event, Pallet};
use core::marker::PhantomData;
use frame_support::traits::{
	fungible::{Inspect, Mutate},
	tokens::{Fortitude, Precision, Preservation},
	Defensive, Get,
};
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

	/// Burn `pot`'s free balance and return the amount.
	///
	/// Only the admin can put a hold, reserve, lock or freeze on a pot, or add a consumer
	/// reference to it. So a pot holds only free balance, and all of it can be burned.
	fn burn_pot(pot: &T::AccountId) -> u128 {
		if !frame_system::Account::<T>::contains_key(pot) {
			return 0;
		}
		let free = <NativeCurrency<T> as Inspect<T::AccountId>>::balance(pot);
		let burned = if free.is_zero() {
			0
		} else {
			<NativeCurrency<T> as Mutate<T::AccountId>>::burn_from(
				pot,
				free,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Force,
			)
			.defensive_proof("a pot without a consumer reference can burn all of its free balance")
			.unwrap_or_default()
		};

		Pallet::<T>::deposit_event(Event::PotSwept { who: pot.clone(), amount: burned });
		burned
	}
}
