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

#![doc = include_str!("proxy.md")]

use frame_support::traits::Currency;

extern crate alloc;
use crate::{types::*, *};
use alloc::vec::Vec;

pub struct ProxyProxiesMigrator<T> {
	_marker: sp_std::marker::PhantomData<T>,
}

pub struct ProxyAnnouncementMigrator<T: Config> {
	_marker: sp_std::marker::PhantomData<T>,
}

type BalanceOf<T> = <<T as pallet_proxy::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

#[derive(Encode, DecodeWithMemTracking, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct RcProxy<AccountId, Balance, ProxyType, BlockNumber> {
	/// The account that is delegating to their proxy.
	pub delegator: AccountId,
	/// The deposit that was `Currency::reserved` from the delegator.
	pub deposit: Balance,
	/// The proxies that were delegated to and that can act on behalf of the delegator.
	pub proxies: Vec<pallet_proxy::ProxyDefinition<AccountId, ProxyType, BlockNumber>>,
}

pub type RcProxyOf<T, ProxyType> =
	RcProxy<AccountIdOf<T>, BalanceOf<T>, ProxyType, pallet_proxy::BlockNumberFor<T>>;

/// A RcProxy in Relay chain format, can only be understood by the RC and must be translated first.
pub(crate) type RcProxyLocalOf<T> = RcProxyOf<T, <T as pallet_proxy::Config>::ProxyType>;

/// A deposit that was taken for a proxy announcement.
#[derive(Encode, DecodeWithMemTracking, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct RcProxyAnnouncement<AccountId, Balance> {
	pub depositor: AccountId,
	pub deposit: Balance,
}

pub type RcProxyAnnouncementOf<T> = RcProxyAnnouncement<AccountIdOf<T>, BalanceOf<T>>;

impl<T: Config> PalletMigration for ProxyProxiesMigrator<T> {
	type Key = T::AccountId;
	type Error = Error<T>;

	fn migrate_many(
		mut last_key: Option<AccountIdOf<T>>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<AccountIdOf<T>>, Error<T>> {
		let mut batch = XcmBatchAndMeter::new_from_config::<T>();

		// Get iterator starting after last processed key
		let mut key_iter = if let Some(last_key) = last_key.clone() {
			pallet_proxy::Proxies::<T>::iter_from(pallet_proxy::Proxies::<T>::hashed_key_for(
				&last_key,
			))
		} else {
			pallet_proxy::Proxies::<T>::iter()
		};

		// Process accounts until we run out of weight or accounts
		for (acc, (proxies, deposit)) in key_iter.by_ref() {
			if proxies.is_empty() {
				defensive!("The proxy pallet disallows empty proxy lists");
				continue;
			}

			match Self::migrate_single(
				acc.clone(),
				(proxies.clone().into_inner(), deposit),
				weight_counter,
				&mut batch,
			) {
				Ok(proxy) => {
					// We keep proxy relations of pure accounts alive for free, otherwise gives the
					// owner of the pure account a big headache with trying to control it through
					// the remote proxy pallet (no UI for it) or similar.
					match PureProxyCandidatesMigrated::<T>::get(&acc) {
						None => {
							pallet_proxy::Proxies::<T>::remove(&acc);
							batch.push(proxy); // Send over to AH
						},
						Some(false) => {
							PureProxyCandidatesMigrated::<T>::insert(&acc, true);

							let free_proxies: BoundedVec<_, _> = proxies
								.into_iter()
								.filter(|p| T::PureProxyFreeVariants::contains(&p.proxy_type))
								.collect::<Vec<_>>()
								.defensive_truncate_into();
							let deposit: BalanceOf<T> = Zero::zero();
							log::debug!(target: LOG_TARGET, "Pure account {} gets {} proxies for free: {:?}", acc.to_ss58check(), free_proxies.len(), free_proxies);

							if !free_proxies.is_empty() {
								pallet_proxy::Proxies::<T>::insert(&acc, (free_proxies, deposit));
							} else {
								log::warn!(target: LOG_TARGET, "Pure proxy account will lose access on the Relay Chain: {:?}", acc.to_ss58check());
								pallet_proxy::Proxies::<T>::remove(&acc);
							}

							batch.push(proxy); // Send over to AH
						},
						Some(true) => {
							// Already migrated
						},
					}

					last_key = Some(acc); // Update last processed key
				},
				Err(OutOfWeightError) if !batch.is_empty() => {
					// We have items to process but ran out of weight
					break;
				},
				Err(OutOfWeightError) => {
					defensive!("Not enough weight to migrate a single account");
					return Err(Error::OutOfWeight);
				},
			}
		}

		// Send batch if we have any items
		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(batch, |batch| {
				types::AhMigratorCall::<T>::ReceiveProxyProxies { proxies: batch }
			})?;
		}

		// Return last processed key if there are more items, None if we're done
		if key_iter.next().is_some() {
			Ok(last_key)
		} else {
			Ok(None)
		}
	}
}

type ProxiesOf<T> = Vec<
	pallet_proxy::ProxyDefinition<
		<T as frame_system::Config>::AccountId,
		<T as pallet_proxy::Config>::ProxyType,
		pallet_proxy::BlockNumberFor<T>,
	>,
>;

impl<T: Config> ProxyProxiesMigrator<T> {
	fn migrate_single(
		acc: AccountIdOf<T>,
		(proxies, deposit): (ProxiesOf<T>, BalanceOf<T>),
		weight_counter: &mut WeightMeter,
		batch: &mut XcmBatchAndMeter<RcProxyLocalOf<T>>,
	) -> Result<RcProxyLocalOf<T>, OutOfWeightError> {
		if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err()
			|| weight_counter.try_consume(batch.consume_weight()).is_err()
		{
			log::info!(
				target: LOG_TARGET,
				"RC weight limit reached at batch length {}, stopping",
				batch.len()
			);
			return Err(OutOfWeightError);
		}

		if T::MaxAhWeight::get().any_lt(T::AhWeightInfo::receive_proxy_proxies(batch.len() + 1)) {
			log::info!(
				target: LOG_TARGET,
				"AH weight limit reached at batch length {}, stopping",
				batch.len()
			);
			return Err(OutOfWeightError);
		}

		if batch.len() >= MAX_ITEMS_PER_BLOCK {
			log::info!(
				target: LOG_TARGET,
				"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
				MAX_ITEMS_PER_BLOCK,
				batch.len()
			);
			return Err(OutOfWeightError);
		}

		if batch.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
			log::info!(
				target: LOG_TARGET,
				"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
				MAX_XCM_MSG_PER_BLOCK,
				batch.batch_count()
			);
			return Err(OutOfWeightError);
		}

		let translated_proxies = proxies
			.into_iter()
			.map(|proxy| pallet_proxy::ProxyDefinition {
				delegate: proxy.delegate,
				proxy_type: proxy.proxy_type,
				delay: proxy.delay,
			})
			.collect();

		let mapped = RcProxy { delegator: acc, deposit, proxies: translated_proxies };

		Ok(mapped)
	}
}

impl<T: Config> PalletMigration for ProxyAnnouncementMigrator<T> {
	type Key = T::AccountId;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut batch = XcmBatchAndMeter::new_from_config::<T>();
		let mut last_processed = None;

		// Get iterator starting after last processed key
		let mut iter = if let Some(last_key) = last_key {
			pallet_proxy::Announcements::<T>::iter_from(
				pallet_proxy::Announcements::<T>::hashed_key_for(&last_key),
			)
		} else {
			pallet_proxy::Announcements::<T>::iter()
		};

		// Process announcements until we run out of weight
		for (acc, (_announcements, deposit)) in iter.by_ref() {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err()
				|| weight_counter.try_consume(batch.consume_weight()).is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if T::MaxAhWeight::get()
				.any_lt(T::AhWeightInfo::receive_proxy_announcements(batch.len() + 1))
			{
				log::info!(
					target: LOG_TARGET,
					"AH weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if batch.len() >= MAX_ITEMS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					batch.len()
				);
				break;
			}

			if batch.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					batch.batch_count()
				);
				break;
			}

			batch.push(RcProxyAnnouncement { depositor: acc.clone(), deposit });
			pallet_proxy::Announcements::<T>::remove(&acc);
			last_processed = Some(acc);
		}

		// Send batch if we have any items
		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(batch, |batch| {
				types::AhMigratorCall::<T>::ReceiveProxyAnnouncements { announcements: batch }
			})?;
		}

		// Return last processed key if there are more items, None if we're done
		if iter.next().is_some() {
			Ok(last_processed)
		} else {
			Ok(None)
		}
	}
}

#[cfg(feature = "std")]
use std::collections::btree_map::BTreeMap;

#[cfg(feature = "std")]
impl<T: Config> RcMigrationCheck for ProxyProxiesMigrator<T> {
	type RcPrePayload =
		BTreeMap<(AccountId32, u32), Vec<(<T as pallet_proxy::Config>::ProxyType, AccountId32)>>; // Map of (Delegator, None) -> (Kind, Delegatee)

	fn pre_check() -> Self::RcPrePayload {
		// Store the proxies per account before the migration
		let mut proxies = BTreeMap::new();
		for (delegator, (delegations, _deposit)) in pallet_proxy::Proxies::<T>::iter() {
			let nonce = frame_system::Pallet::<T>::account_nonce(&delegator);

			for delegation in delegations {
				proxies
					.entry((delegator.clone(), nonce))
					.or_insert_with(Vec::new)
					.push((delegation.proxy_type, delegation.delegate));
			}
		}
		proxies
	}

	fn post_check(pre_accs: Self::RcPrePayload) {
		// sanity check
		let remaining = pallet_proxy::Proxies::<T>::iter_keys().count();
		assert!(remaining >= 10, "Not enough remaining pure proxies, {remaining}");

		// All remaining ones are 'Any'
		for (delegator, (proxies, deposit)) in pallet_proxy::Proxies::<T>::iter() {
			assert_eq!(
				deposit,
				Zero::zero(),
				"Pure account {} should have no deposit but has {:?}",
				delegator.to_ss58check(),
				deposit
			);

			for proxy in proxies {
				let enc_type = proxy.proxy_type.encode();
				let is_any = enc_type == vec![0];

				assert!(is_any, "Pure proxy got wrong account for free");
			}
		}

		for ((acc, nonce), proxies) in pre_accs.into_iter() {
			if nonce != 0 {
				continue;
			}

			// Amount of any proxies
			let num_any = proxies
				.iter()
				.filter(|(proxy_type, _)| T::PureProxyFreeVariants::contains(proxy_type))
				.count();
			if num_any == 0 {
				assert!(
					!pallet_proxy::Proxies::<T>::contains_key(&acc),
					"No empty vectors in storage"
				);
				continue;
			}

			log::debug!(
				"Checking Pure proxy {} has proxies afterwards: {:?} and before: {:?}",
				acc.to_ss58check(),
				pallet_proxy::Proxies::<T>::get(&acc),
				proxies
			);
			assert_eq!(pallet_proxy::Proxies::<T>::get(&acc).1, Zero::zero());
			assert_eq!(pallet_proxy::Proxies::<T>::get(&acc).0.len(), num_any);
		}

		let count = pallet_proxy::Announcements::<T>::iter_keys().count();
		assert_eq!(count, 0, "Assert storage 'Proxy::Announcements::rc_post::empty'");
	}
}
