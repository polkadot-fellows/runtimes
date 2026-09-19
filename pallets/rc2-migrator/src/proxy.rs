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

//! Proxy stage: migrates proxy definitions to the Coretime chain, where para management
//! continues.
//!
//! Scope, per the migration design:
//! - Every definition whose permission the Coretime chain represents travels (the runtime's
//!   `TryInto<PortableProxyType>`: `Any`, `NonTransfer`, `CancelProxy`, `ParaRegistration`);
//!   everything else (staking, governance, …) has no meaning there and stays on this chain. Keyless
//!   (pure) delegators can only ever act through definitions recreated on the Coretime chain — the
//!   accounts stage routes their whole balance there for the same reason; for keyed delegators the
//!   recreation is a harmless convenience.
//! - Deposits do not travel: they were refunded by the accounts stage. Entries left behind get
//!   their recorded deposit clamped to what is still actually reserved, so no ghost deposit records
//!   are created.
//! - Entries of delegators whose accounts already migrated are deleted. For definitions that did
//!   not travel this drops them entirely (deposit refunded, nothing recreated) — safe, because a
//!   signer can recreate on Asset Hub, but whether they should instead be recreated on Asset Hub
//!   (as v1 did) is an OPEN policy question; this stage is where an AH-bound lane would plug in.

use crate::*;
use sp_runtime::traits::UniqueSaturatedInto;

pub struct ProxyMigrator<T>(PhantomData<T>);

impl<T: Config> ProxyMigrator<T> {
	/// Drop the announcement records of announcers whose accounts migrated away. Their deposits
	/// were refunded by the accounts stage (indexed as [`ExpectedRefundReserve`]), so keeping the
	/// record would claim money that is gone. Announcers still on this chain (kept accounts) keep
	/// their record with the recorded deposit clamped to what is still actually reserved — the
	/// same no-ghost-deposit rule as proxy entries.
	///
	/// One-shot, called by `ProxyInit`; the announcement count is small. The caller wraps this in
	/// a storage transaction.
	pub fn drain_announcements() -> Result<(), Error<T>> {
		let mut count = 0u32;
		let entries: Vec<_> = pallet_proxy::Announcements::<T>::iter().collect();
		for (announcer, (announcements, deposit)) in entries {
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
					count += 1;
				},
			}
		}
		if count > 0 {
			log::info!(target: LOG_TARGET, "Dropped {count} announcement records");
		}
		Ok(())
	}

	/// Migrate proxy definitions until the per-block limit is reached.
	///
	/// Returns the cursor to continue from on the next block, or `None` once the map is
	/// exhausted. The caller wraps this in a storage transaction; an `Err` rolls back the whole
	/// block's changes.
	pub fn migrate_many(last_key: Option<T::AccountId>) -> Result<Option<T::AccountId>, Error<T>> {
		let iter = match &last_key {
			Some(last_key) => pallet_proxy::Proxies::<T>::iter_from_key(last_key.clone()),
			None => pallet_proxy::Proxies::<T>::iter(),
		};

		Pallet::<T>::drain_records(
			iter,
			|who, (defs, deposit)| {
				// Convert each definition once; `Ok` means the Coretime chain represents the
				// permission and the definition travels, `Err` means it stays here.
				let defs: Vec<_> = defs
					.into_iter()
					.map(|def| {
						let portable: Result<PortableProxyType, ()> =
							def.proxy_type.clone().try_into().map_err(|_| ());
						(def, portable)
					})
					.collect();
				let (travel, stay): (Vec<_>, Vec<_>) =
					defs.into_iter().partition(|(_, portable)| portable.is_ok());

				let payload = if travel.is_empty() {
					None
				} else {
					let delegates: Vec<_> = travel
						.into_iter()
						.map(|(def, portable)| migrator_types::PortableProxyDelegate {
							// Account ids on the wire are always the destination's address for
							// them — see `migrator_types::translate_destination`. That covers the
							// delegator (whose deposit the accounts stage moved to the translated
							// address) and each delegate (a parachain that is somebody's delegate
							// is a different address there too).
							delegate: migrator_types::translate_destination(&def.delegate),
							proxy_type: portable.expect("partition kept only Ok conversions; qed"),
							delay: def.delay.unique_saturated_into(),
						})
						.collect();
					Some(PortableProxy {
						delegator: migrator_types::translate_destination(who),
						delegates: delegates
							.try_into()
							.map_err(|_| Error::<T>::FailedToWithdrawAccount)?,
					})
				};

				// A delegator with no funds has nothing that could strand: its manager-linked
				// definitions were sent above, the rest of the entry is deleted — this cleans the
				// zero-balance husks v1 left behind. For funded delegators, the deposit was
				// refunded by the accounts stage; clamp the recorded field to what is still
				// actually reserved so the entry never claims money that is gone.
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

				Ok(payload)
			},
			Pallet::<T>::send_proxies,
		)
	}
}
