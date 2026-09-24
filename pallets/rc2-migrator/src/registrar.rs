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

//! Registrar stage: drains `paras_registrar::Paras` records from the relay chain and sends them
//! to the Coretime chain in portable format.
//!
//! The registration deposits themselves already moved during the accounts stage, as
//! `RegistrarDeposit` holds on the manager accounts; the receiving side releases each one as its
//! record arrives so the registrar pallet can take its own deposit. `NextFreeParaId` moves in the
//! stage-init message; `PendingSwap` is deliberately left behind.

use crate::*;
use runtime_parachains::paras;

pub struct RegistrarMigrator<T>(PhantomData<T>);

impl<T: Config> RegistrarMigrator<T> {
	/// Stage init: send `NextFreeParaId` whole and remove it here. `PendingSwap` is deliberately
	/// left behind (`pub(super)` storage, ephemeral swap intent). The caller wraps this in a
	/// storage transaction; a failed send rolls everything back for a retry.
	pub fn migrate_init() -> Result<(), Error<T>> {
		let next_free: u32 = paras_registrar::NextFreeParaId::<T>::get().into();
		Pallet::<T>::send_registrar(Vec::new(), Some(next_free))?;
		paras_registrar::NextFreeParaId::<T>::kill();
		Ok(())
	}

	/// Drain registrar records until the per-block limit is reached.
	///
	/// Returns the cursor to continue from on the next block, or `None` once the map is
	/// exhausted. The caller wraps this in a storage transaction; an `Err` rolls back the whole
	/// block's removals.
	pub fn migrate_many(last_key: Option<ParaId>) -> Result<Option<ParaId>, Error<T>> {
		let iter = match last_key {
			Some(last_key) => paras_registrar::Paras::<T>::iter_from_key(last_key),
			None => paras_registrar::Paras::<T>::iter(),
		};

		Pallet::<T>::drain_records(
			iter,
			|para_id, info| {
				// Removing the current key while iterating a map is sound.
				paras_registrar::Paras::<T>::remove(para_id);
				Ok(Some(PortableParaInfo {
					para_id: (*para_id).into(),
					// Account ids on the wire are always the destination's address for them —
					// see `migrator_types::translate_destination`.
					manager: migrator_types::translate_destination(&info.manager),
					deposit: info.deposit,
					// Carried whole. The registrar leaves this unset until a para's first head,
					// and the destination needs to tell "never locked" from "unlocked on
					// purpose" — only the first is still eligible for its automatic lock.
					locked: info.locked,
					// Both read here because only this chain can: the lifecycle map and the head
					// data stay behind, and the destination prices the arriving registration from
					// the head length exactly as it would price a fresh one.
					registered: paras::Pallet::<T>::lifecycle(*para_id).is_some(),
					head_len: paras::Heads::<T>::get(para_id)
						.map(|head| head.0.len() as u32)
						.unwrap_or_default(),
				}))
			},
			|batch| Pallet::<T>::send_registrar(batch, None),
		)
	}
}
