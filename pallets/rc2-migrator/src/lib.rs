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
//! the counterpart `pallet-ct-migrator` on the Coretime chain and (teleports) to Asset Hub.
//!
//! The machine is inert until governance schedules it.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

use alloc::vec;
use frame_support::{
	pallet_prelude::*,
	traits::{EnsureOrigin, Time},
};
use frame_system::pallet_prelude::*;
use polkadot_parachain_primitives::primitives::{HrmpChannelId, Id as ParaId};
use xcm::prelude::*;

const LOG_TARGET: &str = "runtime::rc2-migrator";

/// Wall-clock type the schedule is expressed in.
pub type MomentOf<T> = <<T as Config>::TimeProvider as Time>::Moment;
pub type MigrationStageOf<T> =
	MigrationStage<<T as frame_system::Config>::AccountId, BlockNumberFor<T>, MomentOf<T>>;

/// Progress of the migration. Advanced by `on_initialize`, except where noted.
///
/// The invariant order is how the migration will progress.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub enum MigrationStage<AccountId, BlockNumber, Moment> {
	/// Nothing has been scheduled; `on_initialize` does no work.
	#[default]
	Pending,
	/// Scheduled to begin at the first block whose predecessor's timestamp is at or past `start`.
	Scheduled {
		start: Moment,
	},
	/// Halts the machine without ending the migration. Entered and left only via
	/// [`Pallet::force_set_stage`].
	Paused,
	/// Start signal sent, confirmation from the Coretime chain not yet received.
	WaitingForCt,
	/// Account balances, their reserves, and the holds those reserves become.
	AccountsInit,
	AccountsOngoing {
		last_key: Option<AccountId>,
	},
	AccountsDone,
	/// Proxy definitions whose permissions have meaning on the Coretime chain.
	///
	/// Runs before the registrar so classification can still read the un-drained `Paras` map.
	ProxyInit,
	ProxyOngoing {
		last_key: Option<AccountId>,
	},
	ProxyDone,
	/// `paras_registrar` records and their deposits.
	RegistrarInit,
	RegistrarOngoing {
		last_key: Option<ParaId>,
	},
	RegistrarDone,
	/// HRMP channels, pending open requests, and their deposits.
	HrmpInit,
	HrmpOngoing {
		last_key: Option<HrmpChannelId>,
	},
	HrmpDone,
	/// Empty the configured leftover pots.
	Sweep,
	/// Reap the accounts left below the existential deposit, and the zero-balance husks.
	///
	/// Separate from, and downstream of, the accounts stage on purpose:
	/// - most of what it reaps does not exist until the earlier stages have run. Draining an
	///   account that a consumer reference forbids reaping leaves a zero-balance shell, and
	///   [`Self::Sweep`] turns every emptied pot into another one.
	/// - [`Self::TiCorrection`] burns the issuance no account holds, which is only a safe
	///   assumption once this has run. It reads this stage's output.
	/// - reaping is idempotent, so this stage can be re-run with `force_set_stage`. The accounts
	///   stage burns balances and sends XCM, so it cannot.
	///
	/// Cursored because the accounts stage skips every below-ED record, so this walks most of the
	/// account map. Both stages exclude sovereign and module accounts by the same prefix list.
	SweepDust {
		last_key: Option<AccountId>,
	},
	/// Burn the audited issuance that no account holds.
	TiCorrection,
	/// All data sent; waiting for manual verification before finishing.
	CoolOff {
		end_at: BlockNumber,
	},
	MigrationDone,
}

impl<AccountId, BlockNumber, Moment> MigrationStage<AccountId, BlockNumber, Moment> {
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	/// Whether the machine is between its start and its end.
	pub fn is_ongoing(&self) -> bool {
		!matches!(self, Self::Pending | Self::Scheduled { .. } | Self::MigrationDone)
	}
}

/// Calls on the Coretime chain, as this chain must encode them.
///
/// The indices are `CtMigrator`'s pallet index in the Coretime `construct_runtime!` and the
/// `#[pallet::call_index]`es in `pallet-ct-migrator`. The compiler checks none of this; the
/// integration tests decode these against the real Coretime `RuntimeCall`, which is why they are
/// public.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum CtRuntimeCall {
	#[codec(index = 100)]
	CtMigrator(CtMigratorCall),
}

/// `CtMigrator`'s pallet index in the Coretime `construct_runtime!`, as encoded above. Each
/// Coretime runtime asserts its real index against this in its own tests.
pub const CT_MIGRATOR_PALLET_INDEX: u8 = 100;

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

		/// Wall clock the schedule is compared against. Governance picks a date, not a block
		/// height, so that a schedule set weeks ahead does not drift with block times.
		type TimeProvider: Time;

		/// The origin the Coretime chain's messages dispatch with here. Only it may confirm
		/// readiness.
		type CtOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// How long the machine parks in [`MigrationStage::CoolOff`] before finishing, so the end
		/// state can be verified first.
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
		/// The only way out of [`MigrationStage::Pending`] other than `force_set_stage`.
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn schedule_migration(origin: OriginFor<T>, start: MomentOf<T>) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(
				RcMigrationStage::<T>::get() == MigrationStage::Pending,
				Error::<T>::AlreadyScheduled
			);
			ensure!(start > T::TimeProvider::now(), Error::<T>::StartInPast);

			Self::transition(MigrationStage::Scheduled { start });
			Ok(())
		}

		/// Set the migration stage directly.
		///
		/// Root-only escape hatch for a lost message or a stage that needs re-running; deliberately
		/// unconstrained.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStageOf<T>) -> DispatchResult {
			ensure_root(origin)?;

			Self::transition(stage);
			Ok(())
		}

		/// The Coretime chain confirms that it can receive migrated state.
		///
		/// Sent by `pallet-ct-migrator` in response to [`CtMigratorCall::StartMigration`]. Nothing
		/// is drained before it arrives.
		#[pallet::call_index(2)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn ct_ready(origin: OriginFor<T>) -> DispatchResult {
			T::CtOrigin::ensure_origin(origin)?;
			ensure!(
				RcMigrationStage::<T>::get() == MigrationStage::WaitingForCt,
				Error::<T>::NotWaitingForCt
			);

			let end_at = frame_system::Pallet::<T>::block_number() + T::CoolOffPeriod::get();
			Self::transition(MigrationStage::CoolOff { end_at });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// One block of the stage machine.
		///
		/// The scheduled start is compared against the clock, which at `on_initialize` still holds
		/// the previous block's timestamp — so the migration begins on the first block *after* the
		/// one whose timestamp passed `start`.
		///
		/// A stage whose XCM send fails is left in place and retried next block. Anything that does
		/// not cover is for `force_set_stage`.
		fn progress_migration(now: BlockNumberFor<T>) -> Weight {
			match RcMigrationStage::<T>::get() {
				MigrationStage::Scheduled { start } if T::TimeProvider::now() >= start => {
					if Self::send_to_ct(CtMigratorCall::StartMigration).is_ok() {
						Self::transition(MigrationStage::WaitingForCt);
					}
					T::DbWeight::get().reads_writes(3, 3)
				},
				MigrationStage::CoolOff { end_at } if now >= end_at => {
					if Self::send_to_ct(CtMigratorCall::FinishMigration).is_ok() {
						Self::transition(MigrationStage::MigrationDone);
					}
					T::DbWeight::get().reads_writes(3, 3)
				},
				_ => T::DbWeight::get().reads(1),
			}
		}

		pub(crate) fn transition(new: MigrationStageOf<T>) {
			let old = RcMigrationStage::<T>::mutate(|stage| core::mem::replace(stage, new.clone()));
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
