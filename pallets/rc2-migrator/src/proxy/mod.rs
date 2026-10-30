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
	Config,
};
use alloc::vec::Vec;
use core::marker::PhantomData;
use frame_support::{traits::DefensiveTruncateFrom, BoundedVec};
use migrator_types::{translate_destination, PortableProxy, PortableProxyDelegate};
use sp_runtime::{traits::UniqueSaturatedInto, AccountId32};

/// What one block of the stage removed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BlockProxies {
	/// Sets to recreate on the Coretime chain.
	pub proxies: Vec<PortableProxy<AccountId32>>,
	/// Proxy and announcement entries removed.
	pub removed: u32,
}

pub struct ProxyMigrator<T>(PhantomData<T>);

impl<T: Config> ProxyMigrator<T> {
	/// Remove up to `limit` entries, the proxy map from the front and, once that is empty, the
	/// announcements, and return what travels. An entry goes whether or not anything of it
	/// does: the relay chain's proxy calls are closed for good, and its deposit left with the
	/// accounts stage.
	pub fn migrate_many(limit: u32) -> BlockProxies {
		let mut out = BlockProxies::default();
		for (who, (defs, _deposit)) in pallet_proxy::Proxies::<T>::drain().take(limit as usize) {
			if let Some(proxy) = Self::migrate_single(&who, &defs) {
				out.proxies.push(proxy);
			}
			out.removed += 1;
		}
		let budget = (limit - out.removed) as usize;
		out.removed +=
			pallet_proxy::Announcements::<T>::iter_keys().drain().take(budget).count() as u32;
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
