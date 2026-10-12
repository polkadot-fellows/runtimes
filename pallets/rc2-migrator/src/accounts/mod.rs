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

#![doc = include_str!("accounts.md")]

#[cfg(test)]
mod tests;

use crate::{Config, Limits, LOG_TARGET};
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::{
	storage::with_storage_layer,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
	},
};
use sp_runtime::{traits::Zero, DispatchError};

type NativeCurrency<T> = pallet_balances::Pallet<T>;
type AccountInfoFor<T> = frame_system::AccountInfo<
	<T as frame_system::Config>::Nonce,
	pallet_balances::AccountData<u128>,
>;

/// The accounts one block withdrew.
#[derive(Debug, PartialEq, Eq)]
pub struct BlockWithdrawals<AccountId> {
	/// Free balance to teleport to Asset Hub, per account.
	pub to_ah: Vec<(AccountId, u128)>,
	/// Where the next block continues from; `None` once every account has been visited.
	pub last_key: Option<AccountId>,
}

pub struct AccountsMigrator<T>(PhantomData<T>);

impl<T: Config> AccountsMigrator<T> {
	/// Withdraw accounts, continuing after `last_key`, up to the block's [`Limits`].
	///
	/// Each account is withdrawn in a storage transaction of its own, so one that fails is left
	/// untouched and the block goes on.
	pub fn migrate_many(last_key: Option<T::AccountId>) -> BlockWithdrawals<T::AccountId> {
		let limits = Limits::<T>::get();
		// as many accounts as the block's messages carry
		let max_to_ah =
			(limits.xcm_per_block as usize).saturating_mul(limits.items_per_xcm as usize);

		let mut iter = match last_key {
			Some(last_key) => frame_system::Account::<T>::iter_from_key(last_key),
			None => frame_system::Account::<T>::iter(),
		};

		let mut out = BlockWithdrawals { to_ah: Vec::new(), last_key: None };
		for _ in 0..limits.items_per_block {
			if out.to_ah.len() >= max_to_ah {
				break;
			}
			let Some((who, info)) = iter.next() else {
				out.last_key = None;
				return out;
			};
			match with_storage_layer(|| Self::withdraw_account(&who, &info)) {
				Ok(Some(amount)) => out.to_ah.push((who.clone(), amount)),
				Ok(None) => (),
				Err(e) => log::error!(target: LOG_TARGET, "Account {who:?} not withdrawn: {e:?}"),
			}
			out.last_key = Some(who);
		}
		out
	}

	/// Burn `who`'s balance and return what is teleported to Asset Hub, or `None` if the account
	/// is passed over.
	fn withdraw_account(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
	) -> Result<Option<u128>, DispatchError> {
		let data = &info.data;
		// TODO(ahm-v2): cases 2 to 18 of `accounts.md`. Until each has its rule, an account with
		// anything but free balance is passed over.
		if !data.reserved.is_zero() ||
			!data.frozen.is_zero() ||
			info.consumers != 0 ||
			data.free < NativeCurrency::<T>::minimum_balance()
		{
			return Ok(None);
		}

		NativeCurrency::<T>::burn_from(
			who,
			data.free,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite,
		)?;
		Ok(Some(data.free))
	}
}
