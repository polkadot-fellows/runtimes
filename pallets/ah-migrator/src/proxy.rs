// This file is part of Substrate.

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

use crate::{types::*, *};
use sp_runtime::BoundedSlice;

impl<T: Config> Pallet<T> {
	pub fn do_receive_proxies(proxies: Vec<RcProxyOf<T, T::RcProxyType>>) -> Result<(), Error<T>> {
		Self::deposit_event(Event::ProxyBatchReceived { count: proxies.len() as u32 });
		let (mut count_good, mut count_bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} proxies", proxies.len());

		for proxy in proxies {
			match Self::do_receive_proxy(proxy) {
				Ok(()) => count_good += 1,
				Err(e) => {
					count_bad += 1;
					log::error!(target: LOG_TARGET, "Error while integrating proxy: {:?}", e);
				},
			}
		}
		Self::deposit_event(Event::ProxyBatchProcessed { count_good, count_bad });

		Ok(())
	}

	/// Receive a single proxy and write it to storage.
	pub fn do_receive_proxy(proxy: RcProxyOf<T, T::RcProxyType>) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Integrating proxy {}, deposit {:?}", proxy.delegator.to_ss58check(), proxy.deposit);

		let max_proxies = <T as pallet_proxy::Config>::MaxProxies::get() as usize;
		if proxy.proxies.len() > max_proxies {
			log::error!(target: LOG_TARGET, "Truncating proxy list of {}", proxy.delegator.to_ss58check());
		}

		let proxies = proxy.proxies.into_iter().enumerate().filter_map(|(i, p)| {
			let Ok(proxy_type) = T::RcToProxyType::try_convert(p.proxy_type) else {
				// This is fine, eg. `Auction` proxy is not supported on AH
				log::warn!(target: LOG_TARGET, "Dropping unsupported proxy at index {} for {}", i, proxy.delegator.to_ss58check());
				return None;
			};

			let Ok(delay) = T::RcToProxyDelay::try_convert(p.delay).defensive() else {
				return None;
			};

			Some(pallet_proxy::ProxyDefinition {
				delegate: p.delegate,
				delay: delay,
				proxy_type,
			})
		})
		.take(max_proxies)
		.collect::<Vec<_>>();

		let Ok(bounded_proxies) =
			BoundedSlice::try_from(proxies.as_slice()).defensive_proof("unreachable")
		else {
			return Err(Error::TODO);
		};

		// The deposit was already taken by the account migration

		// Add the proxies
		pallet_proxy::Proxies::<T>::insert(proxy.delegator, (bounded_proxies, proxy.deposit));

		Ok(())
	}
}
