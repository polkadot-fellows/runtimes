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

//! Sweep stage: empties the leftover pots and reaps the dust the earlier stages left behind.
//!
//! - The configured pots (`Config::SweepAccounts`, the old treasury pot among them) have no owning
//!   account to migrate them with; their full balance is teleported to `Config::SweepBeneficiary`
//!   on Asset Hub. Balance book-kept as inactive issuance is reactivated on leaving.
//! - Below the existential deposit nothing meaningful can still be backed by the balance on a
//!   retiring chain, so dust accounts are zeroed — any hold or reserve with them — and reaped,
//!   their sum teleported to the same beneficiary. Zero-balance husks held alive only by a stale
//!   provider reference are reaped. Module accounts, and records something still references
//!   (session key-holders being the known case), survive.
//!
//! Runs after the accounts stage because most of what it reaps does not exist until then, and
//! before the total-issuance correction, which is only safe once the husks are gone.
//!
//! Shipping the proceeds over XCM and driving the stage from `on_initialize` is the stage
//! machine's job: [`SweepMigrator::sweep_pots`] runs once, then [`SweepMigrator::sweep_dust`]
//! once per block, each inside a storage transaction the caller owns. Both book what they burned
//! into the conservation ledger and hand it back to be teleported.

use crate::{accounts::AccountsMigrator, Config, Event, Pallet, RcMigratedBalance};
use core::marker::PhantomData;
use frame_support::traits::{
	fungible::{Inspect, Mutate, Unbalanced},
	tokens::{Fortitude, Precision, Preservation},
	Get,
};
use sp_runtime::{traits::Zero, AccountId32};

/// Maximum number of accounts scanned per relay-chain block by the dust pass.
///
/// Bounds the unbenchmarked work of one `on_initialize`.
pub const MAX_SWEPT_PER_BLOCK: u32 = 300;

type NativeCurrency<T> = pallet_balances::Pallet<T>;

/// Why a sweep block could not complete. The caller rolls the block back.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// A pot's balance could not be fully withdrawn.
	FailedToWithdrawAccount,
	/// The migrated/kept balance bookkeeping would overflow.
	BalanceAccounting,
}

/// Everything one block of the dust pass burned, ready to be teleported.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BlockSweep {
	/// Dust burned here, to teleport to the sweep beneficiary.
	pub amount: u128,
	/// Where the next block continues from; `None` once the account space is exhausted.
	pub last_key: Option<AccountId32>,
}

pub struct SweepMigrator<T>(PhantomData<T>);

impl<T: Config> SweepMigrator<T> {
	/// Empty the configured leftover pots. Returns the total to teleport to the sweep
	/// beneficiary on Asset Hub. One-shot: the pot list is a short config item.
	pub fn sweep_pots() -> Result<u128, Error> {
		let mut total: u128 = 0;

		// The configured pots (old treasury etc.): full free balance, no reserves expected.
		for who in T::SweepAccounts::get() {
			let amount = frame_system::Account::<T>::get(&who).data.free;
			if amount == 0 {
				continue;
			}
			let burned = NativeCurrency::<T>::burn_from(
				&who,
				amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite,
			)
			.map_err(|_| Error::FailedToWithdrawAccount)?;
			// Pot balances book-kept as inactive issuance (the treasury) must reactivate on
			// leaving so the issuance accounting stays consistent. Applied to every pot: one
			// that was never deactivated just floors the counter (reactivate saturates) toward
			// its true end state — zero, since no pot survives the sweep.
			NativeCurrency::<T>::reactivate(burned);
			total = total.checked_add(burned).ok_or(Error::BalanceAccounting)?;
			Pallet::<T>::deposit_event(Event::AccountSwept { who, amount: burned });
		}
		Self::book_swept(total)?;
		Ok(total)
	}

	/// One block of the dust pass: reap below-ED accounts and stale husks from the cursor on.
	///
	/// Below the existential deposit nothing meaningful can still be backed by the balance on a
	/// retiring chain, so any hold or reserve is killed — including the consumer reference the
	/// backing carried — and the account zeroed. The fungible APIs refuse to touch referenced or
	/// held-against accounts, hence the direct write (same pattern as the accounts-stage shell
	/// drain). A record survives only while something still references it (session key-holders
	/// being the known case) or it is a module account.
	pub fn sweep_dust(last_key: Option<T::AccountId>) -> Result<BlockSweep, Error> {
		// Get iterator starting after last processed key
		let mut iter = match &last_key {
			Some(last_key) => frame_system::Account::<T>::iter_from(
				frame_system::Account::<T>::hashed_key_for(last_key),
			),
			None => frame_system::Account::<T>::iter(),
		};

		let (mut dust_count, mut dust_amount, mut husk_count) = (0u32, 0u128, 0u32);
		let ed = NativeCurrency::<T>::minimum_balance();
		let mut processed = 0u32;
		let last_key = loop {
			let Some((who, info)) = iter.next() else { break None };
			processed += 1;

			let d = &info.data;
			let amount = d.free.saturating_add(d.reserved);
			if amount < ed && !Self::is_unmigrated(&who) {
				if amount == 0 {
					// A husk: exists only via a stale provider reference. `dec_providers`
					// refuses whenever something still references the account.
					if info.consumers == 0 && frame_system::Pallet::<T>::dec_providers(&who).is_ok()
					{
						husk_count += 1;
					}
				} else {
					let holds = pallet_balances::Holds::<T>::take(&who);
					let backed = !d.reserved.is_zero() || !holds.is_empty();
					frame_system::Account::<T>::mutate(&who, |a| {
						a.data.free = 0;
						a.data.reserved = 0;
					});
					pallet_balances::TotalIssuance::<T>::mutate(|ti| {
						*ti = ti.saturating_sub(amount)
					});
					// Reserves and holds collectively carry one consumer reference; killing the
					// backing drops it, so the record does not survive as a stale-ref shell.
					if backed && info.consumers > 0 {
						frame_system::Pallet::<T>::dec_consumers(&who);
					}
					let _ = frame_system::Pallet::<T>::dec_providers(&who);
					dust_count += 1;
					dust_amount =
						dust_amount.checked_add(amount).ok_or(Error::BalanceAccounting)?;
				}
			}

			if processed >= MAX_SWEPT_PER_BLOCK {
				break Some(who);
			}
		};

		if husk_count > 0 {
			Pallet::<T>::deposit_event(Event::HusksReaped { count: husk_count });
		}
		if dust_count > 0 {
			Pallet::<T>::deposit_event(Event::DustSwept { count: dust_count, amount: dust_amount });
		}
		Self::book_swept(dust_amount)?;
		Ok(BlockSweep { amount: dust_amount, last_key })
	}

	/// Whether `who` is an account class the migration never touches (module accounts).
	pub fn is_unmigrated(who: &T::AccountId) -> bool {
		AccountsMigrator::<T>::is_unmigrated(who)
	}

	/// Book `total` swept out of the kept balance as teleported to Asset Hub.
	fn book_swept(total: u128) -> Result<(), Error> {
		if total == 0 {
			return Ok(());
		}
		RcMigratedBalance::<T>::try_mutate(|t| {
			t.kept = t.kept.checked_sub(total).ok_or(Error::BalanceAccounting)?;
			t.ah_free = t.ah_free.checked_add(total).ok_or(Error::BalanceAccounting)?;
			Ok(())
		})
	}
}
