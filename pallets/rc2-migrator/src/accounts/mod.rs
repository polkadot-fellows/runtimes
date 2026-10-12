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
		fungible::{Inspect, Mutate, MutateFreeze, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
		Get, LockableCurrency, ReservableCurrency, StorePreimage,
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

	fn total(&self) -> u128 {
		self.registrar
			.saturating_add(self.hrmp)
			.saturating_add(self.proxy)
			.saturating_add(self.refund)
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
	/// Runs once, in the `AccountsInit` block. Every loop in it, and the clear, walks a whole map
	/// unbounded. That is acceptable here and nowhere else: this is a one-off on the relay chain,
	/// the maps hold a few hundred records, and the worst case is one slow relay-chain block.
	/// Rebuilds the index from scratch, so a rewind to `AccountsInit` does not double count.
	pub fn init() -> u32 {
		Self::release_preimage_deposits();

		let _ = ExpectedReserves::<T>::clear(u32::MAX, None);
		let mut records = 0;

		// registrar deposits, reserved on the para manager
		for (_, info) in paras_registrar::Paras::<T>::iter() {
			Self::add_expected(info.manager, DepositKind::Registrar, info.deposit);
			records += 1;
		}

		// HRMP channel deposits, reserved on both sovereigns. A channel with a pending close
		// request is on its way to returning them, so they are refunded instead.
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

		// HRMP open-request deposits: the sender's as recorded, and the recipient's once it has
		// accepted, which is reserved at the active configuration's value and recorded only when
		// the next session turns the request into a channel. Zero when either side is a system
		// chain.
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

		// Proxy deposits, reserved on the delegator, except a pure proxy's, which its creator
		// reserved. Definitions with a portable permission continue on the Coretime chain; the
		// others' deposit is refunded.
		for (delegator, (defs, deposit)) in pallet_proxy::Proxies::<T>::iter() {
			let kind = if defs.iter().any(|d| Self::portable(&d.proxy_type).is_some()) {
				DepositKind::Proxy
			} else {
				DepositKind::Refund
			};
			let who = Self::pure_creator(&delegator, &defs, deposit).unwrap_or(delegator);
			Self::add_expected(who, kind, deposit);
			records += 1;
		}

		// Proxy announcements are not migrated: the deposit, reserved on the announcer, is
		// refunded.
		for (announcer, (_, deposit)) in pallet_proxy::Announcements::<T>::iter() {
			Self::add_expected(announcer, DepositKind::Refund, deposit);
			records += 1;
		}

		// Multisig operations cannot complete on a retired chain: the deposit, reserved on the
		// depositor, is refunded.
		for (_, _, op) in pallet_multisig::Multisigs::<T>::iter() {
			Self::add_expected(op.depositor, DepositKind::Refund, op.deposit);
			records += 1;
		}

		records
	}

	/// Release every preimage deposit. Nothing dispatches a preimage on the Relay Chain after the
	/// migration, so the deposit has no purpose. An unrequested blob is removed; a requested one
	/// stays for whatever requested it, with its ticket dropped. Unbounded, see [`Self::init`].
	fn release_preimage_deposits() {
		let with_deposit: Vec<_> = pallet_preimage::RequestStatusFor::<T>::iter()
			.filter(|(_, status)| match status {
				// Normally the owner gets this deposit back by unnoting the preimage, or when a
				// request for it has been served and the request is dropped.
				pallet_preimage::RequestStatus::Unrequested { .. } => true,
				pallet_preimage::RequestStatus::Requested { maybe_ticket, .. } =>
					maybe_ticket.is_some(),
			})
			.map(|(hash, _)| hash)
			.collect();
		let count = with_deposit.len();
		for hash in with_deposit {
			// the pallet's own path, minus the origin and ownership checks
			<pallet_preimage::Pallet<T> as StorePreimage>::unnote(&hash);
		}
		if count > 0 {
			log::info!(target: LOG_TARGET, "Released {count} preimage deposit(s)");
		}
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
		defs: &[ProxyDefinitionOf<T>],
		deposit: u128,
	) -> Option<T::AccountId> {
		let account = frame_system::Account::<T>::get(delegator);
		if !account.nonce.is_zero() {
			return None;
		}
		let any = Self::any_delegate(defs)?;
		let needed = ExpectedReserves::<T>::get(delegator).total().saturating_add(deposit);
		(account.data.reserved < needed).then(|| any.clone())
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
			if !Self::deferred(&who, &info, exceptions) {
				match with_storage_layer(|| Self::withdraw_account(&who, &info)) {
					Ok(Withdrawal { to_ct, to_ah }) => {
						out.to_ct.extend(to_ct);
						out.to_ah.extend(to_ah);
					},
					Err(e) =>
						log::error!(target: LOG_TARGET, "Account {who:?} not withdrawn: {e:?}"),
				}
			}
			out.last_key = Some(who);
		}
		out
	}

	/// Whether a later stage handles `who`: a module account (Sweep empties the pots), one below
	/// the existential deposit (Sweep Dust reaps them), or one of the `exceptions` (the manager,
	/// reaped at Cool Off).
	fn deferred(who: &T::AccountId, info: &AccountInfoFor<T>, exceptions: &[T::AccountId]) -> bool {
		Self::is_pallet_account(who) ||
			info.data.total() < NativeCurrency::<T>::minimum_balance() ||
			exceptions.contains(who)
	}

	/// Withdraw `who` whole and say where its balance goes. One step per case of `accounts.md`.
	fn withdraw_account(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
	) -> Result<Withdrawal, DispatchError> {
		// Release holds, locks and reserves first, so the whole balance can be burned. Holds
		// count as reserved, locks and freezes as frozen.
		let mut holds = BoundedVec::new();
		if !info.data.reserved.is_zero() {
			Self::release_holds(who)?;
			holds = Self::withdraw_reserved(who)?;
		}
		if !info.data.frozen.is_zero() {
			Self::release_locks(who)?;
		}

		// Burn the whole balance here. A release may have dropped a consumer reference, so read
		// the account again.
		let released = !info.data.reserved.is_zero() || !info.data.frozen.is_zero();
		let account = if released { frame_system::Account::<T>::get(who) } else { info.clone() };
		let total = account.data.free;
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

	/// Split what was burned by destination: the holds and a working buffer of the liquid
	/// balance go to the Coretime chain, the rest to Asset Hub. A pure proxy's whole balance
	/// goes to the Coretime chain.
	fn route(
		who: &T::AccountId,
		pure: bool,
		total: u128,
		holds: BoundedVec<PortableHold<u128>, MaxPortableHolds>,
	) -> Withdrawal {
		// What the holds do not take is liquid.
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
		// A remainder below the existential deposit follows the holds instead of being teleported.
		if ah_free < NativeCurrency::<T>::minimum_balance() {
			ct_free = ct_free.saturating_add(ah_free);
			ah_free = 0;
		}

		// TODO(ahm-v2): Asset Hub's own sovereign (`para` + 1000) lands on `sibl` + 1000 on Asset
		// Hub, which nothing there controls. Send it to the sweep beneficiary instead. It holds
		// DOT here from HRMP channel deposits made before system channels became deposit-free,
		// and from whatever was sent to it since.
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
	/// Returns the holds that continue on the Coretime chain, including what no record
	/// explains; refunded deposits stay free.
	fn withdraw_reserved(
		who: &T::AccountId,
	) -> Result<BoundedVec<PortableHold<u128>, MaxPortableHolds>, DispatchError> {
		// read again: the holds released before this were counted in it
		let reserved = frame_system::Account::<T>::get(who).data.reserved;
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
		// What no record explains travels too, parked on the Coretime chain under its own reason.
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

	/// Release every named hold on `who`. Preimage deposits are released through their pallet
	/// before the first withdrawal; what is left here is a leftover from a pallet that left in
	/// AHM v1, and it leaves as free balance.
	fn release_holds(who: &T::AccountId) -> DispatchResult {
		for hold in pallet_balances::Holds::<T>::get(who) {
			NativeCurrency::<T>::release(&hold.id, who, hold.amount, Precision::Exact)?;
		}
		Ok(())
	}

	/// Release every lock and freeze on `who`. The pallets that set them left the Relay Chain in
	/// AHM v1, so what is left is a leftover, and its balance leaves as free.
	fn release_locks(who: &T::AccountId) -> DispatchResult {
		for lock in pallet_balances::Locks::<T>::get(who) {
			NativeCurrency::<T>::remove_lock(lock.id, who);
		}
		for freeze in pallet_balances::Freezes::<T>::get(who) {
			NativeCurrency::<T>::thaw(&freeze.id, who)?;
		}
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
			Self::any_delegate(&pallet_proxy::Proxies::<T>::get(who).0).is_some()
	}

	/// Whether `who` is a pallet (module) account, `modl` + pallet id.
	pub fn is_pallet_account(who: &T::AccountId) -> bool {
		let bytes: &[u8] = who.as_ref();
		bytes.starts_with(&<PalletId as TypeId>::TYPE_ID)
	}
}
