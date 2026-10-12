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
	PalletId,
};
use sp_runtime::{traits::Zero, DispatchError, DispatchResult, TypeId};

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
	/// Withdraw accounts, continuing after `last_key`, up to the block's [`Limits`]. `exceptions`
	/// are left alone: the migration manager, which pays for the calls that drive the migration.
	///
	/// Each account is withdrawn in a storage transaction of its own, so one that fails is left
	/// untouched and the block goes on.
	pub fn migrate_many(
		last_key: Option<T::AccountId>,
		exceptions: &[T::AccountId],
	) -> BlockWithdrawals<T::AccountId> {
		let mut iter = match last_key {
			Some(last_key) => frame_system::Account::<T>::iter_from_key(last_key),
			None => frame_system::Account::<T>::iter(),
		};

		let mut out = BlockWithdrawals { to_ah: Vec::new(), last_key: None };
		for _ in 0..Limits::<T>::get().items_per_block() {
			let Some((who, info)) = iter.next() else {
				out.last_key = None;
				return out;
			};
			match with_storage_layer(|| Self::withdraw_account(&who, &info, exceptions)) {
				Ok(Some(amount)) => out.to_ah.push((Self::destination(&who), amount)),
				Ok(None) => (),
				Err(e) => log::error!(target: LOG_TARGET, "Account {who:?} not withdrawn: {e:?}"),
			}
			out.last_key = Some(who);
		}
		out
	}

	/// Withdraw `who` and return what is teleported to Asset Hub, or `None` if nothing leaves.
	/// One step per case of `accounts.md`.
	fn withdraw_account(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
		exceptions: &[T::AccountId],
	) -> Result<Option<u128>, DispatchError> {
		let data = &info.data;

		// Nothing to do here for these; a later stage handles them.
		if Self::is_pallet_account(who) {
			// Sweep empties the pots.
			return Ok(None);
		}
		if data.total() < NativeCurrency::<T>::minimum_balance() {
			// Sweep Dust reaps them.
			return Ok(None);
		}
		if exceptions.contains(who) {
			// The manager, reaped at Cool Off.
			return Ok(None);
		}

		// Release reserves, holds and locks first, so the whole balance can be burned.
		Self::withdraw_reserved(who, info)?;
		Self::release_holds(who, info)?;
		Self::release_locks(who, info)?;
		// TODO(ahm-v2): drop once the three above are implemented. Until then an account with
		// anything but free balance is passed over.
		if !data.reserved.is_zero() || !data.frozen.is_zero() {
			return Ok(None);
		}

		// Referenced accounts keep their record here as a zero-balance shell.
		if info.consumers != 0 {
			return Self::drain_to_shell(who, info);
		}

		// Pure proxies send everything to the Coretime chain.
		if Self::is_pure_proxy(who, info) {
			return Self::withdraw_to_coretime(who, info);
		}

		// Free balance is burned here and teleported to Asset Hub.
		Self::withdraw_free(who, info)
	}

	/// Burn `who`'s free balance; it is teleported to Asset Hub.
	fn withdraw_free(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
	) -> Result<Option<u128>, DispatchError> {
		let free = info.data.free;
		NativeCurrency::<T>::burn_from(
			who,
			free,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite,
		)?;
		Ok(Some(free))
	}

	/// Deposits: registrar, HRMP, proxy, announcement, multisig, and reserve no record explains.
	// TODO(ahm-v2): not implemented; such accounts are passed over.
	fn withdraw_reserved(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> DispatchResult {
		Ok(())
	}

	/// Named holds: preimage deposits, and leftovers from pallets that left in AHM v1.
	// TODO(ahm-v2): not implemented; such accounts are passed over.
	fn release_holds(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> DispatchResult {
		Ok(())
	}

	/// Locks and freezes.
	// TODO(ahm-v2): not implemented; such accounts are passed over.
	fn release_locks(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> DispatchResult {
		Ok(())
	}

	/// An account another pallet references (validators' session keys): the record stays as a
	/// zero-balance shell, the balance leaves like any other.
	// TODO(ahm-v2): not implemented; such accounts are passed over.
	fn drain_to_shell(
		_who: &T::AccountId,
		_info: &AccountInfoFor<T>,
	) -> Result<Option<u128>, DispatchError> {
		Ok(None)
	}

	/// A pure proxy: nonce 0 and at least one `Any` proxy.
	// TODO(ahm-v2): not implemented.
	fn is_pure_proxy(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> bool {
		false
	}

	/// A pure proxy's whole balance goes to the Coretime chain, where its delegate keeps control.
	// TODO(ahm-v2): not implemented.
	fn withdraw_to_coretime(
		_who: &T::AccountId,
		_info: &AccountInfoFor<T>,
	) -> Result<Option<u128>, DispatchError> {
		Ok(None)
	}

	/// The account `who`'s balance lands on: `para…` sovereigns become `sibl…`, `sibl…` passes
	/// through, and system paras' own sovereigns go to the sweep beneficiary.
	// TODO(ahm-v2): not implemented; every account keeps its address.
	fn destination(who: &T::AccountId) -> T::AccountId {
		who.clone()
	}

	/// Whether `who` is a pallet (module) account, `modl` + pallet id.
	pub fn is_pallet_account(who: &T::AccountId) -> bool {
		let bytes: &[u8] = who.as_ref();
		bytes.starts_with(&<PalletId as TypeId>::TYPE_ID)
	}
}
