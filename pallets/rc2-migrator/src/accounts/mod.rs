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

//! Accounts stage: withdraws account balances on the relay chain and routes the pieces to their
//! destinations.
//!
//! Per account, the balance splits by destination:
//! - reserved balance, up to what the registrar and HRMP pallets record as this account's deposits,
//!   goes to the **Coretime chain** as a hold (re-attributed by the later stages);
//! - a small working buffer of free balance (`Config::CtFreeBuffer`) follows the deposit to
//!   Coretime;
//! - all remaining free balance is **teleported to Asset Hub**, where the owners' phase-1 funds
//!   already live.
//!
//! Exception: a delegator that never signed (`nonce == 0`) and grants an `Any` proxy is treated
//! as a keyless pure proxy — its delegate keeps full control only where the definitions are
//! recreated (the Coretime chain, by the proxy stage), so ALL of its balance goes there instead
//! of Asset Hub.
//!
//! Nothing stays behind: reserve that no pallet's deposit records account for (a known on-chain
//! anomaly) also travels to the Coretime chain, under its own hold reason, and stays parked there
//! for investigation. Accounts that a consumer reference forbids reaping (session key-holders)
//! are drained to zero-balance shells — the record survives, the money moves.
//!
//! Para sovereign accounts are included: their child-sovereign id (`para…`) is translated to the
//! sibling id (`sibl…`) that represents the same para on a parachain.
//!
//! This module withdraws and classifies. Shipping the pieces over XCM, and driving the stage from
//! `on_initialize`, is the stage machine's job: [`AccountsMigrator::init`] runs once before the
//! first block of withdrawals and [`AccountsMigrator::migrate_many`] runs once per block, inside
//! a storage transaction the caller owns, and hands back what that block burned.

extern crate alloc;

#[cfg(test)]
mod tests;

use crate::{Config, Event, ExpectedReserves, MigratedBalances, Pallet, RcMigratedBalance};
use alloc::vec::Vec;
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_support::{
	defensive, defensive_assert,
	storage::with_storage_layer,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		Get, ReservableCurrency, StorePreimage,
	},
	BoundedVec,
};
use migrator_types::{PortableAccount, PortableHold, PortableHoldReason, PortableProxyType};
use polkadot_runtime_common::paras_registrar;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Zero},
	AccountId32, DispatchError,
};

const LOG_TARGET: &str = "runtime::rc2-migrator";

/// Maximum number of accounts processed per relay-chain block.
///
/// Bounds the unbenchmarked work of one `on_initialize` here and of the resulting
/// `receive_accounts` calls on the Coretime chain.
pub const MAX_ACCOUNTS_PER_BLOCK: u32 = 300;

/// Account-id prefixes that are never migrated: pallet (module) accounts — leftover pots among
/// them are handled by the `Sweep` stage. Child sovereigns (`para`) ARE migrated, translated to
/// their sibling id; sibling-format sovereigns (an anomaly on a relay chain, someone teleported
/// to the wrong address) migrate untranslated — on Asset Hub the same bytes ARE that para's
/// sovereign, so the para regains control of the funds.
const UNMIGRATED_PREFIXES: [&[u8]; 1] = [b"modl"];

type NativeCurrency<T> = pallet_balances::Pallet<T>;
type AccountInfoFor<T> = frame_system::AccountInfo<
	<T as frame_system::Config>::Nonce,
	pallet_balances::AccountData<u128>,
>;

/// The expected composition of one account's reserved balance. See `ExpectedReserves`.
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
	/// Continues on the Coretime chain as a `RegistrarDeposit` hold: registration deposits
	/// recorded for the account as manager.
	pub registrar: u128,
	/// Continues on the Coretime chain as an `HrmpDeposit` hold: channel and open-request
	/// deposits recorded for the account as (child) para sovereign.
	pub hrmp: u128,
	/// Continues on the Coretime chain as a `ProxyDeposit` hold (resized when the definitions
	/// arrive): proxy deposits of delegators with at least one portable definition.
	pub proxy: u128,
	/// Released and teleported to Asset Hub as free balance: deposits whose purpose ends with
	/// this chain (untranslatable proxy sets, multisig operations, announcements).
	pub refund: u128,
}

/// Why an account could not be withdrawn. The caller rolls the account back and skips it.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// The account balance could not be fully withdrawn.
	FailedToWithdrawAccount,
	/// The migrated/kept balance bookkeeping would overflow.
	BalanceAccounting,
}

impl From<Error> for DispatchError {
	fn from(e: Error) -> Self {
		DispatchError::Other(match e {
			Error::FailedToWithdrawAccount => "FailedToWithdrawAccount",
			Error::BalanceAccounting => "BalanceAccounting",
		})
	}
}

/// Where the pieces of one withdrawn account go.
#[derive(Debug, PartialEq, Eq)]
pub struct Withdrawal {
	/// Deposit hold plus working buffer, minted on the Coretime chain.
	pub ct: Option<PortableAccount<AccountId32, u128>>,
	/// Free balance teleported to Asset Hub: (beneficiary, amount).
	pub ah: Option<(AccountId32, u128)>,
}

/// Everything one block of withdrawals burned, ready to be shipped.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BlockWithdrawals {
	/// Accounts to mint on the Coretime chain, holds included.
	pub ct: Vec<PortableAccount<AccountId32, u128>>,
	/// Free balance to teleport to Asset Hub: (beneficiary, amount).
	pub ah: Vec<(AccountId32, u128)>,
	/// Where the next block continues from; `None` once the account space is exhausted.
	pub last_key: Option<AccountId32>,
}

pub struct AccountsMigrator<T>(PhantomData<T>);

impl<T: Config> AccountsMigrator<T> {
	/// One-time preparation before the first block of withdrawals: releases every preimage
	/// deposit, seeds the conservation ledger with the current total issuance and indexes the
	/// expected reserves. Returns the number of deposit records indexed.
	pub fn init() -> u32 {
		// Before anything is measured: drop every preimage deposit, so accounts that hold one
		// are not skipped by `can_migrate`.
		Self::release_preimage_deposits();
		RcMigratedBalance::<T>::put(MigratedBalances {
			kept: NativeCurrency::<T>::total_issuance(),
			..Default::default()
		});
		let indexed = Self::build_expected_reserves();
		log::info!(target: LOG_TARGET, "Indexed expected reserves from {indexed} records");
		indexed
	}

	/// Index every account's expected reserves from the owning pallets' records:
	/// - Coretime-bound ([`ExpectedReserve::ct`]): registrar deposits per manager, HRMP channel and
	///   request deposits per (child) para sovereign;
	/// - proxy deposits ([`ExpectedReserve::proxy`]): per delegator with at least one portable
	///   definition — they travel under their own hold reason and are resized when the definitions
	///   arrive;
	/// - refunded ([`ExpectedReserve::refund`]): deposits whose purpose does not continue.
	///
	/// Returns the number of records indexed.
	pub fn build_expected_reserves() -> u32 {
		let mut records = 0u32;
		// One record per account; `slot` picks which expectation the amount accrues to.
		let add = |who: T::AccountId, amount: u128, slot: fn(&mut ExpectedReserve) -> &mut u128| {
			if !amount.is_zero() {
				ExpectedReserves::<T>::mutate(&who, |e| {
					let v = slot(e);
					*v = v.saturating_add(amount);
				});
			}
		};
		let add_registrar = |who, amount| add(who, amount, |e| &mut e.registrar);
		let add_hrmp = |who, amount| add(who, amount, |e| &mut e.hrmp);
		let add_proxy = |who, amount| add(who, amount, |e| &mut e.proxy);
		let add_refund = |who, amount| add(who, amount, |e| &mut e.refund);

		for (_, info) in paras_registrar::Paras::<T>::iter() {
			add_registrar(info.manager, info.deposit);
			records += 1;
		}
		for (id, channel) in runtime_parachains::hrmp::HrmpChannels::<T>::iter() {
			add_hrmp(id.sender.into_account_truncating(), channel.sender_deposit);
			add_hrmp(id.recipient.into_account_truncating(), channel.recipient_deposit);
			records += 1;
		}
		// Pending open-channel requests migrate to the Coretime chain with their deposits, so
		// the sender sovereigns' request deposits are Coretime-bound like channel deposits.
		for (id, request) in runtime_parachains::hrmp::HrmpOpenChannelRequests::<T>::iter() {
			add_hrmp(id.sender.into_account_truncating(), request.sender_deposit);
			records += 1;
		}
		for (who, (defs, deposit)) in pallet_proxy::Proxies::<T>::iter() {
			let travels = defs
				.iter()
				.any(|def| TryInto::<PortableProxyType>::try_into(def.proxy_type.clone()).is_ok());
			if travels {
				add_proxy(who, deposit);
			} else {
				add_refund(who, deposit);
			}
			records += 1;
		}
		// Proxy announcement deposits, reserved on the announcer (the delegate). Announcements
		// are not migrated — their purpose ends with this chain — so the deposit is refunded.
		for (announcer, (_, deposit)) in pallet_proxy::Announcements::<T>::iter() {
			add_refund(announcer, deposit);
			records += 1;
		}
		// Multisig operation deposits: the one deposit source whose calls stay open until the
		// migration starts, so entries can still appear. The operation itself cannot complete on
		// a retired chain — the deposit is refunded to the depositor.
		for (_, _, op) in pallet_multisig::Multisigs::<T>::iter() {
			add_refund(op.depositor, op.deposit);
			records += 1;
		}
		records
	}

	/// Withdraw accounts until the per-block limit is reached.
	///
	/// `manager` is the one account that stays funded here until the migration ends: it pays for
	/// the calls that drive the migration.
	///
	/// The caller wraps this in a storage transaction and ships the result; an `Err` rolls back
	/// the whole block's withdrawals. Each account is withdrawn in a transaction of its own, so
	/// one that cannot be withdrawn cleanly is skipped whole, never half-withdrawn.
	pub fn migrate_many(
		last_key: Option<T::AccountId>,
		manager: Option<&T::AccountId>,
	) -> Result<BlockWithdrawals, Error> {
		let mut iter = match &last_key {
			Some(last_key) => frame_system::Account::<T>::iter_from(
				frame_system::Account::<T>::hashed_key_for(last_key),
			),
			None => frame_system::Account::<T>::iter(),
		};

		let mut out = BlockWithdrawals::default();
		// Ledger deltas of this block's successful withdrawals; applied in one write at the end
		// instead of one storage mutation per account.
		let (mut ct_hold_sum, mut ct_free_sum, mut ah_free_sum) = (0u128, 0u128, 0u128);
		let mut processed = 0u32;
		out.last_key = loop {
			let Some((who, info)) = iter.next() else { break None };
			processed += 1;

			match with_storage_layer(|| {
				Self::withdraw_account(&who, info, manager).map_err(DispatchError::from)
			}) {
				Ok(Some(Withdrawal { ct, ah })) => {
					if let Some(account) = ct {
						ct_hold_sum = ct_hold_sum
							.saturating_add(account.holds.iter().map(|h| h.amount).sum());
						ct_free_sum = ct_free_sum.saturating_add(account.free);
						out.ct.push(account);
					}
					if let Some((who, amount)) = ah {
						ah_free_sum = ah_free_sum.saturating_add(amount);
						out.ah.push((who, amount));
					}
				},
				Ok(None) => (),
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping account {who:?}: {e:?}");
				},
			}

			if processed >= MAX_ACCOUNTS_PER_BLOCK {
				break Some(who);
			}
		};

		let burned = ct_hold_sum.saturating_add(ct_free_sum).saturating_add(ah_free_sum);
		if burned > 0 {
			RcMigratedBalance::<T>::try_mutate(|t| {
				t.kept = t.kept.checked_sub(burned).ok_or(Error::BalanceAccounting)?;
				t.ct_reserved =
					t.ct_reserved.checked_add(ct_hold_sum).ok_or(Error::BalanceAccounting)?;
				t.ct_free = t.ct_free.checked_add(ct_free_sum).ok_or(Error::BalanceAccounting)?;
				t.ah_free = t.ah_free.checked_add(ah_free_sum).ok_or(Error::BalanceAccounting)?;
				Ok::<(), Error>(())
			})?;
		}
		Ok(out)
	}

	/// Withdraw a single account from the relay chain and split it by destination.
	///
	/// `Ok(None)` means the account is deliberately not migrated; `Err` means it should have
	/// migrated but could not be withdrawn cleanly (the caller rolls it back and skips it).
	pub fn withdraw_account(
		who: &T::AccountId,
		info: AccountInfoFor<T>,
		manager: Option<&T::AccountId>,
	) -> Result<Option<Withdrawal>, Error> {
		if !Self::can_migrate(who, &info, manager) {
			return Ok(None);
		}
		let free = info.data.free;
		let reserved = info.data.reserved;

		// What the owning pallets say this account deposited: registrar/HRMP deposits continue on
		// the Coretime chain, proxy deposits travel there under their own reason (or are refunded
		// when no definition travels). Anything beyond is money whose origin is unknown (a true
		// anomaly): it travels too — nothing stays behind — but under its own hold reason, parked
		// at the destination for investigation.
		let expected = if reserved.is_zero() {
			ExpectedReserve::default()
		} else {
			ExpectedReserves::<T>::get(who)
		};

		// A reserve is supposed to be backed by a consumer reference; accounts where it is not
		// have broken refcounts (a known on-chain anomaly). Unreserving them still works, but
		// makes `frame_system` log an anonymous "underflow in reducing consumer" error — name
		// the account here so the anomaly is attributable.
		if !reserved.is_zero() && info.consumers == 0 {
			log::warn!(
				target: LOG_TARGET,
				"Account {who:?} has reserved balance but no consumer reference"
			);
		}

		// Deposits on the relay chain are unnamed reserves (`can_migrate` rejects named holds);
		// release them so the full balance is burnable.
		let not_unreserved = NativeCurrency::<T>::unreserve(who, reserved);
		if !not_unreserved.is_zero() {
			defensive!("Reserved balance was not fully released");
			return Err(Error::FailedToWithdrawAccount);
		}

		// Releasing the reserve drops its consumer reference; anything left means some pallet
		// still references this account (session keys being the known case) and the account
		// record must survive. Its balance must not: drain it to a zero-balance shell — the
		// intended end state for validator key-holders. Every fungible API (`burn_from`,
		// `write_balance`) insists on keeping the ED for an unreapable account, so the account
		// data is written directly — record, refcounts and providers untouched — with total
		// issuance adjusted to match. Zero-balance consumer-referenced accounts already exist on
		// chain, so this creates no new state shape.
		let total = free.saturating_add(reserved);
		if frame_system::Pallet::<T>::consumers(who) != 0 {
			frame_system::Account::<T>::mutate(who, |a| {
				a.data.free = 0;
				a.data.reserved = 0;
			});
			pallet_balances::TotalIssuance::<T>::mutate(|ti| *ti = ti.saturating_sub(total));
			Pallet::<T>::deposit_event(Event::AccountShellDrained {
				who: who.clone(),
				amount: total,
			});
		} else {
			let burned = NativeCurrency::<T>::burn_from(
				who,
				total,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite,
			)
			.map_err(|_| Error::FailedToWithdrawAccount)?;
			defensive_assert!(burned == total, "burned the account's whole balance");
		}

		// The split, in priority order: registrar deposits, HRMP deposits, proxy deposits, then
		// refunds; whatever the expectations do not cover is unattributed. Each line consumes
		// from one remainder, so a live reserve that under-covers the records is attributed to
		// the deposits that continue first.
		let mut remainder = reserved;
		let mut consume = |cap: u128| {
			let taken = remainder.min(cap);
			remainder -= taken;
			taken
		};
		let registrar_hold = consume(expected.registrar);
		let hrmp_hold = consume(expected.hrmp);
		let proxy_hold = consume(expected.proxy);
		let refunded = consume(expected.refund);
		let unattributed = remainder;
		if !refunded.is_zero() {
			Pallet::<T>::deposit_event(Event::DepositRefunded {
				who: who.clone(),
				amount: refunded,
			});
		}
		if !unattributed.is_zero() {
			Pallet::<T>::deposit_event(Event::UnattributedReserve {
				who: who.clone(),
				amount: unattributed,
			});
		}

		// Holds and unattributed reserve → Coretime holds (one per reason); refunded deposits
		// become liquid; working buffer → Coretime free; the rest → Asset Hub free. Free balance
		// below Asset Hub's ED cannot teleport into a fresh account, so such dust follows the
		// deposit to Coretime instead (only deposit holders can be in this situation: everyone
		// else has free >= the relay ED, which exceeds Asset Hub's).
		//
		// Exception: a never-signed delegator granting an `Any` proxy is a keyless pure proxy in
		// all but name. Its delegate keeps full control only on the Coretime chain, where the
		// proxy stage recreates the definitions, so ALL of its liquid balance goes there.
		let held = registrar_hold
			.saturating_add(hrmp_hold)
			.saturating_add(proxy_hold)
			.saturating_add(unattributed);
		let liquid = free.saturating_add(refunded);
		let mut ct_free = if Self::is_pure_like(who, &info) {
			liquid
		} else if held.is_zero() {
			0
		} else {
			liquid.min(T::CtFreeBuffer::get())
		};
		let mut ah_free = liquid.saturating_sub(ct_free);
		if !ah_free.is_zero() && ah_free < T::AhExistentialDeposit::get() && !held.is_zero() {
			ct_free = ct_free.saturating_add(ah_free);
			ah_free = 0;
		}

		let dest = migrator_types::translate_destination(who);
		let ct = if held.is_zero() && ct_free.is_zero() {
			None
		} else {
			let mut holds = BoundedVec::default();
			for (reason, amount) in [
				(PortableHoldReason::RegistrarDeposit, registrar_hold),
				(PortableHoldReason::HrmpDeposit, hrmp_hold),
				(PortableHoldReason::ProxyDeposit, proxy_hold),
				(PortableHoldReason::UnattributedReserve, unattributed),
			] {
				if !amount.is_zero() {
					holds
						.try_push(PortableHold { reason, amount })
						.map_err(|_| Error::FailedToWithdrawAccount)?;
				}
			}
			Some(PortableAccount { who: dest.clone(), free: ct_free, holds })
		};
		let ah = (!ah_free.is_zero()).then_some((dest, ah_free));

		Ok(Some(Withdrawal { ct, ah }))
	}

	/// Release every preimage deposit on this chain.
	///
	/// [`Self::can_migrate`] refuses any account holding a *named* hold, so a single preimage
	/// deposit would strand that account's entire balance. Nothing carries these across: the
	/// relay chain keeps hosting preimages, and the deposit behind one is the last thing tying a
	/// user's money to a chain that is meant to end with none.
	///
	/// Only preimages that actually hold a ticket are touched, and the distinction matters:
	///
	/// - `Unrequested` — nobody wants it. Deleted, deposit returned.
	/// - `Requested` **with** a ticket — the blob is kept and only the ticket dropped, so a
	///   referendum that depends on it still resolves.
	/// - `Requested` **without** a ticket — already deposit-free, and unnoting it would *unrequest*
	///   it (see `do_unnote_preimage`), which can delete a blob live governance is waiting on.
	///   Skipped.
	pub fn release_preimage_deposits() {
		let with_deposit: Vec<_> = pallet_preimage::RequestStatusFor::<T>::iter()
			.filter(|(_, status)| match status {
				pallet_preimage::RequestStatus::Unrequested { .. } => true,
				pallet_preimage::RequestStatus::Requested { maybe_ticket, .. } =>
					maybe_ticket.is_some(),
			})
			.map(|(hash, _)| hash)
			.collect();

		let count = with_deposit.len();
		for hash in with_deposit {
			// The trait's manager path: no origin, no ownership check. Calling the extrinsic as
			// Root would work too, but only on a runtime whose `ManagerOrigin` accepts Root.
			<pallet_preimage::Pallet<T> as StorePreimage>::unnote(&hash);
		}
		if count > 0 {
			log::info!(target: LOG_TARGET, "Released {count} preimage deposit(s)");
		}
	}

	/// A delegator that never signed and grants a portable `Any` proxy: a pure proxy in all but
	/// name, since a pure is always created with `Any` and one that never signed cannot have been
	/// anything else.
	///
	/// Such an account's funds are reachable *only* through its delegate, so they follow the
	/// delegate to the Coretime chain whole — see the split in [`Self::withdraw_account`], and
	/// the exemption in [`Self::can_migrate`].
	pub fn is_pure_like(who: &T::AccountId, info: &AccountInfoFor<T>) -> bool {
		info.nonce.is_zero() &&
			pallet_proxy::Proxies::<T>::get(who).0.iter().any(|def| {
				matches!(def.proxy_type.clone().try_into(), Ok(PortableProxyType::Any))
			})
	}

	/// Whether `who` is an account class the migration never touches (module accounts). The
	/// sweep's dust pass consults this too, so one prefix list rules both.
	pub fn is_unmigrated(who: &T::AccountId) -> bool {
		let bytes: &[u8] = who.as_ref();
		UNMIGRATED_PREFIXES.iter().any(|prefix| bytes.starts_with(prefix))
	}

	/// Whether the account migrates at all. The rejections here are deliberate policy, not
	/// failures.
	pub fn can_migrate(
		who: &T::AccountId,
		info: &AccountInfoFor<T>,
		manager: Option<&T::AccountId>,
	) -> bool {
		// The manager pays for the calls that drive the migration, so it is the one account that
		// stays funded here until the migration ends.
		if manager.is_some_and(|manager| manager == who) {
			log::info!(target: LOG_TARGET, "Keeping the manager account {who:?} on the RC");
			return false;
		}
		if Self::is_unmigrated(who) {
			log::info!(target: LOG_TARGET, "Keeping module account {who:?} on the RC");
			return false;
		}

		let data = &info.data;
		let total = data.free.saturating_add(data.reserved);
		if total < NativeCurrency::<T>::minimum_balance() {
			// Below-ED accounts are left here and reaped: the balance is by definition less than
			// the chain considers worth keeping an account for.
			//
			// This includes pure-like delegators, and that is a deliberate exception to
			// funds-follow-control rather than an oversight. Their definitions still reach the
			// Coretime chain, so the delegate keeps the *para*, but a sub-ED remainder is not
			// worth a migration path of its own — there is nothing usable to lose. Above the ED
			// the rule applies in full: see the `pure_like` split in `withdraw_account`.
			log::info!(target: LOG_TARGET, "Keeping below-ED account {who:?} on the RC");
			return false;
		}

		// Locks, freezes and named holds should no longer occur on the relay chain post-AHM
		// (staking and governance are gone); translating them is not implemented.
		if !data.frozen.is_zero() ||
			!pallet_balances::Locks::<T>::get(who).is_empty() ||
			!pallet_balances::Freezes::<T>::get(who).is_empty() ||
			!pallet_balances::Holds::<T>::get(who).is_empty()
		{
			log::warn!(
				target: LOG_TARGET,
				"Keeping account {who:?} with untranslatable locks/freezes/holds on the RC"
			);
			return false;
		}

		true
	}
}
