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

//! Relay-chain side of the AHM v2 migration.
//!
//! Drives the migration stage machine: drains the relay chain's remaining state and sends it to
//! the counterpart `pallet-ct-migrator` on the Coretime chain and to Asset Hub. Temporary pallet;
//! removed once the migration is complete.
//!
//! The machine is inert until governance schedules it: the default stage is
//! [`MigrationStage::Pending`], where `on_initialize` does nothing at all, and only root can move
//! it out of there. Data stages are added as they are implemented, between the readiness handshake
//! and the cool-off.

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

const LOG_TARGET: &str = "runtime::rc2-migrator";

pub type MigrationStageOf<T> = MigrationStage<BlockNumberFor<T>>;

/// Progress of the migration. Advanced by `on_initialize`, except where noted.
///
/// Data stages slot in between [`Self::WaitingForCt`] and [`Self::CoolOff`]: nothing may be
/// drained before the Coretime chain has confirmed it can receive, and nothing may be finished
/// before the verification window has passed.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub enum MigrationStage<BlockNumber> {
	/// Nothing has been scheduled. `on_initialize` does no work and no call is closed.
	#[default]
	Pending,
	/// Scheduled to begin at `start`. The relay chain still serves its users normally until then,
	/// which is what stops the runtime upgrade itself from being an outage.
	Scheduled {
		start: BlockNumber,
	},
	/// Halts the machine while keeping the migration "ongoing", so call filters stay engaged.
	/// Entered and left only via [`Pallet::force_set_stage`].
	Paused,
	/// The start signal has been sent and the Coretime chain has yet to confirm that it can
	/// receive data.
	///
	/// Deliberately has no timeout: if the confirmation never arrives, no state has been drained
	/// and the correct response is a human deciding what to do, via `force_set_stage`.
	WaitingForCt,
	/// All data sent; waiting for manual verification before finishing.
	CoolOff {
		end_at: BlockNumber,
	},
	MigrationDone,
}

impl<BlockNumber> MigrationStage<BlockNumber> {
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	/// Whether the machine is between its start and its end, and so whether the migration's own
	/// call filters and barriers are engaged.
	pub fn is_ongoing(&self) -> bool {
		!matches!(self, Self::Pending | Self::Scheduled { .. } | Self::MigrationDone)
	}

	/// Whether the migration has begun, and so whether the calls it moves away are closed.
	///
	/// A scheduled migration has not begun; a finished one has. This is the predicate for
	/// anything that must stay closed after the migration ends, where [`Self::is_ongoing`] is the
	/// one for anything that reopens.
	pub fn has_started(&self) -> bool {
		self.is_ongoing() || self.is_finished()
	}
}

/// Calls on the Coretime chain, as this chain must encode them.
///
/// Audit: the outer index is `CtMigrator`'s pallet index in the Coretime `construct_runtime!` and
/// the inner ones are `#[pallet::call_index]` in `pallet-ct-migrator`. Nothing here is checked by
/// the compiler, so the integration tests decode what this chain would send using the real
/// Coretime `RuntimeCall` — which is why these types are public.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum CtRuntimeCall {
	#[codec(index = 100)]
	CtMigrator(CtMigratorCall),
}

#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum CtMigratorCall {
	#[codec(index = 0)]
	StartMigration,
	#[codec(index = 1)]
	FinishMigration,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Router for XCM messages to the Coretime chain.
		type SendXcm: SendXcm;

		/// Para id of the Coretime chain.
		type CtParaId: Get<u32>;

		/// The origin the Coretime chain's own messages dispatch with here. Only it may confirm
		/// readiness — an acknowledgement from anywhere else says nothing about whether the
		/// destination can receive.
		type CtOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// How long the machine parks in [`MigrationStage::CoolOff`] before finishing, so the end
		/// state can be verified while the migration's filters are still engaged.
		type CoolOffPeriod: Get<BlockNumberFor<Self>>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::unbounded]
	pub type RcMigrationStage<T: Config> = StorageValue<_, MigrationStageOf<T>, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The migration can only be scheduled while it is pending.
		AlreadyScheduled,
		/// The migration cannot be scheduled to start in the past.
		StartInPast,
		/// Readiness was confirmed while the machine was not waiting for it.
		NotWaitingForCt,
		/// Sending an XCM message to the Coretime chain failed.
		XcmSendFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		StageTransition { old: MigrationStageOf<T>, new: MigrationStageOf<T> },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			Self::progress_migration(now)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule the migration to begin at `start`.
		///
		/// The only way out of [`MigrationStage::Pending`] other than `force_set_stage`, and the
		/// governance call that commits to a migration.
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn schedule_migration(
			origin: OriginFor<T>,
			start: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				RcMigrationStage::<T>::get() == MigrationStage::Pending,
				Error::<T>::AlreadyScheduled
			);
			ensure!(start > frame_system::Pallet::<T>::block_number(), Error::<T>::StartInPast);

			Self::transition(MigrationStage::Scheduled { start });
			Ok(())
		}

		/// Set the migration stage directly.
		///
		/// The escape hatch for everything the machine cannot recover from itself: a lost message,
		/// a stage that needs re-running, or a halt. Deliberately unconstrained — a machine that
		/// second-guesses root here is one that cannot be rescued.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStageOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			Self::transition(stage);
			Ok(())
		}

		/// The Coretime chain confirms that it can receive migrated state.
		///
		/// Sent by `pallet-ct-migrator` in response to [`CtMigratorCall::StartMigration`]. Until
		/// it arrives nothing is drained, so a Coretime chain that never upgraded cannot be
		/// migrated into.
		#[pallet::call_index(2)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn ct_ready(origin: OriginFor<T>) -> DispatchResult {
			T::CtOrigin::ensure_origin(origin)?;
			ensure!(
				RcMigrationStage::<T>::get() == MigrationStage::WaitingForCt,
				Error::<T>::NotWaitingForCt
			);

			// Readiness admits the machine to the first data stage. With none implemented, that
			// is the cool-off.
			let end_at = frame_system::Pallet::<T>::block_number() + T::CoolOffPeriod::get();
			Self::transition(MigrationStage::CoolOff { end_at });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// One block of the stage machine.
		///
		/// A stage whose XCM send fails is left in place, so the same step is retried next block.
		/// That is the whole retry story on purpose: XCM delivery is assumed reliable, and the
		/// cases it does not cover are `force_set_stage`'s.
		fn progress_migration(now: BlockNumberFor<T>) -> Weight {
			match RcMigrationStage::<T>::get() {
				MigrationStage::Scheduled { start } if now >= start => {
					if Self::send_to_ct(CtMigratorCall::StartMigration).is_ok() {
						Self::transition(MigrationStage::WaitingForCt);
					}
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::CoolOff { end_at } if now >= end_at => {
					if Self::send_to_ct(CtMigratorCall::FinishMigration).is_ok() {
						Self::transition(MigrationStage::MigrationDone);
					}
					T::DbWeight::get().reads_writes(1, 1)
				},
				_ => T::DbWeight::get().reads(1),
			}
		}

		pub(crate) fn transition(new: MigrationStageOf<T>) {
			let old = RcMigrationStage::<T>::get();
			RcMigrationStage::<T>::put(new.clone());
			log::info!(target: LOG_TARGET, "Stage transition: {old:?} -> {new:?}");
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Send a `pallet-ct-migrator` call to the Coretime chain.
		fn send_to_ct(call: CtMigratorCall) -> Result<(), Error<T>> {
			let call = CtRuntimeCall::CtMigrator(call);
			// `Superuser` converts to Root on the Coretime chain, which system chains grant the
			// relay-chain location; the receiving calls check for Root.
			let message = Xcm(vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Superuser,
					fallback_max_weight: None,
					call: call.encode().into(),
				},
			]);

			let dest = Location::new(0, [Parachain(T::CtParaId::get())]);
			send_xcm::<T::SendXcm>(dest, message).map_err(|e| {
				log::error!(target: LOG_TARGET, "Sending to CT failed: {e:?}");
				Error::<T>::XcmSendFailed
			})?;
			Ok(())
		}
	}
}
