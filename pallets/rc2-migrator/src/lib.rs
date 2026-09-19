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

//! The operational pallet for the Relay Chain, designed to manage and facilitate the migration of
//! the parachain registrar, HRMP and the accounts holding their deposits from the Relay Chain to
//! the Coretime chain. This pallet works alongside its counterpart, `pallet_ct_migrator`, which
//! handles migration processes on the Coretime chain side.
//!
//! This pallet is responsible for controlling the initiation, progression, and completion of the
//! migration process, including managing its various stages and transferring the necessary data.
//! The pallet directly accesses the storage of other pallets for read/write operations while
//! maintaining compatibility with their existing APIs.
//!
//! Every `TODO(ahm-v2)` here and in `pallet-ct-migrator` is work this migration needs before it
//! runs for real. Go through all of them before release.
//!
//! This pallet follows `pallet_rc_migrator` (removed in polkadot-fellows/runtimes#1016, readable at
//! <https://github.com/polkadot-fellows/runtimes/tree/985df25829b3385730ff66acc50161ac57f0692c/pallets/rc-migrator>).

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
	sp_runtime::traits::Saturating,
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

/// The migration stage of the Relay Chain. Advanced by `on_initialize`, except where noted, and
/// only while [`Paused`] is clear.
///
/// Variants are in the order the migration progresses through them.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub enum MigrationStage<AccountId, BlockNumber, Moment> {
	/// The migration has not yet started but will start in the future.
	#[default]
	Pending,
	/// The migration has been scheduled to start at the given moment.
	Scheduled {
		/// The wall-clock time at which the migration will start.
		///
		/// The moment at which we notify the Coretime chain about the start of the migration and
		/// move to `WaitingForCt` stage. After we receive the confirmation, the Relay Chain will
		/// enter the `WarmUp` stage and wait for the warm-up period to end (`WarmUpPeriod`)
		/// before starting to send the migration data to the Coretime chain.
		start: Moment,
	},
	/// The migration is waiting for confirmation from the Coretime chain to go ahead.
	///
	/// This stage involves waiting for the notification from the Coretime chain that it is ready
	/// to receive the migration data.
	WaitingForCt,
	WarmUp {
		/// The block number at which the warm-up period will end.
		///
		/// After the warm-up period ends, the Relay Chain will start to send the migration data
		/// to the Coretime chain.
		end_at: BlockNumber,
	},
	/// Initializing the account migration process.
	AccountsInit,
	/// Migrating account balances, their reserves, and the holds those reserves become.
	AccountsOngoing {
		/// Last migrated account.
		last_key: Option<AccountId>,
	},
	/// Note that the `*Done` stages do not have any logic attached to themselves. They exist to
	/// make it easier to swap out what stage should run next for testing, and as a clean
	/// `force_set_stage` target for rewinding a single stage.
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
	/// Empty the pots whose balance has no owning account to migrate it with, such as the
	/// treasury's.
	Sweep,
	/// Reap the accounts left below the existential deposit, and the zero-balance husks.
	///
	/// Runs after the accounts stage because most of what it reaps does not exist until the
	/// earlier stages have run, and because [`Self::TiCorrection`] reads its output: burning the
	/// issuance no account holds is only safe once the husks are gone.
	SweepDust {
		last_key: Option<AccountId>,
	},
	/// Burn the audited issuance that no account holds.
	TiCorrection,
	CoolOff {
		/// The block number at which the post migration cool-off period will end.
		end_at: BlockNumber,
	},
	/// The migration is done.
	MigrationDone,
}

impl<AccountId, BlockNumber, Moment> MigrationStage<AccountId, BlockNumber, Moment> {
	/// Whether the migration is finished.
	///
	/// This is not the same as `!self.is_ongoing()` since it may not have started.
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	/// Whether the migration is ongoing.
	///
	/// This is not the same as `!self.is_finished()` since it may not have started.
	pub fn is_ongoing(&self) -> bool {
		!matches!(self, Self::Pending | Self::Scheduled { .. } | Self::MigrationDone)
	}

	/// Whether the machine has left [`Self::Pending`]/[`Self::Scheduled`]. Stays true after
	/// [`Self::MigrationDone`].
	pub fn has_started(&self) -> bool {
		self.is_ongoing() || self.is_finished()
	}
}

/// `CtMigrator`'s pallet index in the Coretime (receiver) chain.
pub const CT_MIGRATOR_PALLET_INDEX: u8 = 100;

/// Call encoding for the Coretime chain runtime, reduced to the pallet this chain dispatches into.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CtRuntimeCall {
	CtMigrator(CtMigratorCall) = CT_MIGRATOR_PALLET_INDEX,
}

/// Call encoding for the calls needed from the ct-migrator pallet.
///
/// Indices are the `#[pallet::call_index]`es in `pallet-ct-migrator`.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum CtMigratorCall {
	#[codec(index = 0)]
	StartMigration,
	#[codec(index = 1)]
	EndLockdown,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Send DMP message.
		type SendXcm: SendXcm;

		/// Para id of the Coretime chain.
		type CtParaId: Get<u32>;

		/// Wall clock that [`MigrationStage::Scheduled`] is compared against, so a schedule set
		/// weeks ahead does not drift with block times.
		type TimeProvider: Time;

		/// The origin of the Coretime chain's messages on this chain.
		type CtOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// The origin that can perform permissioned operations like setting the migration stage.
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// The Relay Chain migration state.
	#[pallet::storage]
	#[pallet::unbounded]
	pub type RcMigrationStage<T: Config> = StorageValue<_, MigrationStageOf<T>, ValueQuery>;

	/// The duration of the pre migration warm-up period.
	///
	/// This is the duration of the warm-up period before the data migration starts. During this
	/// period, the migration will be in ongoing state and the concerned extrinsics will be locked.
	#[pallet::storage]
	pub type WarmUpPeriod<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// The duration of the post migration cool-off period.
	///
	/// This is the duration of the cool-off period after the data migration is finished. During
	/// this period, the migration will be still in ongoing state and the concerned extrinsics will
	/// be locked.
	#[pallet::storage]
	pub type CoolOffPeriod<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// An optional account id of a manager.
	///
	/// This account id has similar privileges to [`Config::AdminOrigin`] except that it
	/// can not set the manager account id via `set_manager` call.
	#[pallet::storage]
	pub type Manager<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Whether the migration is paused.
	///
	/// The stage is untouched, so the migration still counts as ongoing. While paused the machine
	/// may be repositioned with `force_set_stage`, and `resume_migration` continues from whatever
	/// stage it then holds. Inbound signals such as `ct_ready` are still recorded; only
	/// `on_initialize` stands still.
	///
	/// Different from v1's `MigrationStage::MigrationPaused` variant: an independent flag, so the
	/// stage paused at is kept.
	#[pallet::storage]
	pub type Paused<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The migration can only be scheduled while it is pending.
		AlreadyScheduled,
		/// Indicates that the specified start moment is in the past.
		StartInPast,
		/// Readiness was confirmed while the machine was not waiting for it.
		NotWaitingForCt,
		/// Failed to send XCM message.
		XcmSendFailed,
		/// The migration can only be cancelled while it is scheduled.
		NotScheduled,
		/// The account is referenced by some other pallet. It might have freezes or holds.
		AccountReferenced,
		/// The migration can only be paused while it is running.
		NotRunning,
		/// The migration is already paused.
		AlreadyPaused,
		/// The migration is not paused.
		NotPaused,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A stage transition has occurred.
		StageTransition {
			/// The old stage before the transition.
			old: MigrationStageOf<T>,
			/// The new stage after the transition.
			new: MigrationStageOf<T>,
		},
		/// The manager account id was set.
		ManagerSet {
			/// The old manager account id.
			old: Option<T::AccountId>,
			/// The new manager account id.
			new: Option<T::AccountId>,
		},
		/// The migration was paused.
		MigrationPaused {
			/// The stage at which the migration was paused.
			stage: MigrationStageOf<T>,
		},
		/// The migration was resumed.
		MigrationResumed {
			/// The stage from which the migration continues.
			stage: MigrationStageOf<T>,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			Self::progress_migration(now)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Schedule the migration to start at a given moment.
		///
		/// ### Parameters:
		/// - `start`: The wall-clock time at which the migration will start.
		/// - `warm_up`: Duration in blocks used to prepare for the migration. Calls are filtered
		///   during this period. It is intended to give enough time for UMP and DMP queues to
		///   empty. Counted from the transition to the warm-up stage.
		/// - `cool_off`: Duration in blocks of the post migration cool-off period. Counted from the
		///   transition to the cool-off stage.
		///
		/// Read [`MigrationStage::Scheduled`] documentation for more details.
		#[pallet::call_index(0)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 3))]
		pub fn schedule_migration(
			origin: OriginFor<T>,
			start: MomentOf<T>,
			warm_up: BlockNumberFor<T>,
			cool_off: BlockNumberFor<T>,
		) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			ensure!(
				RcMigrationStage::<T>::get() == MigrationStage::Pending,
				Error::<T>::AlreadyScheduled
			);
			ensure!(start > T::TimeProvider::now(), Error::<T>::StartInPast);

			WarmUpPeriod::<T>::put(warm_up);
			CoolOffPeriod::<T>::put(cool_off);
			Self::transition(MigrationStage::Scheduled { start });
			Ok(())
		}

		/// Set the migration stage.
		///
		/// This call is intended for emergency use only and is guarded by the
		/// [`Config::AdminOrigin`] or the [`Manager`]. Unlike v1 it is only accepted while
		/// [`Paused`]: pause, force, then resume.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStageOf<T>) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			ensure!(Paused::<T>::get(), Error::<T>::NotPaused);

			Self::transition(stage);
			Ok(())
		}

		/// Start the data migration.
		///
		/// This is typically called by the Coretime chain to indicate its readiness to receive the
		/// migration data, in response to [`CtMigratorCall::StartMigration`]. The admin origin and
		/// the [`Manager`] may call it too, to stand in for a reply that never arrived. A repeat
		/// during the warm-up is accepted and changes nothing.
		#[pallet::call_index(2)]
		#[pallet::weight(T::DbWeight::get().reads_writes(3, 1))]
		pub fn ct_ready(origin: OriginFor<T>) -> DispatchResult {
			if T::CtOrigin::ensure_origin(origin.clone()).is_err() {
				Self::ensure_admin_or_manager(origin)?;
			}

			match RcMigrationStage::<T>::get() {
				MigrationStage::WaitingForCt => {
					let end_at = frame_system::Pallet::<T>::block_number()
						.saturating_add(WarmUpPeriod::<T>::get());
					Self::transition(MigrationStage::WarmUp { end_at });
				},
				// A repeated confirmation during the warm-up is accepted and changes nothing; one
				// at any other stage is an error.
				MigrationStage::WarmUp { .. } => (),
				_ => return Err(Error::<T>::NotWaitingForCt.into()),
			}
			Ok(())
		}

		/// Cancel the migration.
		///
		/// Migration can only be cancelled if it is in the [`MigrationStage::Scheduled`] state, so
		/// the Coretime chain has not been told anything yet.
		#[pallet::call_index(3)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn cancel_migration(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			ensure!(
				matches!(RcMigrationStage::<T>::get(), MigrationStage::Scheduled { .. }),
				Error::<T>::NotScheduled
			);

			Self::transition(MigrationStage::Pending);
			Ok(())
		}

		/// Set the manager account id.
		///
		/// The manager has the similar to [`Config::AdminOrigin`] privileges except that it
		/// can not set the manager account id via `set_manager` call.
		///
		/// The account must have no consumers references, so that the migration can reap it at
		/// the end.
		#[pallet::call_index(4)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn set_manager(origin: OriginFor<T>, new: Option<T::AccountId>) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			if let Some(ref who) = new {
				ensure!(
					frame_system::Pallet::<T>::consumers(who) == 0,
					Error::<T>::AccountReferenced
				);
				// TODO(ahm-v2): Manager account will be preserved and kept funded until
				// the cool-off reaps it.
			}
			let old = Manager::<T>::get();
			Manager::<T>::set(new.clone());
			Self::deposit_event(Event::ManagerSet { old, new });
			Ok(())
		}

		/// Pause the migration.
		///
		/// The stage machine stands still until [`Pallet::resume_migration`], and may be
		/// repositioned with `force_set_stage` in between. Only an ongoing migration can be
		/// paused; a scheduled one is cancelled instead.
		#[pallet::call_index(5)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn pause_migration(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			let stage = RcMigrationStage::<T>::get();
			ensure!(stage.is_ongoing(), Error::<T>::NotRunning);
			ensure!(!Paused::<T>::get(), Error::<T>::AlreadyPaused);

			Paused::<T>::put(true);
			Self::deposit_event(Event::MigrationPaused { stage });
			Ok(())
		}

		/// Resume a paused migration from its current stage.
		#[pallet::call_index(6)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn resume_migration(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			ensure!(Paused::<T>::get(), Error::<T>::NotPaused);

			Paused::<T>::kill();
			Self::deposit_event(Event::MigrationResumed { stage: RcMigrationStage::<T>::get() });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Ensure that the origin is [`Config::AdminOrigin`] or signed by [`Manager`] account id.
		fn ensure_admin_or_manager(origin: OriginFor<T>) -> DispatchResult {
			// TODO(ahm-v2): allow hardcoded local multisig to act as manager as well.
			if let Ok(who) = ensure_signed(origin.clone()) {
				if Manager::<T>::get().is_some_and(|manager| manager == who) {
					return Ok(());
				}
			}
			T::AdminOrigin::ensure_origin(origin)?;
			Ok(())
		}

		/// Execute one block of the stage machine.
		// TODO(ahm-v2): proper benchmark
		fn progress_migration(now: BlockNumberFor<T>) -> Weight {
			if Paused::<T>::get() {
				return T::DbWeight::get().reads(1);
			}
			match RcMigrationStage::<T>::get() {
				// The scheduled start is compared against the clock, which at `on_initialize` still
				// holds the previous block's timestamp -- so the migration begins on the first
				// block after the one whose timestamp passed `start`.
				// TODO(ahm-v2): lock down here, which is two things. Filter the calls whose
				// state is about to move, and refuse inbound XCM from anyone but the Coretime
				// chain.
				// TODO(ahm-v2): give the Coretime chain's queue priority.
				MigrationStage::Scheduled { start } if T::TimeProvider::now() >= start => {
					if Self::send_to_ct(CtMigratorCall::StartMigration).is_ok() {
						Self::transition(MigrationStage::WaitingForCt);
					}
					T::DbWeight::get().reads_writes(3, 3)
				},
				MigrationStage::WarmUp { end_at } if now >= end_at => {
					Self::transition(MigrationStage::AccountsInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::AccountsInit => {
					Self::transition(MigrationStage::AccountsOngoing { last_key: None });
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::AccountsOngoing { .. } => {
					Self::transition(MigrationStage::AccountsDone);
					T::DbWeight::get().reads_writes(1, 1)
				},
				// The `*Done` stages are one-block checkpoints rather than direct `*Init`
				// transitions: each boundary is a visible `StageTransition` event the migration
				// monitor keys on, and a clean force-set target for rewinding a single stage.
				MigrationStage::AccountsDone => {
					Self::transition(MigrationStage::ProxyInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::ProxyInit => {
					Self::transition(MigrationStage::ProxyOngoing { last_key: None });
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::ProxyOngoing { .. } => {
					Self::transition(MigrationStage::ProxyDone);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::ProxyDone => {
					Self::transition(MigrationStage::RegistrarInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::RegistrarInit => {
					Self::transition(MigrationStage::RegistrarOngoing { last_key: None });
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::RegistrarOngoing { .. } => {
					Self::transition(MigrationStage::RegistrarDone);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::RegistrarDone => {
					Self::transition(MigrationStage::HrmpInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::HrmpInit => {
					Self::transition(MigrationStage::HrmpOngoing { last_key: None });
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::HrmpOngoing { .. } => {
					Self::transition(MigrationStage::HrmpDone);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::HrmpDone => {
					Self::transition(MigrationStage::Sweep);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::Sweep => {
					Self::transition(MigrationStage::SweepDust { last_key: None });
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::SweepDust { .. } => {
					Self::transition(MigrationStage::TiCorrection);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::TiCorrection => {
					Self::transition(MigrationStage::CoolOff {
						end_at: now.saturating_add(CoolOffPeriod::<T>::get()),
					});
					T::DbWeight::get().reads_writes(2, 2)
				},
				// wait cool off period before finishing migration
				MigrationStage::CoolOff { end_at } if now >= end_at => {
					if Self::send_to_ct(CtMigratorCall::EndLockdown).is_ok() {
						Self::transition(MigrationStage::MigrationDone);
					}
					T::DbWeight::get().reads_writes(3, 3)
				},
				_ => T::DbWeight::get().reads(1),
			}
		}

		/// Execute a stage transition and log it.
		pub(crate) fn transition(new: MigrationStageOf<T>) {
			let old = RcMigrationStage::<T>::mutate(|stage| core::mem::replace(stage, new.clone()));
			log::info!(target: LOG_TARGET, "Stage transition: {old:?} -> {new:?}");
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Send a `pallet-ct-migrator` call to the Coretime chain as a single XCM `Transact`.
		fn send_to_ct(call: CtMigratorCall) -> Result<(), Error<T>> {
			let call = CtRuntimeCall::CtMigrator(call);
			// `Superuser` converts to Root on the Coretime chain; the receiving calls check for
			// Root.
			let message = Xcm(vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Superuser,
					fallback_max_weight: None,
					call: call.encode().into(),
				},
				// A call that fails inside `Transact` does not fail the XCM by itself; this makes
				// it fail, so the Coretime chain reports it instead of a success.
				ExpectTransactStatus(MaybeErrorCode::Success),
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
