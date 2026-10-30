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
use frame_support::{
	defensive, defensive_assert,
	traits::{
		fungible::{InspectHold, MutateHold},
		tokens::Precision,
		Currency, Get, ReservableCurrency,
	},
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
				Self::park_proxy(proxy);
			},
		);
		Self::deposit_event(Event::ProxiesReceived { count_good, count_bad });
	}

	/// Park a set that could not be written, merged with one already parked for the same
	/// delegator.
	fn park_proxy(proxy: PortableProxyOf<T>) {
		let who = proxy.delegator.clone();
		let Some(mut parked) = FailedProxies::<T>::get(&who) else {
			FailedProxies::<T>::insert(&who, proxy);
			return;
		};
		// Each relay chain entry is visited once and only a para sovereign is renamed on the
		// wire, so one delegator parks once unless a `sibl` account on the relay chain also
		// holds proxies; none does. The merge keeps the second set anyway.
		defensive!("the same delegator parked twice", who);
		for delegate in proxy.delegates {
			if !parked.delegates.contains(&delegate) && parked.delegates.try_push(delegate).is_err()
			{
				defensive!("more parked definitions than the wire holds", who);
			}
		}
		FailedProxies::<T>::insert(&who, parked);
	}

	/// Write one proxy set, merged with the delegator's entry here. Delays are converted from
	/// relay chain blocks, rounding up. The `ProxyDeposit` hold is released and this chain's
	/// deposit for the entry is reserved out of it, capped at what was migrated; the rest stays
	/// free. A set that arrives without a hold, as a pure proxy's does, is recorded with a
	/// deposit of 0 until its owner calls `poke_deposit`.
	fn do_receive_proxy(proxy: &PortableProxyOf<T>) -> DispatchResult {
		let who = &proxy.delegator;
		// the relay chain sends no empty set, and the proxy pallet keeps no empty entry
		defensive_assert!(!proxy.delegates.is_empty(), "an empty proxy set");
		if proxy.delegates.is_empty() {
			return Ok(());
		}

		// release the migrated deposit whole
		let reason: T::RuntimeHoldReason = HoldReason::ProxyDeposit.into();
		let migrated = CurrencyOf::<T>::balance_on_hold(&reason, who);
		if !migrated.is_zero() {
			CurrencyOf::<T>::release(&reason, who, migrated, Precision::Exact)?;
		}
		let migrated: ProxyBalanceOf<T> = migrated.saturated_into::<u128>().saturated_into();

		// written directly: `add_proxy_delegate` refuses a duplicate and reserves the full
		// deposit from free balance
		let ratio = T::RcBlockTimeRatio::get();
		pallet_proxy::Proxies::<T>::try_mutate(who, |(defs, deposit)| {
			for delegate in &proxy.delegates {
				// `NoSelfProxy`: the proxy pallet's own calls refuse one, so the relay chain holds
				// none
				defensive_assert!(delegate.delegate != *who, "a self proxy");
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
