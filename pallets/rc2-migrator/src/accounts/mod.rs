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
	defensive_assert, ensure,
	storage::with_storage_layer,
	traits::{
		fungible::{Inspect, Mutate, MutateFreeze, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
		Contains, Get, LockableCurrency, ReservableCurrency, StorePreimage,
	},
	BoundedVec,
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
	AccountId32, DispatchError, DispatchResult,
};

type NativeCurrency<T> = pallet_balances::Pallet<T>;
pub type AccountInfoFor<T> = frame_system::AccountInfo<
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
	/// Registrar deposits of the paras the account manages; a `RegistrarDeposit` hold.
	pub registrar: u128,
	/// HRMP channel and open-request deposits of the para sovereign; an `HrmpDeposit` hold.
	pub hrmp: u128,
	/// Proxy deposits of a delegator with a portable definition; a `ProxyDeposit` hold.
	pub proxy: u128,
	/// Deposits with no purpose after the migration; they leave as free balance.
	pub refund: u128,
}

impl ExpectedReserve {
	fn get(&self, kind: DepositKind) -> u128 {
		match kind {
			DepositKind::Registrar => self.registrar,
			DepositKind::Hrmp => self.hrmp,
			DepositKind::Proxy => self.proxy,
			DepositKind::Refund => self.refund,
		}
	}

	fn get_mut(&mut self, kind: DepositKind) -> &mut u128 {
		match kind {
			DepositKind::Registrar => &mut self.registrar,
			DepositKind::Hrmp => &mut self.hrmp,
			DepositKind::Proxy => &mut self.proxy,
			DepositKind::Refund => &mut self.refund,
		}
	}
}

/// The kinds of deposit an [`ExpectedReserve`] is made of.
#[derive(Clone, Copy)]
enum DepositKind {
	Registrar,
	Hrmp,
	Proxy,
	Refund,
}

/// Where one withdrawn account's balance goes.
struct Withdrawal {
	/// Holds and the working buffer, minted on the Coretime chain.
	to_ct: Option<PortableAccount<AccountId32, u128>>,
	/// Free balance teleported to Asset Hub.
	to_ah: Option<(AccountId32, u128)>,
}

type ProxyDefinitionOf<T> = pallet_proxy::ProxyDefinition<
	<T as frame_system::Config>::AccountId,
	<T as pallet_proxy::Config>::ProxyType,
	pallet_proxy::BlockNumberFor<T>,
>;

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
	/// Release the preimage deposits and build `ExpectedReserves` from the records of the pallets
	/// that took the deposits. Returns the number of records read.
	///
	/// Runs once, in the `AccountsInit` block. Unbounded: a one-off over a few hundred records,
	/// worst case one slow relay chain block. Rebuilds the index from scratch, so a rewind does
	/// not double count.
	pub fn init() -> u32 {
		Self::release_preimage_deposits();

		let _ = ExpectedReserves::<T>::clear(u32::MAX, None);
		let mut records = 0;

		// registrar deposits, reserved on the para manager
		for (_, info) in paras_registrar::Paras::<T>::iter() {
			Self::add_expected(info.manager, DepositKind::Registrar, info.deposit);
			records += 1;
		}

		// HRMP channel deposits, reserved on both sovereigns. A channel about to close returns
		// them, so they are refunded.
		for (id, channel) in hrmp::HrmpChannels::<T>::iter() {
			let kind = if hrmp::HrmpCloseChannelRequests::<T>::contains_key(&id) {
				DepositKind::Refund
			} else {
				DepositKind::Hrmp
			};
			Self::add_expected(id.sender.into_account_truncating(), kind, channel.sender_deposit);
			Self::add_expected(
				id.recipient.into_account_truncating(),
				kind,
				channel.recipient_deposit,
			);
			records += 1;
		}

		// HRMP open-request deposits. The recipient's is reserved on acceptance at the active
		// configuration's value and only recorded once the channel opens; zero when either side
		// is a system chain.
		let recipient_deposit = configuration::ActiveConfig::<T>::get().hrmp_recipient_deposit;
		for (id, request) in hrmp::HrmpOpenChannelRequests::<T>::iter() {
			Self::add_expected(
				id.sender.into_account_truncating(),
				DepositKind::Hrmp,
				request.sender_deposit,
			);
			if request.confirmed && !id.sender.is_system() && !id.recipient.is_system() {
				Self::add_expected(
					id.recipient.into_account_truncating(),
					DepositKind::Hrmp,
					recipient_deposit,
				);
			}
			records += 1;
		}

		// Proxy deposits, reserved on the delegator, or on the creator for a pure proxy. A set
		// with a portable permission continues on the Coretime chain, the others are refunded.
		for (delegator, (defs, deposit)) in pallet_proxy::Proxies::<T>::iter() {
			let kind = if defs.iter().any(|d| Self::portable(&d.proxy_type).is_some()) {
				DepositKind::Proxy
			} else {
				DepositKind::Refund
			};
			let who = Self::pure_creator(&delegator, &defs).unwrap_or(delegator);
			Self::add_expected(who, kind, deposit);
			records += 1;
		}

		// announcements are not migrated: refunded
		for (announcer, (_, deposit)) in pallet_proxy::Announcements::<T>::iter() {
			Self::add_expected(announcer, DepositKind::Refund, deposit);
			records += 1;
		}

		// multisig operations cannot complete here: refunded
		for (_, _, op) in pallet_multisig::Multisigs::<T>::iter() {
			Self::add_expected(op.depositor, DepositKind::Refund, op.deposit);
			records += 1;
		}

		records
	}

	/// Release every preimage deposit: nothing dispatches a preimage here after the migration. An
	/// unrequested blob is removed; a requested one stays, its ticket dropped. Unbounded, see
	/// [`Self::init`].
	fn release_preimage_deposits() {
		let with_deposit = Self::preimage_deposits();
		let count = with_deposit.len();
		for hash in with_deposit {
			// the pallet's own path, minus the ownership check
			<pallet_preimage::Pallet<T> as StorePreimage>::unnote(&hash);
		}
		if count > 0 {
			log::info!(target: LOG_TARGET, "Released {count} preimage deposit(s)");
		}
	}

	/// The preimages whose request status still holds a deposit. Unbounded, see [`Self::init`].
	pub fn preimage_deposits() -> Vec<T::Hash> {
		pallet_preimage::RequestStatusFor::<T>::iter()
			.filter(|(_, status)| match status {
				pallet_preimage::RequestStatus::Unrequested { .. } => true,
				pallet_preimage::RequestStatus::Requested { maybe_ticket, .. } =>
					maybe_ticket.is_some(),
			})
			.map(|(hash, _)| hash)
			.collect()
	}

	/// What `proxy_type` is on the Coretime chain, if anything.
	fn portable(proxy_type: &<T as pallet_proxy::Config>::ProxyType) -> Option<PortableProxyType> {
		proxy_type.clone().try_into().ok()
	}

	/// The creator of a pure proxy, who reserved its deposit: the `Any` delegate of a delegator
	/// that never signed. See [`Self::is_pure_proxy`].
	fn pure_creator(
		delegator: &T::AccountId,
		defs: &[ProxyDefinitionOf<T>],
	) -> Option<T::AccountId> {
		if !frame_system::Account::<T>::get(delegator).nonce.is_zero() {
			return None;
		}
		Self::any_delegate(defs).cloned()
	}

	/// The delegate of the first definition that is `Any` on the Coretime chain.
	fn any_delegate(defs: &[ProxyDefinitionOf<T>]) -> Option<&T::AccountId> {
		defs.iter()
			.find(|d| Self::portable(&d.proxy_type) == Some(PortableProxyType::Any))
			.map(|d| &d.delegate)
	}

	/// Add `amount` to `who`'s expected reserve of `kind`.
	fn add_expected(who: T::AccountId, kind: DepositKind, amount: u128) {
		if amount.is_zero() {
			return;
		}
		ExpectedReserves::<T>::mutate(&who, |e| {
			let v = e.get_mut(kind);
			*v = v.saturating_add(amount);
		});
	}

	/// Withdraw accounts, continuing after `last_key`, up to the block's [`Limits`]. The
	/// `manager` is left alone: it pays for the calls that drive the migration.
	///
	/// Each account is withdrawn in a storage transaction of its own, so one that fails is left
	/// as it was and the block goes on.
	pub fn migrate_many(
		last_key: Option<T::AccountId>,
		manager: Option<&T::AccountId>,
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
			if !Self::deferred(&who, &info, manager) {
				match with_storage_layer(|| Self::withdraw_account(&who, &info)) {
					Ok(Withdrawal { to_ct, to_ah }) => {
						out.to_ct.extend(to_ct);
						out.to_ah.extend(to_ah);
					},
					Err(error) => {
						log::error!(target: LOG_TARGET, "Account {who:?} not withdrawn: {error:?}");
						Pallet::<T>::deposit_event(Event::AccountNotWithdrawn {
							who: who.clone(),
							error,
						});
					},
				}
			}
			out.last_key = Some(who);
		}
		out
	}

	/// Whether a later stage handles `who`: a module account (Sweep), one below the existential
	/// deposit (Sweep Dust), or the `manager` (Cool Off).
	pub fn deferred(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
		manager: Option<&T::AccountId>,
	) -> bool {
		Self::is_swept(who) ||
			info.data.total() < NativeCurrency::<T>::minimum_balance() ||
			manager == Some(who)
	}

	/// Withdraw `who` whole and say where its balance goes. One step per case of `accounts.md`.
	fn withdraw_account(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
	) -> Result<Withdrawal, DispatchError> {
		// release everything, so the whole balance can be burned
		let mut holds = BoundedVec::new();
		if !info.data.reserved.is_zero() {
			Self::release_holds(who)?;
			holds = Self::withdraw_reserved(who)?;
		}
		if !info.data.frozen.is_zero() {
			Self::release_locks(who)?;
		}

		// burn; a release may have dropped a consumer ref, so read the account again
		let account = frame_system::Account::<T>::get(who);
		defensive_assert!(
			account.data.reserved.is_zero() && account.data.frozen.is_zero(),
			"everything was released"
		);
		let total = account.data.free;
		// A consumer left after the releases are from the session keys of a validator: the
		// account should not be reaped, only emptied.
		if account.consumers != 0 {
			Self::drain_to_shell(who, total);
		} else {
			NativeCurrency::<T>::burn_from(
				who,
				total,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite,
			)?;
		}

		Ok(Self::route(who, Self::is_pure_proxy(who, info), total, holds))
	}

	/// Split what was burned by destination: holds and a buffer of free balance to the Coretime
	/// chain, the rest to Asset Hub. A pure proxy goes to the Coretime chain whole.
	fn route(
		who: &T::AccountId,
		pure: bool,
		total: u128,
		holds: BoundedVec<PortableHold<u128>, MaxPortableHolds>,
	) -> Withdrawal {
		// what the holds do not take is liquid
		let held: u128 = holds.iter().map(|h| h.amount).sum();
		let liquid = total.saturating_sub(held);
		let mut ct_free = if pure {
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
		// a remainder below this chain's existential deposit follows the holds, so what is
		// teleported endows its account on Asset Hub, whose ED is lower.
		if ah_free < NativeCurrency::<T>::minimum_balance() {
			ct_free = ct_free.saturating_add(ah_free);
			ah_free = 0;
		}

		// TODO(ahm-v2): Asset Hub's own sovereign lands on `sibl` + 1000 on Asset Hub, which
		// nothing there controls. Send it to the sweep beneficiary instead.
		let dest = migrator_types::translate_destination(who);
		let to_ct = (!holds.is_empty() || !ct_free.is_zero()).then(|| PortableAccount {
			who: dest.clone(),
			free: ct_free,
			holds,
		});
		let to_ah = (!ah_free.is_zero()).then_some((dest, ah_free));
		Withdrawal { to_ct, to_ah }
	}

	/// Release `who`'s reserve and attribute it to the deposits recorded for the account.
	/// Returns the holds that continue on the Coretime chain, what no record explains included;
	/// refunds stay free.
	fn withdraw_reserved(
		who: &T::AccountId,
	) -> Result<BoundedVec<PortableHold<u128>, MaxPortableHolds>, DispatchError> {
		// read again: the holds just released were part of it
		let reserved = frame_system::Account::<T>::get(who).data.reserved;
		if reserved.is_zero() {
			return Ok(BoundedVec::new());
		}
		let expected = ExpectedReserves::<T>::take(who);

		// deposits are unnamed reserves: release them all
		let not_released = NativeCurrency::<T>::unreserve(who, reserved);
		ensure!(not_released.is_zero(), Error::<T>::FailedToWithdrawAccount);

		// attribute in priority order; a short reserve goes to the deposits that continue first
		let mut remainder = reserved;
		let mut take = |cap: u128| {
			let taken = remainder.min(cap);
			remainder -= taken;
			taken
		};
		let mut holds = BoundedVec::new();
		for (kind, reason) in [
			(DepositKind::Registrar, PortableHoldReason::RegistrarDeposit),
			(DepositKind::Hrmp, PortableHoldReason::HrmpDeposit),
			(DepositKind::Proxy, PortableHoldReason::ProxyDeposit),
		] {
			let amount = take(expected.get(kind));
			if !amount.is_zero() {
				Self::push_hold(&mut holds, reason, amount)?;
			}
		}
		let refund = take(expected.refund);
		if !refund.is_zero() {
			Pallet::<T>::deposit_event(Event::DepositRefunded { who: who.clone(), amount: refund });
		}
		// what no record explains is parked on the Coretime chain
		if !remainder.is_zero() {
			Pallet::<T>::deposit_event(Event::UnattributedReserve {
				who: who.clone(),
				amount: remainder,
			});
			Self::push_hold(&mut holds, PortableHoldReason::UnattributedReserve, remainder)?;
		}
		Ok(holds)
	}

	/// One hold per reason, which is what the bound allows.
	fn push_hold(
		holds: &mut BoundedVec<PortableHold<u128>, MaxPortableHolds>,
		reason: PortableHoldReason,
		amount: u128,
	) -> DispatchResult {
		holds
			.try_push(PortableHold { reason, amount })
			.map_err(|_| Error::<T>::FailedToWithdrawAccount.into())
	}

	/// Release every named hold on `who`. Preimage deposits went through their pallet when the
	/// stage started; anything else is a leftover of AHM v1 and leaves as free balance.
	fn release_holds(who: &T::AccountId) -> DispatchResult {
		for hold in pallet_balances::Holds::<T>::get(who) {
			NativeCurrency::<T>::release(&hold.id, who, hold.amount, Precision::Exact)?;
		}
		Ok(())
	}

	/// Release every lock and freeze on `who`. The pallets that set them left in AHM v1; the
	/// balance leaves as free.
	fn release_locks(who: &T::AccountId) -> DispatchResult {
		for lock in pallet_balances::Locks::<T>::get(who) {
			NativeCurrency::<T>::remove_lock(lock.id, who);
		}
		for freeze in pallet_balances::Freezes::<T>::get(who) {
			NativeCurrency::<T>::thaw(&freeze.id, who)?;
		}
		Ok(())
	}

	/// An account another pallet references (a validator's session keys): the record stays as a
	/// zero-balance shell, the balance leaves like any other. The fungible API would keep the
	/// existential deposit of an account it cannot reap, so the balance is written directly.
	fn drain_to_shell(who: &T::AccountId, amount: u128) {
		frame_system::Account::<T>::mutate(who, |a| {
			defensive_assert!(a.data.free == amount, "drained what the caller read");
			a.data.free = 0;
		});
		pallet_balances::TotalIssuance::<T>::mutate(|ti| *ti = ti.saturating_sub(amount));
		Pallet::<T>::deposit_event(Event::AccountShellDrained { who: who.clone(), amount });
	}

	/// A pure proxy: never signed, and has an `Any` delegate. A pure is created with `Any`, and
	/// an account that never signed cannot have been anything else.
	pub fn is_pure_proxy(who: &T::AccountId, info: &AccountInfoFor<T>) -> bool {
		info.nonce.is_zero() &&
			Self::any_delegate(&pallet_proxy::Proxies::<T>::get(who).0).is_some()
	}

	/// Whether Sweep empties `who`, [`Config::SweptAccounts`].
	pub fn is_swept(who: &T::AccountId) -> bool {
		T::SweptAccounts::contains(who)
	}
}
