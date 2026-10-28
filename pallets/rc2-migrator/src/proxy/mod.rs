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
	Config, Limits, LOG_TARGET,
};
use alloc::vec::Vec;
use core::marker::PhantomData;
use migrator_types::{translate_destination, PortableProxy, PortableProxyDelegate};
use sp_runtime::{traits::UniqueSaturatedInto, AccountId32};

/// Why a delegator's definitions could not be sent. The entry is skipped.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// More portable definitions than the wire format holds.
	TooManyDelegates,
}

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
	// TODO(ahm-v2): open question for review: the entries stay as they are. Each still records
	// the deposit the accounts stage released, and nothing can act on it since `Proxy` is closed
	// after the migration. Decide whether to leave them, remove them, or zero the recorded
	// deposit.
	pub fn migrate_many(last_key: Option<T::AccountId>) -> BlockProxies {
		let mut iter = match last_key {
			Some(last_key) => pallet_proxy::Proxies::<T>::iter_from(
				pallet_proxy::Proxies::<T>::hashed_key_for(&last_key),
			),
			None => pallet_proxy::Proxies::<T>::iter(),
		};

		let mut out = BlockProxies::default();
		for _ in 0..Limits::<T>::get().items_per_block() {
			let Some((who, (defs, _deposit))) = iter.next() else {
				out.last_key = None;
				return out;
			};
			match Self::migrate_single(&who, &defs) {
				Ok(Some(proxy)) => out.proxies.push(proxy),
				Ok(None) => (),
				Err(error) => {
					log::error!(target: LOG_TARGET, "Proxies of {who:?} not sent: {error:?}");
				},
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
	) -> Result<Option<PortableProxy<AccountId32>>, Error> {
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
			return Ok(None);
		}
		Ok(Some(PortableProxy {
			delegator: translate_destination(who),
			delegates: delegates.try_into().map_err(|_| Error::TooManyDelegates)?,
		}))
	}
}
