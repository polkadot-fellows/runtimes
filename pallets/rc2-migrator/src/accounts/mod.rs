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

use crate::{Config, Error, Event, ExpectedReserves, Limits, Pallet, LOG_TARGET};
use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_support::{
	ensure,
	storage::with_storage_layer,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		Get, ReservableCurrency,
	},
	BoundedVec, PalletId,
};
use migrator_types::{
	MaxPortableHolds, PortableAccount, PortableHold, PortableHoldReason, PortableProxyType,
};
use polkadot_parachain_primitives::primitives::IsSystem;
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::{configuration, hrmp};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	AccountId32, DispatchError, DispatchResult, TypeId,
};

type NativeCurrency<T> = pallet_balances::Pallet<T>;
type AccountInfoFor<T> = frame_system::AccountInfo<
	<T as frame_system::Config>::Nonce,
	pallet_balances::AccountData<u128>,
>;

/// What an account's reserved balance is made of, from the records of the pallets that took the
/// deposits. See `ExpectedReserves`.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct ExpectedReserve {
	/// Registrar deposits recorded for the account as para manager. They continue on the
	/// Coretime chain as a `RegistrarDeposit` hold.
	pub registrar: u128,
	/// HRMP channel and open-request deposits recorded for the account as para sovereign. They
	/// continue as an `HrmpDeposit` hold.
	pub hrmp: u128,
	/// Proxy deposits of a delegator with at least one portable definition. They continue as a
	/// `ProxyDeposit` hold.
	pub proxy: u128,
	/// Deposits whose purpose ends with the Relay Chain. Refunded: they leave as free balance.
	pub refund: u128,
}

/// Where one withdrawn account's balance goes.
#[derive(Debug, PartialEq, Eq)]
pub struct Withdrawal {
	/// Holds and the working buffer, minted on the Coretime chain.
	pub to_ct: Option<PortableAccount<AccountId32, u128>>,
	/// Free balance teleported to Asset Hub.
	pub to_ah: Option<(AccountId32, u128)>,
}

/// The accounts one block withdrew.
#[derive(Debug, PartialEq, Eq)]
pub struct BlockWithdrawals {
	/// Accounts to mint on the Coretime chain, holds included.
	pub to_ct: Vec<PortableAccount<AccountId32, u128>>,
	/// Free balance to teleport to Asset Hub, per account.
	pub to_ah: Vec<(AccountId32, u128)>,
	/// Where the next block continues from; `None` once every account has been visited.
	pub last_key: Option<AccountId32>,
}

pub struct AccountsMigrator<T>(PhantomData<T>);

impl<T: Config> AccountsMigrator<T> {
	/// Build `ExpectedReserves` from the records of the pallets that took the deposits. Returns
	/// the number of records read.
	///
	/// Runs in one block, from the `AccountsInit` stage. The records are a few hundred on the
	/// relay chain, and the worst case is one slow relay-chain block. Rebuilds from scratch, so
	/// a rewind to `AccountsInit` does not double count.
	pub fn init() -> u32 {
		let _ = ExpectedReserves::<T>::clear(u32::MAX, None);
		let mut records = 0;

		// registrar deposits, reserved on the para manager
		// This is unbounded but only on RC. We don't expect many records (<150) but in any case,
		// this can only cause a slightly slower block.
		for (_, info) in paras_registrar::Paras::<T>::iter() {
			Self::add_expected(info.manager, info.deposit, |e| &mut e.registrar);
			records += 1;
		}

		// HRMP channel deposits, reserved on both sovereigns. A channel with a pending close
		// request is on its way to returning them, so they are refunded instead.
		for (id, channel) in hrmp::HrmpChannels::<T>::iter() {
			let of: fn(&mut ExpectedReserve) -> &mut u128 =
				if hrmp::HrmpCloseChannelRequests::<T>::contains_key(&id) {
					|e| &mut e.refund
				} else {
					|e| &mut e.hrmp
				};
			Self::add_expected(id.sender.into_account_truncating(), channel.sender_deposit, of);
			Self::add_expected(
				id.recipient.into_account_truncating(),
				channel.recipient_deposit,
				of,
			);
			records += 1;
		}

		// HRMP open-request deposits: the sender's as recorded, and the recipient's once it has
		// accepted, which is reserved at the active configuration's value and recorded only when
		// the next session turns the request into a channel. Zero when either side is a system
		// chain.
		let recipient_deposit = configuration::ActiveConfig::<T>::get().hrmp_recipient_deposit;
		for (id, request) in hrmp::HrmpOpenChannelRequests::<T>::iter() {
			Self::add_expected(id.sender.into_account_truncating(), request.sender_deposit, |e| {
				&mut e.hrmp
			});
			if request.confirmed && !id.sender.is_system() && !id.recipient.is_system() {
				Self::add_expected(
					id.recipient.into_account_truncating(),
					recipient_deposit,
					|e| &mut e.hrmp,
				);
			}
			records += 1;
		}

		// Proxy deposits, reserved on the delegator, except a pure proxy's, which its creator
		// reserved. Definitions with a portable permission continue on the Coretime chain; the
		// others' deposit is refunded.
		for (delegator, (defs, deposit)) in pallet_proxy::Proxies::<T>::iter() {
			let portable = defs.iter().any(|d| Self::portable(&d.proxy_type).is_some());
			let of: fn(&mut ExpectedReserve) -> &mut u128 =
				if portable { |e| &mut e.proxy } else { |e| &mut e.refund };
			let who = Self::pure_creator(&delegator, defs.as_slice(), deposit).unwrap_or(delegator);
			Self::add_expected(who, deposit, of);
			records += 1;
		}

		// Proxy announcements are not migrated: the deposit, reserved on the announcer, is
		// refunded.
		for (announcer, (_, deposit)) in pallet_proxy::Announcements::<T>::iter() {
			Self::add_expected(announcer, deposit, |e| &mut e.refund);
			records += 1;
		}

		// Multisig operations cannot complete on a retired chain: the deposit, reserved on the
		// depositor, is refunded.
		for (_, _, op) in pallet_multisig::Multisigs::<T>::iter() {
			Self::add_expected(op.depositor, op.deposit, |e| &mut e.refund);
			records += 1;
		}

		records
	}

	/// What `proxy_type` is on the Coretime chain, if anything.
	fn portable(proxy_type: &<T as pallet_proxy::Config>::ProxyType) -> Option<PortableProxyType> {
		proxy_type.clone().try_into().ok()
	}

	/// The creator of a pure proxy, who holds its deposit: a delegator that never signed, has an
	/// `Any` delegate, and whose own reserve does not cover what is expected of it. The deposit
	/// goes to that delegate.
	fn pure_creator(
		delegator: &T::AccountId,
		defs: &[pallet_proxy::ProxyDefinition<
			T::AccountId,
			<T as pallet_proxy::Config>::ProxyType,
			pallet_proxy::BlockNumberFor<T>,
		>],
		deposit: u128,
	) -> Option<T::AccountId> {
		let account = frame_system::Account::<T>::get(delegator);
		if !account.nonce.is_zero() {
			return None;
		}
		let any = defs
			.iter()
			.find(|d| Self::portable(&d.proxy_type) == Some(PortableProxyType::Any))?;
		let expected = ExpectedReserves::<T>::get(delegator);
		let needed = expected
			.registrar
			.saturating_add(expected.hrmp)
			.saturating_add(expected.proxy)
			.saturating_add(expected.refund)
			.saturating_add(deposit);
		(account.data.reserved < needed).then(|| any.delegate.clone())
	}

	/// Add `amount` to `who`'s expected reserve, under the deposit kind `of` picks.
	fn add_expected(who: T::AccountId, amount: u128, of: fn(&mut ExpectedReserve) -> &mut u128) {
		if amount.is_zero() {
			return;
		}
		ExpectedReserves::<T>::mutate(&who, |e| {
			let v = of(e);
			*v = v.saturating_add(amount);
		});
	}

	/// Withdraw accounts, continuing after `last_key`, up to the block's [`Limits`]. `exceptions`
	/// are left alone: the migration manager, which pays for the calls that drive the migration.
	///
	/// Each account is withdrawn in a storage transaction of its own, so one that fails is left
	/// untouched and the block goes on.
	pub fn migrate_many(
		last_key: Option<T::AccountId>,
		exceptions: &[T::AccountId],
	) -> BlockWithdrawals {
		let mut iter = match last_key {
			Some(last_key) => frame_system::Account::<T>::iter_from_key(last_key),
			None => frame_system::Account::<T>::iter(),
		};

		let mut out = BlockWithdrawals { to_ct: Vec::new(), to_ah: Vec::new(), last_key: None };
		for _ in 0..Limits::<T>::get().items_per_block() {
			let Some((who, info)) = iter.next() else {
				out.last_key = None;
				return out;
			};
			match with_storage_layer(|| Self::withdraw_account(&who, &info, exceptions)) {
				Ok(Some(Withdrawal { to_ct, to_ah })) => {
					out.to_ct.extend(to_ct);
					out.to_ah.extend(to_ah);
				},
				Ok(None) => (),
				Err(e) => log::error!(target: LOG_TARGET, "Account {who:?} not withdrawn: {e:?}"),
			}
			out.last_key = Some(who);
		}
		out
	}

	/// Withdraw `who` whole and say where its balance goes, or `None` if nothing leaves. One
	/// step per case of `accounts.md`.
	fn withdraw_account(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
		exceptions: &[T::AccountId],
	) -> Result<Option<Withdrawal>, DispatchError> {
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

		// TODO(ahm-v2): named holds, locks and freezes. Until they have their rule, such an
		// account is passed over.
		if !data.frozen.is_zero() || !pallet_balances::Holds::<T>::get(who).is_empty() {
			return Ok(None);
		}

		// Release reserves, holds and locks first, so the whole balance can be burned.
		let holds = Self::withdraw_reserved(who, info)?;
		Self::release_holds(who, info)?;
		Self::release_locks(who, info)?;

		// Burn the whole balance here. The release above may have dropped a consumer reference,
		// so read the account again.
		let account = frame_system::Account::<T>::get(who);
		let total = account.data.free;
		if account.consumers != 0 {
			Self::drain_to_shell(who, total);
		} else {
			Self::burn_free(who, total)?;
		}

		// What the holds do not take is liquid.
		let held: u128 = holds.iter().map(|h| h.amount).sum();
		Ok(Some(Self::route(who, info, total.saturating_sub(held), holds, held)?))
	}

	/// Split what was burned by destination: the holds and a working buffer of the liquid
	/// balance go to the Coretime chain, the rest to Asset Hub. A pure proxy's whole balance
	/// goes to the Coretime chain.
	fn route(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
		liquid: u128,
		holds: BoundedVec<PortableHold<u128>, MaxPortableHolds>,
		held: u128,
	) -> Result<Withdrawal, DispatchError> {
		let mut ct_free = if Self::is_pure_proxy(who, info) {
			// all goes to CT
			liquid
		} else if held.is_zero() {
			// no deposit: all goes to AH
			0
		} else {
			// a buffer goes to CT with the deposit, the rest to AH
			liquid.min(T::CtFreeBuffer::get())
		};
		let mut ah_free = liquid.saturating_sub(ct_free);
		// A remainder below the existential deposit follows the holds instead of being teleported.
		if !held.is_zero() && ah_free < NativeCurrency::<T>::minimum_balance() {
			ct_free = ct_free.saturating_add(ah_free);
			ah_free = 0;
		}

		let dest = Self::destination(who);
		let to_ct = (!held.is_zero() || !ct_free.is_zero()).then(|| PortableAccount {
			who: dest.clone(),
			free: ct_free,
			holds,
		});
		let to_ah = (!ah_free.is_zero()).then_some((dest, ah_free));
		Ok(Withdrawal { to_ct, to_ah })
	}

	/// Burn `who`'s free balance; the account is reaped.
	fn burn_free(who: &T::AccountId, amount: u128) -> DispatchResult {
		NativeCurrency::<T>::burn_from(
			who,
			amount,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite,
		)?;
		Ok(())
	}

	/// Release `who`'s reserve and attribute it to the deposits recorded for the account.
	/// Returns the holds that continue on the Coretime chain, including what no record
	/// explains; refunded deposits stay free.
	fn withdraw_reserved(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
	) -> Result<BoundedVec<PortableHold<u128>, MaxPortableHolds>, DispatchError> {
		let reserved = info.data.reserved;
		if reserved.is_zero() {
			return Ok(BoundedVec::new());
		}
		let expected = ExpectedReserves::<T>::take(who);

		// Deposits are unnamed reserves; release them all.
		let not_released = NativeCurrency::<T>::unreserve(who, reserved);
		ensure!(not_released.is_zero(), Error::<T>::FailedToWithdrawAccount);

		// Attribute the reserve in priority order: registrar, HRMP, proxy, then refunds. A reserve
		// that covers less than the records goes to the deposits that continue first.
		let mut remainder = reserved;
		let mut consume = |cap: u128| {
			let taken = remainder.min(cap);
			remainder -= taken;
			taken
		};
		let registrar = consume(expected.registrar);
		let hrmp = consume(expected.hrmp);
		let proxy = consume(expected.proxy);
		let refund = consume(expected.refund);
		if !refund.is_zero() {
			Pallet::<T>::deposit_event(Event::DepositRefunded { who: who.clone(), amount: refund });
		}
		// What no record explains travels too, parked on the Coretime chain under its own reason.
		let unattributed = remainder;
		if !unattributed.is_zero() {
			Pallet::<T>::deposit_event(Event::UnattributedReserve {
				who: who.clone(),
				amount: unattributed,
			});
		}

		// one hold per reason, so the bound holds by construction
		let mut holds = BoundedVec::new();
		for (reason, amount) in [
			(PortableHoldReason::RegistrarDeposit, registrar),
			(PortableHoldReason::HrmpDeposit, hrmp),
			(PortableHoldReason::ProxyDeposit, proxy),
			(PortableHoldReason::UnattributedReserve, unattributed),
		] {
			if !amount.is_zero() {
				holds
					.try_push(PortableHold { reason, amount })
					.map_err(|_| Error::<T>::FailedToWithdrawAccount)?;
			}
		}
		Ok(holds)
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
	///
	/// The fungible API keeps the existential deposit of an account it cannot reap, so the
	/// balance is written directly and the issuance adjusted to match.
	fn drain_to_shell(who: &T::AccountId, amount: u128) {
		frame_system::Account::<T>::mutate(who, |a| a.data.free = 0);
		pallet_balances::TotalIssuance::<T>::mutate(|ti| *ti = ti.saturating_sub(amount));
		Pallet::<T>::deposit_event(Event::AccountShellDrained { who: who.clone(), amount });
	}

	/// A pure proxy: nonce 0 and at least one `Any` proxy. A pure is always created with `Any`,
	/// and an account that never signed cannot have been anything else.
	fn is_pure_proxy(who: &T::AccountId, info: &AccountInfoFor<T>) -> bool {
		info.nonce.is_zero() &&
			pallet_proxy::Proxies::<T>::get(who)
				.0
				.iter()
				.any(|d| Self::portable(&d.proxy_type) == Some(PortableProxyType::Any))
	}

	/// The account `who`'s balance lands on: a para's sovereign is `para…` here and `sibl…` on
	/// Asset Hub and the Coretime chain; every other account keeps its address.
	// TODO(ahm-v2): Asset Hub's own sovereign (`para` + 1000) lands on `sibl` + 1000 on Asset
	// Hub, which nothing there controls. Send it to the sweep beneficiary instead. It holds DOT
	// here from HRMP channel deposits made before system channels became deposit-free, and from
	// whatever was sent to it since.
	fn destination(who: &T::AccountId) -> AccountId32 {
		migrator_types::translate_destination(who)
	}

	/// Whether `who` is a pallet (module) account, `modl` + pallet id.
	pub fn is_pallet_account(who: &T::AccountId) -> bool {
		let bytes: &[u8] = who.as_ref();
		bytes.starts_with(&<PalletId as TypeId>::TYPE_ID)
	}
}
