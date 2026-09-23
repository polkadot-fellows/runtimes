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
		fungible::{Inspect, InspectHold, Mutate, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
	},
};
use migrator_types::PortableAccount;
use sp_runtime::{traits::Zero, DispatchError, Saturating};

const LOG_TARGET: &str = "runtime::ct-migrator";

pub use crate::BalanceOf;
pub type PortableAccountOf<T> =
	PortableAccount<<T as frame_system::Config>::AccountId, BalanceOf<T>>;

/// Why an account could not be integrated. The caller rolls the account back and parks it.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// Minting the balance or placing one of its holds failed.
	FailedToProcessAccount,
}

impl From<Error> for DispatchError {
	fn from(e: Error) -> Self {
		DispatchError::Other(match e {
			Error::FailedToProcessAccount => "FailedToProcessAccount",
		})
	}
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
		let (count_good, count_bad) = Pallet::<T>::receive_batch(
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

	/// Mint one account and place its holds. Returns the amount minted.
	///
	/// Holds are best effort: the existential deposit stays free, and whatever part of a hold
	/// the free balance cannot cover stays free with it. The owning pallet takes its deposit at
	/// this chain's rates out of the free balance later, so a short hold surfaces only as a
	/// shortfall on release.
	fn receive_account(account: &PortableAccountOf<T>) -> Result<BalanceOf<T>, DispatchError> {
		let who = &account.who;
		let held: BalanceOf<T> = account
			.holds
			.iter()
			.fold(Zero::zero(), |acc: BalanceOf<T>, hold| acc.saturating_add(hold.amount));
		let total = account.free.saturating_add(held);

		let minted = <T as Config>::Currency::mint_into(who, total)
			.map_err(|_| Error::FailedToProcessAccount)?;
		defensive_assert!(minted == total, "minted what the relay chain burned");

		for hold in &account.holds {
			let holdable = <T as Config>::Currency::reducible_balance(
				who,
				Preservation::Preserve,
				Fortitude::Force,
			);
			let amount = hold.amount.min(holdable);
			if amount.is_zero() {
				continue;
			}
			let reason: T::RuntimeHoldReason = HoldReason::from(hold.reason).into();
			<T as Config>::Currency::hold(&reason, who, amount)
				.map_err(|_| Error::FailedToProcessAccount)?;
		}

		Ok(minted)
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
			<T as Config>::Currency::release(&rc_reason, who, release, Precision::Exact)?;
		}
		Ok((release, wanted.saturating_sub(held)))
	}
}
