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
use sp_core::ByteArray;

pub struct ProxyProxiesMigrator<T: Config> {
	_marker: sp_std::marker::PhantomData<T>,
}

pub struct ProxyAnnouncementMigrator<T: Config> {
	_marker: sp_std::marker::PhantomData<T>,
}

type BalanceOf<T> = <<T as pallet_proxy::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct RcProxy<AccountId, Balance, ProxyType, BlockNumber> {
	/// The account that is delegating to their proxy.
	pub delegator: AccountId,
	/// The deposit that was `Currency::reserved` from the delegator.
	pub deposit: Balance,
	/// The proxies that were delegated to and that can act on behalf of the delegator.
	pub proxies: Vec<pallet_proxy::ProxyDefinition<AccountId, ProxyType, BlockNumber>>,
}

pub type RcProxyOf<T, ProxyType> =
	RcProxy<AccountIdOf<T>, BalanceOf<T>, ProxyType, BlockNumberFor<T>>;

/// A RcProxy in Relay chain format, can only be understood by the RC and must be translated first.
pub(crate) type RcProxyLocalOf<T> = RcProxyOf<T, <T as pallet_proxy::Config>::ProxyType>;

/// A deposit that was taken for a proxy announcement.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
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
		let mut batch = Vec::new();

		let mut key_iter = if let Some(last_key) = last_key.clone() {
			pallet_proxy::Proxies::<T>::iter_keys_from(pallet_proxy::Proxies::<T>::hashed_key_for(
				&last_key,
			))
		} else {
			pallet_proxy::Proxies::<T>::iter_keys()
		};

		loop {
			let Some(acc) = key_iter.next() else {
				last_key = None;
				log::info!(target: LOG_TARGET, "No more proxies to migrate, last key: {:?}", &last_key);
				break;
			};
			log::debug!("Migrating proxies of acc {:?}", acc);

			let (proxies, deposit) = pallet_proxy::Proxies::<T>::get(&acc);

			if proxies.is_empty() {
				last_key = None;
				defensive!("No more proxies to migrate");
				break;
			};

			match Self::migrate_single(acc.clone(), (proxies.into_inner(), deposit), weight_counter)
			{
				Ok(proxy) => batch.push(proxy),
				Err(Error::OutOfWeight) if batch.len() > 0 => {
					log::info!(target: LOG_TARGET, "Out of weight, continuing with next batch");
					break;
				},
				Err(Error::OutOfWeight) if batch.len() == 0 => {
					defensive!("Not enough weight to migrate a single account");
					return Err(Error::OutOfWeight);
				},
				Err(e) => {
					defensive!("Error while migrating account");
					log::error!(target: LOG_TARGET, "Error while migrating account: {:?}", e);
					return Err(e);
				},
			}

			last_key = Some(acc); // Mark as successfully migrated
		}

		// TODO send xcm
		if !batch.is_empty() {
			Self::send_batch_xcm(batch)?;
		}
		log::info!(target: LOG_TARGET, "Last key: {:?}", &last_key);

		Ok(last_key)
	}
}

impl<T: Config> ProxyProxiesMigrator<T> {
	fn migrate_single(
		acc: AccountIdOf<T>,
		(proxies, deposit): (
			Vec<pallet_proxy::ProxyDefinition<T::AccountId, T::ProxyType, BlockNumberFor<T>>>,
			BalanceOf<T>,
		),
		weight_counter: &mut WeightMeter,
	) -> Result<RcProxyLocalOf<T>, Error<T>> {
		if weight_counter.try_consume(Weight::from_all(1_000)).is_err() {
			return Err(Error::<T>::OutOfWeight);
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

	/// Storage changes must be rolled back on error.
	fn send_batch_xcm(mut proxies: Vec<RcProxyLocalOf<T>>) -> Result<(), Error<T>> {
		const MAX_MSG_SIZE: u32 = 50_000; // Soft message size limit. Hard limit is about 64KiB

		while !proxies.is_empty() {
			let mut remaining_size: u32 = MAX_MSG_SIZE;
			let mut batch = Vec::new();

			while !proxies.is_empty() {
				// Order does not matter, so we take from the back as optimization
				let proxy = proxies.last().unwrap(); // FAIL-CI no unwrap
				let msg_size = proxy.encoded_size() as u32;
				if msg_size > remaining_size {
					break;
				}
				remaining_size -= msg_size;

				batch.push(proxies.pop().unwrap()); // FAIL-CI no unwrap
			}

			log::info!(target: LOG_TARGET, "Sending batch of {} proxies", batch.len());
			let call = types::AssetHubPalletConfig::<T>::AhmController(
				types::AhMigratorCall::<T>::ReceiveProxyProxies { proxies: batch },
			);

			let message = Xcm(vec![
				Instruction::UnpaidExecution {
					weight_limit: WeightLimit::Unlimited,
					check_origin: None,
				},
				Instruction::Transact {
					origin_kind: OriginKind::Superuser,
					require_weight_at_most: Weight::from_all(1), // TODO
					call: call.encode().into(),
				},
			]);

			if let Err(err) = send_xcm::<T::SendXcm>(
				Location::new(0, [Junction::Parachain(1000)]),
				message.clone(),
			) {
				log::error!(target: LOG_TARGET, "Error while sending XCM message: {:?}", err);
				return Err(Error::TODO);
			};
		}

		Ok(())
	}
}

impl<T: Config> PalletMigration for ProxyAnnouncementMigrator<T> {
	type Key = T::AccountId;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		if weight_counter.try_consume(Weight::from_all(1_000)).is_err() {
			return Err(Error::<T>::OutOfWeight);
		}

		let mut batch = Vec::new();
		let mut iter = if let Some(last_key) = last_key {
			pallet_proxy::Proxies::<T>::iter_from(pallet_proxy::Proxies::<T>::hashed_key_for(
				&last_key,
			))
		} else {
			pallet_proxy::Proxies::<T>::iter()
		};

		while let Some((acc, (_announcements, deposit))) = iter.next() {
			if weight_counter.try_consume(Weight::from_all(1_000)).is_err() {
				break;
			}

			batch.push(RcProxyAnnouncement { depositor: acc, deposit });
		}

		if !batch.is_empty() {
			Self::send_batch_xcm(batch)?;
		}

		Ok(None)
	}
}

impl<T: Config> ProxyAnnouncementMigrator<T> {
	fn send_batch_xcm(mut announcements: Vec<RcProxyAnnouncementOf<T>>) -> Result<(), Error<T>> {
		const MAX_MSG_SIZE: u32 = 50_000; // Soft message size limit. Hard limit is about 64KiB

		while !announcements.is_empty() {
			let mut remaining_size: u32 = MAX_MSG_SIZE;
			let mut batch = Vec::new();

			while !announcements.is_empty() {
				let announcement = announcements.last().unwrap(); // FAIL-CI no unwrap
				let msg_size = announcement.encoded_size() as u32;
				if msg_size > remaining_size {
					break;
				}
				remaining_size -= msg_size;

				batch.push(announcements.pop().unwrap()); // FAIL-CI no unwrap
			}

			log::info!(target: LOG_TARGET, "Sending batch of {} proxy announcements", batch.len());
			let call = types::AssetHubPalletConfig::<T>::AhmController(
				types::AhMigratorCall::<T>::ReceiveProxyAnnouncements { announcements: batch },
			);

			let message = Xcm(vec![
				Instruction::UnpaidExecution {
					weight_limit: WeightLimit::Unlimited,
					check_origin: None,
				},
				Instruction::Transact {
					origin_kind: OriginKind::Superuser,
					require_weight_at_most: Weight::from_all(1), // TODO
					call: call.encode().into(),
				},
			]);

			if let Err(err) = send_xcm::<T::SendXcm>(
				Location::new(0, [Junction::Parachain(1000)]),
				message.clone(),
			) {
				log::error!(target: LOG_TARGET, "Error while sending XCM message: {:?}", err);
				return Err(Error::TODO);
			};
		}

		Ok(())
	}
}
