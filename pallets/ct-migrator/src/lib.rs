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

//! The operational pallet for the Coretime chain, designed to manage and facilitate the migration
//! of the parachain registrar, HRMP and the accounts holding their deposits from the Relay Chain
//! to the Coretime chain. This pallet works alongside its counterpart, `pallet_rc2_migrator`,
//! which handles migration processes on the Relay Chain side. On a network without a Coretime
//! chain (Paseo), it runs on Asset Hub.
//!
//! This pallet ingests the migrated state, writing through the same code paths as ordinary
//! extrinsics so that migrated and natively created state are indistinguishable. It has no stage
//! machine of its own; the Relay Chain drives every transition and the stage is stored so this
//! chain can check locally whether the migration is over.
//!
//! The Relay Chain's signals are gated by root, and may also be sent by [`Config::AdminOrigin`]
//! or the [`Manager`] to stand in for one that never arrived. Any signal that is already acted on
//! is accepted (so idempotent), and a signal out of order is an error.
//!
//! # Differences from AHM v1
//!
//! This pallet follows `pallet_ah_migrator` (refer:
//! <https://github.com/polkadot-fellows/runtimes/tree/985df25829b3385730ff66acc50161ac57f0692c/pallets/ah-migrator>).
//! - The cool-off is held on the Relay Chain only. `end_lockdown` arrives when it ends, so there
//!   is no `CoolOff` stage here.
//! - No message-count confirmations or queue-priority controls are sent back.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

use alloc::vec;
use frame_support::{pallet_prelude::*, traits::EnsureOrigin};
use frame_system::pallet_prelude::*;
use xcm::prelude::*;

const LOG_TARGET: &str = "runtime::ct-migrator";

/// The migration stage of the Coretime chain. Advanced by messages from `pallet-rc2-migrator`.
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
	/// The migration has not started but will start in the future.
	#[default]
	Pending,
	/// Migrating data from the Relay Chain.
	DataMigrationOngoing,
	/// The migration is done.
	MigrationDone,
}

impl MigrationStage {
	/// Whether the migration is finished.
	///
	/// This is **not** the same as `!self.is_ongoing()` since it may not have started.
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	/// Whether the migration is ongoing.
	///
	/// This is **not** the same as `!self.is_finished()` since it may not have started.
	pub fn is_ongoing(&self) -> bool {
		matches!(self, Self::DataMigrationOngoing)
	}
}

/// `Rc2Migrator`'s pallet index in the relay `construct_runtime!`.
pub const RC2_MIGRATOR_PALLET_INDEX: u8 = 254;

/// Call encoding for the Relay Chain runtime, reduced to the pallet this chain dispatches into.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Rc2RuntimeCall {
	Rc2Migrator(Rc2MigratorCall) = RC2_MIGRATOR_PALLET_INDEX,
}

/// Call encoding for the calls needed from the rc2-migrator pallet.
///
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

		/// Send UMP message.
		type SendXcm: SendXcm;

		/// The origin that can perform permissioned operations like setting the migration stage.
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The Coretime chain migration state.
	#[pallet::storage]
	pub type CtMigrationStage<T: Config> = StorageValue<_, MigrationStage, ValueQuery>;

	/// An optional account id of a manager.
	///
	/// This account id has similar privileges to [`Config::AdminOrigin`] except that it
	/// can not set the manager account id via `set_manager` call.
	#[pallet::storage]
	pub type Manager<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The migration has already finished on this chain.
		AlreadyFinished,
		/// The Relay Chain signalled the end of a migration that never started here.
		NotStarted,
		/// Failed to send XCM message.
		XcmSendFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A stage transition has occurred.
		StageTransition {
			/// The old stage before the transition.
			old: MigrationStage,
			/// The new stage after the transition.
			new: MigrationStage,
		},
		/// The manager account id was set.
		ManagerSet {
			/// The old manager account id.
			old: Option<T::AccountId>,
			/// The new manager account id.
			new: Option<T::AccountId>,
		},
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Start the data migration.
		///
		/// This is called by the Relay Chain to start the migration on the Coretime chain and
		/// receive a handshake message indicating the Coretime chain's readiness. A Relay Chain
		/// rewound to `Scheduled` re-runs the handshake, which is accepted here without a matching
		/// rewind.
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().reads_writes(4, 2))]
		pub fn start_migration(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_root_or_admin_or_manager(origin)?;

			// TODO(ahm-v2): lock this chain down before answering, until the migration ends:
			// filter the calls whose state is about to move, and refuse inbound XCM from anyone
			// but the relay chain.
			// TODO(ahm-v2): give the relay chain's queue priority while the migration runs.
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

		/// Finish the migration.
		///
		/// This is called by the Relay Chain to signal the migration has finished and this chain's
		/// call filters may lift.
		// TODO(ahm-v2): the data stages add a `reconcile_balances` ahead of this one, carrying
		// the relay side's bookkeeping to check against. That one arrives at the start of the
		// window so a mismatch is visible while it can still be acted on; this one stays at the
		// end, because the filters must hold until the relay chain says they may drop.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn end_lockdown(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_root_or_admin_or_manager(origin)?;

			match CtMigrationStage::<T>::get() {
				MigrationStage::DataMigrationOngoing => {
					Self::transition(MigrationStage::MigrationDone)
				},
				MigrationStage::MigrationDone => (),
				MigrationStage::Pending => return Err(Error::<T>::NotStarted.into()),
			}
			Ok(())
		}

		/// Set the migration stage.
		///
		/// This call is intended for emergency use only and is guarded by the
		/// [`Config::AdminOrigin`] or the [`Manager`].
		#[pallet::call_index(2)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStage) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;

			Self::transition(stage);
			Ok(())
		}

		/// Set the manager account id.
		///
		/// The manager has the similar to [`Config::AdminOrigin`] privileges except that it
		/// can not set the manager account id via `set_manager` call. Each chain's manager is set
		/// on that chain; the Relay Chain does not carry one over.
		#[pallet::call_index(3)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn set_manager(origin: OriginFor<T>, new: Option<T::AccountId>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			let old = Manager::<T>::get();
			Manager::<T>::set(new.clone());
			Self::deposit_event(Event::ManagerSet { old, new });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Ensure that the origin is [`Config::AdminOrigin`] or signed by [`Manager`] account id.
		fn ensure_admin_or_manager(origin: OriginFor<T>) -> DispatchResult {
			if let Ok(who) = ensure_signed(origin.clone()) {
				if Manager::<T>::get().is_some_and(|manager| manager == who) {
					return Ok(());
				}
			}
			T::AdminOrigin::ensure_origin(origin)?;
			Ok(())
		}

		/// Ensure that the origin is root, which the Relay Chain's messages dispatch as, or one
		/// accepted by [`Self::ensure_admin_or_manager`].
		fn ensure_root_or_admin_or_manager(origin: OriginFor<T>) -> DispatchResult {
			if ensure_root(origin.clone()).is_err() {
				Self::ensure_admin_or_manager(origin)?;
			}
			Ok(())
		}

		/// Execute a stage transition and log it.
		pub(crate) fn transition(new: MigrationStage) {
			let old = CtMigrationStage::<T>::mutate(|stage| core::mem::replace(stage, new.clone()));
			log::info!(target: LOG_TARGET, "Stage transition: {old:?} -> {new:?}");
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Send a `pallet-rc2-migrator` call to the Relay Chain as a single XCM `Transact`.
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
				// A call that fails inside `Transact` does not fail the XCM by itself; this makes
				// it fail, so the relay chain reports it instead of a success.
				ExpectTransactStatus(MaybeErrorCode::Success),
			]);

			send_xcm::<T::SendXcm>(Location::parent(), message).map_err(|e| {
				log::error!(target: LOG_TARGET, "Sending to RC failed: {e:?}");
				Error::<T>::XcmSendFailed
			})?;
			Ok(())
		}
	}
}
