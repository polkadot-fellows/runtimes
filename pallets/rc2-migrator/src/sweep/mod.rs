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

//! The Sweep and Sweep Dust stages send the balances the accounts stage leaves to Asset Hub.
//!
//! The accounts stage leaves the pots in [`Config::SweptAccounts`] for Sweep. Sweep burns each
//! pot's balance on the Relay Chain and teleports the total to [`Config::SweepBeneficiary`] on
//! Asset Hub. Burning a pot cannot fail, so every pot ends empty.
//!
//! Sweep handles the whole list in one block. If sending the teleport fails, the entire stage
//! is rolled back and retried in the next block.
//!
//! Sweep Dust then goes through the accounts left, a block's worth at a time, and empties those
//! below the existential deposit, sending the balance to its owner on Asset Hub or to the sweep
//! beneficiary. See [`SweepMigrator::sweep_dust`].

#[cfg(test)]
mod tests;

use crate::{accounts::AccountsMigrator, Config, Event, Limits, Pallet};
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::traits::{fungible::Inspect, Get};
use sp_runtime::{traits::Zero, AccountId32};

type NativeCurrency<T> = pallet_balances::Pallet<T>;

/// The dust one block swept.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BlockDust {
	/// Balance to teleport to Asset Hub, per account.
	pub to_ah: Vec<(AccountId32, u128)>,
	/// Where the next block continues from; `None` once every account has been visited.
	pub last_key: Option<AccountId32>,
}

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
	fn burn_pot(pot: &T::AccountId) -> u128 {
		if !frame_system::Account::<T>::contains_key(pot) {
			return 0;
		}
		// all of it goes to the beneficiary, deposits included; nothing goes to the Coretime chain
		let total = Self::burn_account(pot);
		Pallet::<T>::deposit_event(Event::PotSwept { who: pot.clone(), amount: total });
		total
	}

	/// Burn everything `who` holds, drop the balances pallet's references to it, and return the
	/// amount.
	///
	/// Writes the account's storage directly: the balances pallet can refuse to release a hold,
	/// or destroy a balance below the existential deposit as dust, and this must not fail.
	fn burn_account(who: &T::AccountId) -> u128 {
		// so the reference counts below follow the balances pallet's current rules
		NativeCurrency::<T>::ensure_upgraded(who);
		let balance = frame_system::Account::<T>::get(who).data;
		let total = balance.free.saturating_add(balance.reserved);
		// the balances pallet's references: a provider while free is at least the existential
		// deposit, a consumer while anything is reserved or frozen
		let balances_provides =
			balance.free >= <NativeCurrency<T> as Inspect<T::AccountId>>::minimum_balance();
		let balances_consumes = !balance.reserved.is_zero() || !balance.frozen.is_zero();

		// empty the account and its balance records, and burn what it held
		frame_system::Account::<T>::mutate(who, |account| {
			account.data.free = Zero::zero();
			account.data.reserved = Zero::zero();
			account.data.frozen = Zero::zero();
		});
		pallet_balances::Holds::<T>::remove(who);
		pallet_balances::Reserves::<T>::remove(who);
		pallet_balances::Locks::<T>::remove(who);
		pallet_balances::Freezes::<T>::remove(who);
		pallet_balances::TotalIssuance::<T>::mutate(|issuance| {
			*issuance = issuance.saturating_sub(total)
		});

		// drop the balances pallet's references; dropping the last provider removes the account
		if balances_consumes {
			frame_system::Pallet::<T>::dec_consumers(who);
		}
		if balances_provides {
			// fails only if another pallet still references the account, which then stays
			let _ = frame_system::Pallet::<T>::dec_providers(who);
		}

		total
	}

	/// Sweep the dust of the accounts after `last_key`, as many as [`items_per_block`] allows.
	///
	/// An account below the existential deposit, other than the `manager`, is emptied. Its balance
	/// goes to its owner on Asset Hub if it is at least Asset Hub's existential deposit, and to
	/// the [`SweepBeneficiary`] otherwise. A pot's always goes to the sweep beneficiary, like the
	/// rest of the pots, and so does a pure proxy's, since nobody controls a pure proxy on Asset
	/// Hub. The account is then removed, unless it is a pot or something else still needs it.
	///
	/// [`items_per_block`]: crate::BlockLimits::items_per_block
	/// [`SweepBeneficiary`]: Config::SweepBeneficiary
	pub fn sweep_dust(last_key: Option<AccountId32>, manager: Option<&AccountId32>) -> BlockDust {
		let rc_ed = <NativeCurrency<T> as Inspect<T::AccountId>>::minimum_balance();
		let ah_ed = T::AhExistentialDeposit::get();
		let pots = T::SweptAccounts::get();
		let beneficiary = T::SweepBeneficiary::get();

		let mut iter = last_key
			.map(frame_system::Account::<T>::iter_from_key)
			.unwrap_or_else(frame_system::Account::<T>::iter);

		let mut out = BlockDust { to_ah: Vec::new(), last_key: None };
		let mut to_beneficiary = 0u128;
		for _ in 0..Limits::<T>::get().items_per_block() {
			let Some((who, info)) = iter.next() else {
				out.last_key = None;
				break;
			};
			// Always skip the account if it's the manager; it's not handled in this step.
			if manager == Some(&who) {
				out.last_key = Some(who);
				continue;
			}
			// An account at or above the existential deposit can still be here, if the accounts
			// stage failed to withdraw it or fees refilled it. It is not dust.
			if info.data.total() < rc_ed {
				let amount = Self::burn_account(&who);
				let pot = pots.contains(&who);
				if !amount.is_zero() {
					let owned = !pot && !AccountsMigrator::<T>::is_pure_proxy(&who, &info);
					let to = if owned && amount >= ah_ed {
						AccountsMigrator::<T>::ah_destination(&who)
					} else {
						beneficiary.clone()
					};
					// the beneficiary's dust is sent in one deposit at the end of the block
					if to == beneficiary {
						to_beneficiary = to_beneficiary.saturating_add(amount);
					} else {
						out.to_ah.push((to.clone(), amount));
					}
					Pallet::<T>::deposit_event(Event::DustSwept { who: who.clone(), amount, to });
				}
				if !pot {
					Self::remove_if_unreferenced(&who);
				}
			}
			out.last_key = Some(who);
		}
		if !to_beneficiary.is_zero() {
			out.to_ah.push((beneficiary, to_beneficiary));
		}
		out
	}

	/// Remove `who` if at most one provider reference keeps it. An account with more, or with a
	/// consumer or sufficient reference, stays.
	fn remove_if_unreferenced(who: &T::AccountId) {
		let account = frame_system::Account::<T>::get(who);
		match (account.providers, account.consumers, account.sufficients) {
			(1, 0, 0) => {
				// cannot fail with these references
				let _ = frame_system::Pallet::<T>::dec_providers(who);
			},
			// `dec_providers` would remove it too, but log an underflow
			(0, 0, 0) => frame_system::Account::<T>::remove(who),
			_ => {},
		}
	}
}
