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

//! Proxy stage: migrates proxy delegations to the Coretime chain.
//!
//! - Every definition whose permission the Coretime chain represents (the runtime's
//!   `TryInto<PortableProxyType>`: `Any`, `NonTransfer`, `CancelProxy`, `ParaRegistration`) is sent
//!   there and recreated, whatever the delegator's balance. Keyless (pure) delegators can only ever
//!   act through the recreated definitions — the accounts stage routes their whole balance there
//!   for the same reason; for keyed delegators the recreation is a harmless convenience.
//! - Definitions it does not represent (staking, governance, …) stay here. Deposits do not travel:
//!   the accounts stage moved them, so the recorded deposit is clamped to what is still reserved
//!   and no entry claims money that is gone.
//! - An entry with nothing left to keep, or whose delegator has no account, is removed.
//!
//! Announcements are not migrated. A record whose announcer's account is gone is removed; the
//! others get their recorded deposit clamped the same way.
//!
//! Shipping the batches over XCM and driving the stage from `on_initialize` is the stage
//! machine's job: [`ProxyMigrator::drain_announcements`] runs once before the first block and
//! [`ProxyMigrator::migrate_many`] once per block, inside a storage transaction the caller owns.

extern crate alloc;

use crate::Config;
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::BoundedVec;
use migrator_types::{
	translate_destination, with_rollback, PortableProxy, PortableProxyDelegate, PortableProxyType,
};
use sp_runtime::{traits::UniqueSaturatedInto, AccountId32};

const LOG_TARGET: &str = "runtime::rc2-migrator";

/// Maximum number of proxy entries processed per relay-chain block.
///
/// Bounds the unbenchmarked work of one `on_initialize` here and of the resulting
/// `receive_proxies` calls on the Coretime chain.
pub const MAX_PROXIES_PER_BLOCK: u32 = 100;

type ProxyDefinitionOf<T> = pallet_proxy::ProxyDefinition<
	<T as frame_system::Config>::AccountId,
	<T as pallet_proxy::Config>::ProxyType,
	pallet_proxy::BlockNumberFor<T>,
>;

/// Why a proxy entry could not be migrated. The caller rolls the entry back and skips it.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// More definitions travel than the wire format holds.
	TooManyDelegates,
}

/// Everything one block of the stage sends, ready to be shipped.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BlockProxies {
	/// Proxy sets to recreate on the Coretime chain.
	pub proxies: Vec<PortableProxy<AccountId32>>,
	/// Where the next block continues from; `None` once the map is exhausted.
	pub last_key: Option<AccountId32>,
}

pub struct ProxyMigrator<T>(PhantomData<T>);

impl<T: Config> ProxyMigrator<T> {
	/// Drop the announcement records of announcers whose accounts migrated away and clamp the
	/// recorded deposit of the others to what is still reserved. Returns the number of records
	/// dropped.
	///
	/// One-shot; the map is small. The caller wraps this in a storage transaction.
	pub fn drain_announcements() -> u32 {
		let mut dropped = 0u32;
		let records: Vec<_> = pallet_proxy::Announcements::<T>::iter().collect();
		for (announcer, (announcements, deposit)) in records {
			match frame_system::Account::<T>::try_get(&announcer) {
				Ok(account) => {
					let backed = deposit.min(account.data.reserved);
					if backed != deposit {
						pallet_proxy::Announcements::<T>::insert(
							&announcer,
							(announcements, backed),
						);
					}
				},
				Err(()) => {
					pallet_proxy::Announcements::<T>::remove(&announcer);
					dropped += 1;
				},
			}
		}
		log::info!(target: LOG_TARGET, "Dropped {dropped} announcement records");
		dropped
	}

	/// Migrate proxy entries until the per-block limit is reached.
	///
	/// The caller wraps this in a storage transaction and ships the result. Each entry is
	/// migrated in a transaction of its own, so one that cannot be is skipped whole.
	pub fn migrate_many(last_key: Option<T::AccountId>) -> BlockProxies {
		// Get iterator starting after last processed key
		let mut iter = match &last_key {
			Some(last_key) => pallet_proxy::Proxies::<T>::iter_from(
				pallet_proxy::Proxies::<T>::hashed_key_for(last_key),
			),
			None => pallet_proxy::Proxies::<T>::iter(),
		};

		let mut out = BlockProxies::default();
		let mut processed = 0u32;
		out.last_key = loop {
			let Some((who, (defs, deposit))) = iter.next() else { break None };
			processed += 1;

			match with_rollback(|| Self::migrate_single(&who, defs.into_inner(), deposit)) {
				Ok(Some(proxy)) => out.proxies.push(proxy),
				Ok(None) => (),
				Err(e) => {
					log::warn!(target: LOG_TARGET, "Skipping proxy entry of {who:?}: {e:?}");
				},
			}

			if processed >= MAX_PROXIES_PER_BLOCK {
				break Some(who);
			}
		};
		out
	}

	/// Migrate a single proxy entry. `Ok(None)` means none of its definitions is portable.
	pub fn migrate_single(
		who: &T::AccountId,
		defs: Vec<ProxyDefinitionOf<T>>,
		deposit: u128,
	) -> Result<Option<PortableProxy<AccountId32>>, Error> {
		// Convert each definition once; `Ok` means the Coretime chain represents the permission
		// and the definition travels, `Err` means it stays here.
		let (travel, stay): (Vec<_>, Vec<_>) = defs
			.into_iter()
			.map(|def| {
				let portable: Option<PortableProxyType> = def.proxy_type.clone().try_into().ok();
				(def, portable)
			})
			.partition(|(_, portable)| portable.is_some());

		let proxy = if travel.is_empty() {
			None
		} else {
			// Account ids on the wire are always the destination's address for them — see
			// `migrator_types::translate_destination`.
			let delegates: Vec<_> = travel
				.into_iter()
				.map(|(def, portable)| PortableProxyDelegate {
					delegate: translate_destination(&def.delegate),
					proxy_type: portable.expect("partition kept only converted definitions; qed"),
					delay: def.delay.unique_saturated_into(),
				})
				.collect();
			Some(PortableProxy {
				delegator: translate_destination(who),
				delegates: delegates.try_into().map_err(|_| Error::TooManyDelegates)?,
			})
		};

		// Whatever stays keeps a deposit record no larger than what is still reserved.
		let stay: Vec<_> = stay.into_iter().map(|(def, _)| def).collect();
		match frame_system::Account::<T>::try_get(who) {
			Ok(account) if !stay.is_empty() => {
				let backed = deposit.min(account.data.reserved);
				let stay: BoundedVec<_, <T as pallet_proxy::Config>::MaxProxies> =
					stay.try_into().expect("subset of a bounded vec; qed");
				pallet_proxy::Proxies::<T>::insert(who, (stay, backed));
			},
			_ => pallet_proxy::Proxies::<T>::remove(who),
		}

		// TODO(ahm-v2): definitions that do not travel are dropped with the entry of a delegator
		// whose account is gone. Whether to recreate them on Asset Hub instead, as v1 did, is
		// undecided.
		Ok(proxy)
	}
}
