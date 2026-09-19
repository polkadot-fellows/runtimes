// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Accounts stage, receiving side: integrates the account payloads the relay chain burned.
//!
//! Each account is minted whole and its non-liquid parts re-established as holds, one per
//! [`PortableHoldReason`], through the regular fungible APIs — so refcounts and events are
//! indistinguishable from locally created state. A bad account is rolled back and parked in
//! `FailedAccounts` without failing the batch: the migration cannot stop mid-run to deal with
//! it, and the parked entry is what makes it recoverable afterwards.
//!
//! The call that carries a batch over XCM is the stage machine's; [`AccountsReceiver::receive`]
//! is what it invokes.

extern crate alloc;

#[cfg(test)]
mod tests;

use crate::{Config, CtMintedTotal, Event, FailedAccounts, HoldReason, Pallet};
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::{
	defensive_assert,
	traits::{
		fungible::{Inspect, InspectHold, Mutate, Unbalanced, UnbalancedHold},
		tokens::{Fortitude, Precision, Preservation},
	},
};
use migrator_types::{with_rollback, PortableAccount};
use sp_runtime::{traits::Zero, DispatchError, Saturating};

const LOG_TARGET: &str = "runtime::ct-migrator";

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
pub type PortableAccountOf<T> =
	PortableAccount<<T as frame_system::Config>::AccountId, BalanceOf<T>>;

/// Why an account could not be integrated. The caller rolls the account back and parks it.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// Minting the balance or placing one of its holds failed.
	FailedToProcessAccount,
}

pub struct AccountsReceiver<T>(PhantomData<T>);

impl<T: Config> AccountsReceiver<T> {
	/// Integrate a batch of migrated accounts.
	///
	/// Every account is processed in a transaction of its own: one that fails is rolled back and
	/// parked in `FailedAccounts`, the rest of the batch continues. Successful mints accrue to
	/// `CtMintedTotal`.
	pub fn receive(accounts: Vec<PortableAccountOf<T>>) {
		let mut minted: BalanceOf<T> = Zero::zero();
		let (count_good, count_bad) = Self::receive_batch(
			accounts,
			Self::receive_account,
			|amount| minted = minted.saturating_add(amount),
			|account, e| {
				log::error!(
					target: LOG_TARGET,
					"Failed to integrate account {:?}: {e:?}; parking it",
					account.who,
				);
				FailedAccounts::<T>::insert(account.who.clone(), account);
			},
		);
		if !minted.is_zero() {
			CtMintedTotal::<T>::mutate(|t| *t = t.saturating_add(minted));
		}
		Pallet::<T>::deposit_event(Event::AccountsReceived { count_good, count_bad });
	}

	/// Run `integrate` over every item in its own storage transaction, counting successes and
	/// handing each failure to `park`. Shared by every receiving stage so all of them isolate
	/// and report failures identically.
	pub(crate) fn receive_batch<I, R, E>(
		items: Vec<I>,
		integrate: impl Fn(&I) -> Result<R, E>,
		mut on_good: impl FnMut(R),
		park: impl Fn(I, E),
	) -> (u32, u32) {
		let (mut count_good, mut count_bad) = (0, 0);
		for item in items {
			match with_rollback(|| integrate(&item)) {
				Ok(r) => {
					count_good += 1;
					on_good(r);
				},
				Err(e) => {
					count_bad += 1;
					park(item, e);
				},
			}
		}
		(count_good, count_bad)
	}

	/// Mint one account and place its holds. Returns the amount minted.
	fn receive_account(account: &PortableAccountOf<T>) -> Result<BalanceOf<T>, Error> {
		let who = &account.who;
		let held: BalanceOf<T> = account
			.holds
			.iter()
			.fold(Zero::zero(), |acc: BalanceOf<T>, hold| acc.saturating_add(hold.amount));
		let total = account.free.saturating_add(held);

		// Accounts whose incoming free balance cannot provide the existential deposit get a
		// provider reference so the mint and hold below cannot fail or dust the account.
		if frame_system::Pallet::<T>::providers(who).is_zero() &&
			<T as Config>::Currency::balance(who).saturating_add(account.free) <
				<T as Config>::Currency::minimum_balance()
		{
			frame_system::Pallet::<T>::inc_providers(who);
		}

		let minted = <T as Config>::Currency::mint_into(who, total)
			.map_err(|_| Error::FailedToProcessAccount)?;
		defensive_assert!(minted == total, "minted what the relay chain burned");

		for hold in &account.holds {
			Self::place_hold(&hold.reason.into(), who, hold.amount)
				.map_err(|_| Error::FailedToProcessAccount)?;
		}

		Ok(minted)
	}

	/// Place `amount` of `who`'s free balance under `reason` without the account ever sitting
	/// at zero reserve mid-operation.
	///
	/// `MutateHold::hold` decreases the free balance before it books the hold; if that leaves a
	/// sub-ED free remainder while nothing is reserved yet, pallet-balances dusts the remainder.
	/// Deposit holders whose liquid dust deliberately travelled here alongside the deposit are in
	/// exactly that shape, so the two steps run in the reverse (safe) order. The low-level
	/// primitives keep total issuance untouched, like `hold` itself.
	pub(crate) fn place_hold(
		reason: &T::RuntimeHoldReason,
		who: &T::AccountId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		<T as Config>::Currency::increase_balance_on_hold(reason, who, amount, Precision::Exact)?;
		<T as Config>::Currency::decrease_balance(
			who,
			amount,
			Precision::Exact,
			Preservation::Expendable,
			Fortitude::Force,
		)?;
		Ok(())
	}

	/// The inverse of [`Self::place_hold`]: move `amount` from a hold back to free balance. For
	/// the stages that re-attribute a migrated reserve to the pallet owning the deposit.
	///
	/// Same hazard as there, same fix in reverse. `release` decreases the hold first, and if
	/// that takes it to zero while the free part is still sub-ED, pallet-balances dusts the
	/// remainder. Crediting the free part *first* means the hold never passes through zero while
	/// free is below ED. The two primitives are mint-and-burn of the same amount, so total
	/// issuance is untouched, just as `release` would be.
	pub fn release_hold(
		reason: &T::RuntimeHoldReason,
		who: &T::AccountId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		<T as Config>::Currency::increase_balance(who, amount, Precision::Exact)?;
		<T as Config>::Currency::decrease_balance_on_hold(reason, who, amount, Precision::Exact)?;
		Ok(())
	}

	/// Release `min(wanted, actually-held)` of `who`'s migrated `RcMigratedReserve` hold to free
	/// balance, returning `(released, shortfall)`.
	///
	/// The reconciliation rule of the whole receiving side: recorded deposits are honoured up to
	/// what actually arrived held, and the difference is the caller's to park under its own key.
	/// One implementation so every deposit kind reconciles identically.
	pub fn release_rc_reserve(
		who: &T::AccountId,
		wanted: BalanceOf<T>,
	) -> Result<(BalanceOf<T>, BalanceOf<T>), DispatchError> {
		let rc_reason: T::RuntimeHoldReason = HoldReason::RcMigratedReserve.into();
		let held = <T as Config>::Currency::balance_on_hold(&rc_reason, who);
		let release = wanted.min(held);
		if !release.is_zero() {
			Self::release_hold(&rc_reason, who, release)?;
		}
		Ok((release, wanted.saturating_sub(held)))
	}
}
