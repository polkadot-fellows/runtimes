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

//! Proxy stage, receiving side: each proxy set the relay chain sent is written into the proxy
//! pallet, merged with what the delegator already has here. The `ProxyDeposit` hold the accounts
//! stage placed is released, the entry is reserved at this chain's rates out of it, and the rest
//! stays free. A set that fails is rolled back and parked in `FailedProxies`; the rest of the
//! batch continues.

#[cfg(test)]
mod tests;

use crate::{Config, CurrencyOf, Event, FailedProxies, HoldReason, Pallet, LOG_TARGET};
use alloc::vec::Vec;
use frame_support::traits::{
	fungible::{InspectHold, MutateHold},
	tokens::Precision,
	Get, ReservableCurrency,
};
use migrator_types::PortableProxy;
use sp_runtime::{traits::Zero, DispatchResult, SaturatedConversion, Saturating};

pub type PortableProxyOf<T> = PortableProxy<<T as frame_system::Config>::AccountId>;

impl<T: Config> Pallet<T> {
	/// Write a batch of proxy sets, each in a transaction of its own. A failed set is parked in
	/// `FailedProxies`.
	pub fn do_receive_proxies(proxies: Vec<PortableProxyOf<T>>) {
		let (count_good, count_bad) = Self::receive_batch(
			proxies,
			Self::do_receive_proxy,
			|()| (),
			|proxy, e| {
				log::error!(target: LOG_TARGET, "Proxies of {:?} not written: {e:?}", proxy.delegator);
				FailedProxies::<T>::insert(proxy.delegator.clone(), proxy);
			},
		);
		Self::deposit_event(Event::ProxiesReceived { count_good, count_bad });
	}

	/// Write one proxy set. Delays arrive in relay chain blocks and are converted to this
	/// chain's. The entry is backed by the migrated deposit and nothing else: a set that arrives
	/// without one, as a pure proxy's does, is written with none, and its owner can
	/// `poke_deposit`.
	fn do_receive_proxy(proxy: &PortableProxyOf<T>) -> DispatchResult {
		let who = &proxy.delegator;

		// Release the migrated deposit whole; the entry is reserved out of it below, at this
		// chain's rates, and the rest stays free.
		let reason: T::RuntimeHoldReason = HoldReason::ProxyDeposit.into();
		let migrated = CurrencyOf::<T>::balance_on_hold(&reason, who);
		if !migrated.is_zero() {
			CurrencyOf::<T>::release(&reason, who, migrated, Precision::Exact)?;
		}
		// the proxy pallet's currency spells its balance differently
		let migrated: u128 = migrated.saturated_into();

		// Written into storage rather than through `add_proxy_delegate`, which refuses a
		// duplicate and a deposit it cannot reserve.
		let ratio = T::RcBlockTimeRatio::get().max(1);
		pallet_proxy::Proxies::<T>::try_mutate(who, |(defs, deposit)| {
			for delegate in &proxy.delegates {
				let def = pallet_proxy::ProxyDefinition {
					delegate: delegate.delegate.clone(),
					proxy_type: delegate.proxy_type.into(),
					delay: (delegate.delay / ratio).into(),
				};
				// Kept sorted, as the pallet keeps it: its own calls find entries by binary search.
				if let Err(i) = defs.binary_search(&def) {
					defs.try_insert(i, def).map_err(|_| pallet_proxy::Error::<T>::TooMany)?;
				}
			}

			// What the proxy pallet charges for the entry on top of what is reserved for the
			// definitions already here, capped at what was migrated: nothing of the delegator's
			// own balance is reserved.
			let required = pallet_proxy::Pallet::<T>::deposit(defs.len() as u32);
			let top_up = required.saturating_sub(*deposit).min(migrated.saturated_into());
			if !top_up.is_zero() {
				<T as pallet_proxy::Config>::Currency::reserve(who, top_up)?;
				*deposit = deposit.saturating_add(top_up);
			}
			Ok(())
		})
	}
}
