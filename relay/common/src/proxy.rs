// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Storage repair migration for legacy `pallet_proxy::Proxies` entries.
//!
//! Some historic entries are undecodable under the current value type because of two
//! breaking changes: the added `delay` field (which grew each proxy and made the value
//! `(BoundedVec<ProxyDefinition>, Balance)`), and removed `ProxyType` variants (e.g. the
//! old `SudoBalances` discriminant `4`).
//!
//! This migration decodes such entries leniently, reading each proxy type as its raw
//! discriminant byte, and repairs them: valid proxies are kept with `delay = 0`, proxies
//! of removed types are dropped, and if none survive the entry is removed and its deposit
//! unreserved. Entries that already decode are left untouched, so this is an idempotent
//! no-op on chains without legacy entries.

use codec::{Compact, Decode, Input};
use frame_support::{
	traits::{Currency, Get, OnRuntimeUpgrade, ReservableCurrency},
	weights::Weight,
	BoundedVec,
};
use pallet_proxy::{BlockNumberFor, ProxyDefinition};
use sp_runtime::traits::Zero;

extern crate alloc;
use alloc::vec::Vec;
use core::marker::PhantomData;

type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type ProxyTypeOf<T> = <T as pallet_proxy::Config>::ProxyType;
type BalanceOf<T> = <<T as pallet_proxy::Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
type ProxyDefinitionOf<T> = ProxyDefinition<AccountIdOf<T>, ProxyTypeOf<T>, BlockNumberFor<T>>;

/// The current value type of `pallet_proxy::Proxies`.
type CurrentProxiesValue<T> =
	(BoundedVec<ProxyDefinitionOf<T>, <T as pallet_proxy::Config>::MaxProxies>, BalanceOf<T>);

const LOG_TARGET: &str = "runtime::migrations::proxy";

/// Sanity bound on the legacy proxy count to avoid huge allocations from corrupt data.
/// Far above any historic `MaxProxies` value on the relay chains.
const MAX_LEGACY_PROXIES: u32 = 1024;

/// A legacy `Proxies` value decoded leniently.
struct LegacyEntry<T: pallet_proxy::Config> {
	/// Proxies whose type is still valid under the current runtime (removed/unknown types
	/// are dropped). Each carries `delay = 0`.
	proxies: Vec<ProxyDefinitionOf<T>>,
	/// The reserved deposit recorded in the legacy value.
	deposit: BalanceOf<T>,
}

/// Leniently decode a legacy `(Vec<(AccountId, ProxyType)>, Balance)` value, reading each
/// `ProxyType` as its one-byte discriminant so removed variants don't abort the decode.
/// Returns `None` if the value isn't fully consumed (assumption wrong or data corrupt),
/// in which case the caller must leave the entry untouched.
fn decode_legacy<T: pallet_proxy::Config>(raw: &[u8]) -> Option<LegacyEntry<T>> {
	let mut input = raw;

	let count = Compact::<u32>::decode(&mut input).ok()?.0;
	if count > MAX_LEGACY_PROXIES {
		return None;
	}

	let mut proxies = Vec::new();
	for _ in 0..count {
		let delegate = AccountIdOf::<T>::decode(&mut input).ok()?;
		// Read the proxy-type as a raw discriminant byte so a removed/unknown variant
		// doesn't abort the decode; such a proxy is dropped.
		let mut discriminant = [0u8; 1];
		input.read(&mut discriminant).ok()?;
		if let Ok(proxy_type) = ProxyTypeOf::<T>::decode(&mut &discriminant[..]) {
			proxies.push(ProxyDefinition { delegate, proxy_type, delay: Zero::zero() });
		}
	}

	let deposit = BalanceOf::<T>::decode(&mut input).ok()?;

	// The whole value must be consumed; otherwise our one-byte assumption was wrong
	// (or the data is corrupt) and we must not touch the entry.
	if !input.is_empty() {
		return None;
	}

	Some(LegacyEntry { proxies, deposit })
}

/// Repairs any legacy-encoded `pallet_proxy::Proxies` entry into the current format.
pub struct MigrateLegacyProxies<T>(PhantomData<T>);

impl<T: pallet_proxy::Config> MigrateLegacyProxies<T> {
	/// Reads the raw, undecoded bytes stored for `who` in `pallet_proxy::Proxies`.
	fn raw_value(who: &AccountIdOf<T>) -> Option<Vec<u8>> {
		let key = pallet_proxy::Proxies::<T>::hashed_key_for(who);
		frame_support::storage::unhashed::get_raw(&key)
	}

	/// `true` if the raw value already decodes under the current type.
	fn decodes_under_current(raw: &[u8]) -> bool {
		CurrentProxiesValue::<T>::decode(&mut &raw[..]).is_ok()
	}
}

impl<T: pallet_proxy::Config> OnRuntimeUpgrade for MigrateLegacyProxies<T> {
	fn on_runtime_upgrade() -> Weight {
		let db_weight = <T as frame_system::Config>::DbWeight::get();
		// Reading the set of keys to iterate over is itself a read.
		let mut weight = db_weight.reads(1);
		let (mut scanned, mut repaired) = (0u64, 0u64);

		// `iter_keys` decodes only the storage keys, never the values, so it also yields
		// the broken entries; rewriting/removing values doesn't invalidate key iteration.
		for who in pallet_proxy::Proxies::<T>::iter_keys() {
			scanned += 1;
			weight = weight.saturating_add(db_weight.reads(1));

			let raw = match Self::raw_value(&who) {
				Some(raw) => raw,
				None => continue,
			};

			// Leave already-valid entries untouched.
			if Self::decodes_under_current(&raw) {
				continue;
			}

			let entry = match decode_legacy::<T>(&raw) {
				Some(entry) => entry,
				None => {
					// Decodable under neither layout: nothing safe to do, leave untouched.
					log::error!(
						target: LOG_TARGET,
						"Proxy entry decodable under neither current nor legacy layout; left untouched",
					);
					continue;
				},
			};

			if entry.proxies.is_empty() {
				// No proxy survived: drop the entry and refund the deposit.
				pallet_proxy::Proxies::<T>::remove(&who);
				T::Currency::unreserve(&who, entry.deposit);
				weight = weight.saturating_add(db_weight.reads_writes(1, 2));
			} else {
				let bounded = match BoundedVec::<_, T::MaxProxies>::try_from(entry.proxies) {
					Ok(bounded) => bounded,
					Err(_) => {
						// Unreachable: legacy entries used the same `MaxProxies` bound. Bail
						// rather than lose proxies by truncating.
						log::error!(
							target: LOG_TARGET,
							"Legacy proxy entry exceeds MaxProxies; left untouched",
						);
						continue;
					},
				};
				pallet_proxy::Proxies::<T>::insert(&who, (bounded, entry.deposit));
				weight = weight.saturating_add(db_weight.writes(1));
			}

			repaired += 1;
		}

		log::info!(
			target: LOG_TARGET,
			"MigrateLegacyProxies: scanned {scanned} entries, repaired {repaired}",
		);

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;
		let (mut undecodable, mut fixable) = (0u32, 0u32);
		for who in pallet_proxy::Proxies::<T>::iter_keys() {
			let raw = match Self::raw_value(&who) {
				Some(raw) => raw,
				None => continue,
			};
			if !Self::decodes_under_current(&raw) {
				undecodable += 1;
				if decode_legacy::<T>(&raw).is_some() {
					fixable += 1;
				}
			}
		}
		let unfixable = undecodable.saturating_sub(fixable);
		log::info!(
			target: LOG_TARGET,
			"pre_upgrade: {undecodable} undecodable proxy entries ({fixable} fixable, {unfixable} unfixable)",
		);
		// Carry forward the count we cannot repair; `post_upgrade` requires exactly that
		// many entries to remain undecodable (we repair all and only the fixable ones).
		Ok(unfixable.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let unfixable_before: u32 =
			Decode::decode(&mut &state[..]).map_err(|_| "Failed to decode pre_upgrade state")?;

		let mut undecodable_after = 0u32;
		for who in pallet_proxy::Proxies::<T>::iter_keys() {
			if let Some(raw) = Self::raw_value(&who) {
				if !Self::decodes_under_current(&raw) {
					undecodable_after += 1;
				}
			}
		}

		frame_support::ensure!(
			undecodable_after == unfixable_before,
			"MigrateLegacyProxies left a repairable proxy entry undecodable",
		);
		log::info!(
			target: LOG_TARGET,
			"post_upgrade: {undecodable_after} undecodable proxy entries remain (all unrepairable)",
		);
		Ok(())
	}
}
