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
//! pallet. A set that fails is rolled back and parked in `FailedProxies`; the rest of the batch
//! continues.

#[cfg(test)]
mod tests;

use crate::{Config, CurrencyOf, Event, FailedProxies, HoldReason, Pallet, LOG_TARGET};
use alloc::vec::Vec;
use frame_support::traits::{
	fungible::{InspectHold, MutateHold},
	tokens::Precision,
	Currency, Get, ReservableCurrency,
};
use migrator_types::PortableProxy;
use sp_runtime::{traits::Zero, DispatchResult, SaturatedConversion, Saturating};

pub type PortableProxyOf<T> = PortableProxy<<T as frame_system::Config>::AccountId>;
/// The proxy pallet's balance; `pallet_proxy::BalanceOf` is private.
type ProxyBalanceOf<T> = <<T as pallet_proxy::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

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

	/// Write one proxy set, merged with what the delegator already has here. Delays arrive in
	/// relay chain blocks and are converted to this chain's, rounding up. The `ProxyDeposit`
	/// hold the accounts stage placed is released, the entry is reserved at this chain's rates
	/// out of it and nothing else, and the rest stays free: a set that arrives without a
	/// deposit, as a pure proxy's does, is written with none, and its owner can `poke_deposit`.
	fn do_receive_proxy(proxy: &PortableProxyOf<T>) -> DispatchResult {
		let who = &proxy.delegator;

		// release the migrated deposit whole
		let reason: T::RuntimeHoldReason = HoldReason::ProxyDeposit.into();
		let migrated = CurrencyOf::<T>::balance_on_hold(&reason, who);
		if !migrated.is_zero() {
			CurrencyOf::<T>::release(&reason, who, migrated, Precision::Exact)?;
		}
		let migrated: ProxyBalanceOf<T> = migrated.saturated_into::<u128>().saturated_into();

		// `add_proxy_delegate` would refuse a duplicate, so the entry is written directly
		let ratio = T::RcBlockTimeRatio::get();
		pallet_proxy::Proxies::<T>::try_mutate(who, |(defs, deposit)| {
			for delegate in &proxy.delegates {
				let def = pallet_proxy::ProxyDefinition {
					delegate: delegate.delegate.clone(),
					proxy_type: delegate.proxy_type.into(),
					delay: delegate.delay.div_ceil(ratio).into(),
				};
				// Kept sorted, as the pallet keeps it: its own calls find entries by binary search.
				if let Err(i) = defs.binary_search(&def) {
					defs.try_insert(i, def).map_err(|_| pallet_proxy::Error::<T>::TooMany)?;
				}
			}

			// reserve at this chain's rates, out of the migrated deposit only
			let required = pallet_proxy::Pallet::<T>::deposit(defs.len() as u32);
			let top_up = required.saturating_sub(*deposit).min(migrated);
			if !top_up.is_zero() {
				<T as pallet_proxy::Config>::Currency::reserve(who, top_up)?;
				*deposit = deposit.saturating_add(top_up);
			}
			Ok(())
		})
	}
}
