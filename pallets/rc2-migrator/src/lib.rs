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
//! Drives the migration stage machine: drains legacy `paras_registrar` and `hrmp` state together
//! with their deposits and sends everything to the counterpart `pallet-ct-migrator` over XCM.
//! Temporary pallet; removed once the migration is complete.
//!
//! The AHM v1 migrators are the reference for the stage machine, the manager and the origins.
//! They were removed in polkadot-fellows/runtimes#1016; read them at
//! `https://github.com/polkadot-fellows/runtimes/tree/985df25829b3385730ff66acc50161ac57f0692c/pallets/rc-migrator`.

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
/// Variants are in the order the migration progresses through them.
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
	/// Waiting for the Coretime chain to confirm that it is ready to receive data.
	WaitingForCt,
	/// Both chains are locked down and nothing has moved yet: the window for the queues to drain
	/// and for an operator to halt the migration before any data is sent.
	WarmUp {
		end_at: BlockNumber,
	},
	// TODO(ahm-v2): every variant from here to `TiCorrection` is declared but not driven --
	// `progress_migration` goes from `WarmUp` straight to `CoolOff`.
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

	/// Whether the machine has left [`Self::Pending`]/[`Self::Scheduled`]. Stays true after
	/// [`Self::MigrationDone`].
	pub fn has_started(&self) -> bool {
		self.is_ongoing() || self.is_finished()
	}
}

/// `CtMigrator`'s pallet index in the Coretime (receiver) chain.
pub const CT_MIGRATOR_PALLET_INDEX: u8 = 100;

/// Calls on the Coretime chain, as this chain must encode them.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CtRuntimeCall {
	CtMigrator(CtMigratorCall) = CT_MIGRATOR_PALLET_INDEX,
}

/// Indices are the `#[pallet::call_index]`es in `pallet-ct-migrator`.
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

		/// Wall clock the schedule is compared against, so a schedule set weeks ahead does not
		/// drift with block times.
		type TimeProvider: Time;

		/// The origin that the Coretime chain's messages dispatch with on this chain.
		type CtOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The origin that may schedule and force the migration.
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::unbounded]
	pub type RcMigrationStage<T: Config> = StorageValue<_, MigrationStageOf<T>, ValueQuery>;

	/// How long [`MigrationStage::WarmUp`] holds, as the scheduled migration set it.
	#[pallet::storage]
	pub type WarmUpPeriod<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// How long [`MigrationStage::CoolOff`] holds, as the scheduled migration set it.
	#[pallet::storage]
	pub type CoolOffPeriod<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	/// An account that may drive the migration alongside [`Config::AdminOrigin`], except that it
	/// cannot appoint a manager itself.
	#[pallet::storage]
	pub type Manager<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

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
		/// The migration can only be cancelled while it is scheduled.
		NotScheduled,
		/// An account that is referenced cannot be appointed manager.
		AccountReferenced,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		StageTransition { old: MigrationStageOf<T>, new: MigrationStageOf<T> },
		ManagerSet { old: Option<T::AccountId>, new: Option<T::AccountId> },
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
		/// `warm_up` is how long both chains stay locked down before any data moves, and
		/// `cool_off` how long they stay locked down after it, for verification.
		///
		/// The only way out of [`MigrationStage::Pending`] other than `force_set_stage`.
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

		/// Set the migration stage directly.
		///
		/// Root-only escape hatch for a lost message or a stage that needs re-running.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStageOf<T>) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;

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

			let end_at = frame_system::Pallet::<T>::block_number() + WarmUpPeriod::<T>::get();
			Self::transition(MigrationStage::WarmUp { end_at });
			Ok(())
		}

		/// Return the machine to [`MigrationStage::Pending`] so it can be rescheduled.
		///
		/// Only valid before the handshake, so the Coretime chain has not been told anything yet.
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

		/// Appoint or remove the [`Manager`].
		///
		/// The account must be unreferenced, so that the migration can reap it at the end.
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
	}

	impl<T: Config> Pallet<T> {
		/// Ensure the origin is [`Config::AdminOrigin`] or signed by the [`Manager`].
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

		/// One block of the stage machine.
		// TODO(ahm-v2): proper benchmark
		fn progress_migration(now: BlockNumberFor<T>) -> Weight {
			match RcMigrationStage::<T>::get() {
				// The scheduled start is compared against the clock, which at `on_initialize` still
				// holds the previous block's timestamp -- so the migration begins on the first
				// block after the one whose timestamp passed `start`.
				// TODO(ahm-v2): start filtering the calls whose state is about to move, so that
				// the warm-up drains queues that nothing is refilling.
				MigrationStage::Scheduled { start } if T::TimeProvider::now() >= start => {
					if Self::send_to_ct(CtMigratorCall::StartMigration).is_ok() {
						Self::transition(MigrationStage::WaitingForCt);
					}
					T::DbWeight::get().reads_writes(3, 3)
				},
				// TODO(ahm-v2): the data stages run from here, once they exist.
				MigrationStage::WarmUp { end_at } if now >= end_at => {
					let end_at = now + CoolOffPeriod::<T>::get();
					Self::transition(MigrationStage::CoolOff { end_at });
					T::DbWeight::get().reads_writes(2, 2)
				},
				// wait cool off period before finishing migration
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
			// `Superuser` converts to Root on the Coretime chain; the receiving calls check for
			// Root.
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
