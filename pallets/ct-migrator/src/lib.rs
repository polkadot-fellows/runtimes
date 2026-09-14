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

//! Receiver side of the AHM v2 migration.
//! Note: This is usually CoreTime chain, but for networks (such as Paseo) w/o CT, we will use
//! this on AH.
//!
//! Ingests state sent by `pallet-rc2-migrator`, writing through the same code paths as ordinary
//! extrinsics so that migrated and natively created state are indistinguishable. Temporary
//! pallet; removed once the migration is complete.
//!
//! Has no stage machine of its own; the relay chain drives every transition. The stage is stored
//! so this chain can check locally whether the migration is over.
//!
//! Calls are gated by root. Any signal that is already acted on is accepted (so idempotent), and
//! a signal out of order is an error.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

use alloc::vec;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use xcm::prelude::*;

const LOG_TARGET: &str = "runtime::ct-migrator";

/// Progress of the migration, as this chain sees it. Advanced by messages from
/// `pallet-rc2-migrator`.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Default,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum MigrationStage {
	/// No migration has started.
	#[default]
	Pending,
	/// The relay chain is draining state to this chain.
	DataMigrationOngoing,
	MigrationDone,
}

impl MigrationStage {
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	pub fn is_ongoing(&self) -> bool {
		matches!(self, Self::DataMigrationOngoing)
	}
}

/// `Rc2Migrator`'s pallet index in the relay `construct_runtime!`.
pub const RC2_MIGRATOR_PALLET_INDEX: u8 = 254;

/// Calls on the relay chain, as this chain must encode them.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Rc2RuntimeCall {
	Rc2Migrator(Rc2MigratorCall) = RC2_MIGRATOR_PALLET_INDEX,
}

/// Indices are the `#[pallet::call_index]`es in `pallet-rc2-migrator`.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum Rc2MigratorCall {
	#[codec(index = 2)]
	CtReady,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Router for XCM messages to the relay chain.
		type SendXcm: SendXcm;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type CtMigrationStage<T: Config> = StorageValue<_, MigrationStage, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The migration has already finished on this chain.
		AlreadyFinished,
		/// The relay chain signalled the end of a migration that never started here.
		NotStarted,
		/// Sending an XCM message to the relay chain failed.
		XcmSendFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		StageTransition { old: MigrationStage, new: MigrationStage },
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The relay chain asks whether this chain can receive migrated state.
		///
		/// The CT response unblocks the relay chain's first data stage. A relay chain rewound to
		/// `Scheduled` re-runs the handshake, which is accepted here without a matching rewind.
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
		pub fn start_migration(origin: OriginFor<T>) -> DispatchResult {
			// relay chain origin converts to root.
			ensure_root(origin)?;

			match CtMigrationStage::<T>::get() {
				// try send xcm before updating stage.
				MigrationStage::Pending => {
					Self::send_to_rc(Rc2MigratorCall::CtReady)?;
					Self::transition(MigrationStage::DataMigrationOngoing);
				},
				MigrationStage::DataMigrationOngoing => Self::send_to_rc(Rc2MigratorCall::CtReady)?,
				MigrationStage::MigrationDone => return Err(Error::<T>::AlreadyFinished.into()),
			}
			Ok(())
		}

		/// The relay chain signals that all data has been sent.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn finish_migration(origin: OriginFor<T>) -> DispatchResult {
			// rc origin
			ensure_root(origin)?;

			match CtMigrationStage::<T>::get() {
				MigrationStage::DataMigrationOngoing =>
					Self::transition(MigrationStage::MigrationDone),
				MigrationStage::MigrationDone => (),
				MigrationStage::Pending => return Err(Error::<T>::NotStarted.into()),
			}
			Ok(())
		}

		/// Set the migration stage directly. See `pallet-rc2-migrator`'s equivalent.
		#[pallet::call_index(2)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStage) -> DispatchResult {
			ensure_root(origin)?;

			Self::transition(stage);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn transition(new: MigrationStage) {
			let old = CtMigrationStage::<T>::mutate(|stage| core::mem::replace(stage, new.clone()));
			log::info!(target: LOG_TARGET, "Stage transition: {old:?} -> {new:?}");
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Send a `pallet-rc2-migrator` call to the relay chain.
		fn send_to_rc(call: Rc2MigratorCall) -> Result<(), Error<T>> {
			let call = Rc2RuntimeCall::Rc2Migrator(call);
			// This is dispatched as `Origin::Xcm(<this chain>)`.
			let message = Xcm(vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.encode().into(),
				},
			]);

			send_xcm::<T::SendXcm>(Location::parent(), message).map_err(|e| {
				log::error!(target: LOG_TARGET, "Sending to RC failed: {e:?}");
				Error::<T>::XcmSendFailed
			})?;
			Ok(())
		}
	}
}
