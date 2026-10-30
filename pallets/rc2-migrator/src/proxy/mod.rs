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

#![doc = include_str!("proxy.md")]

#[cfg(test)]
mod tests;

use crate::{
	accounts::{AccountsMigrator, ProxyDefinitionOf},
	Config, Limits,
};
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::{traits::DefensiveTruncateFrom, BoundedVec};
use migrator_types::{translate_destination, PortableProxy, PortableProxyDelegate};
use sp_runtime::{traits::UniqueSaturatedInto, AccountId32};

/// What one block of the stage read.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BlockProxies {
	/// Sets to recreate on the Coretime chain.
	pub proxies: Vec<PortableProxy<AccountId32>>,
	/// Where the next block continues from; `None` once every entry has been visited.
	pub last_key: Option<AccountId32>,
}

pub struct ProxyMigrator<T>(PhantomData<T>);

impl<T: Config> ProxyMigrator<T> {
	/// Read the proxy entries after `last_key`, up to the block's [`Limits`], and return what
	/// travels. Writes nothing.
	// TODO(ahm-v2): open for review: leave the entries, remove them, or zero the recorded
	// deposit. See proxy.md, "What stays".
	pub fn migrate_many(last_key: Option<T::AccountId>) -> BlockProxies {
		let mut iter = match last_key {
			Some(last_key) => pallet_proxy::Proxies::<T>::iter_from_key(last_key),
			None => pallet_proxy::Proxies::<T>::iter(),
		};

		let mut out = BlockProxies::default();
		for _ in 0..Limits::<T>::get().items_per_block() {
			let Some((who, (defs, _deposit))) = iter.next() else {
				out.last_key = None;
				return out;
			};
			if let Some(proxy) = Self::migrate_single(&who, &defs) {
				out.proxies.push(proxy);
			}
			out.last_key = Some(who);
		}
		out
	}

	/// The definitions of `who` that have a meaning on the Coretime chain, as the wire carries
	/// them. `None` when there are none.
	pub fn migrate_single(
		who: &T::AccountId,
		defs: &[ProxyDefinitionOf<T>],
	) -> Option<PortableProxy<AccountId32>> {
		let delegates: Vec<_> = defs
			.iter()
			.filter_map(|def| {
				AccountsMigrator::<T>::portable(&def.proxy_type).map(|proxy_type| {
					PortableProxyDelegate {
						delegate: translate_destination(&def.delegate),
						proxy_type,
						delay: def.delay.unique_saturated_into(),
					}
				})
			})
			.collect();
		if delegates.is_empty() {
			return None;
		}
		Some(PortableProxy {
			delegator: translate_destination(who),
			// a relay set fits the wire, which the integrity test asserts
			delegates: BoundedVec::defensive_truncate_from(delegates),
		})
	}
}
