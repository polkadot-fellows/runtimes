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

//! Relay-chain side of the registrar + HRMP migration to the Coretime chain.
//!
//! Drives the migration stage machine: drains account balances and legacy `paras_registrar` and
//! `hrmp` state together with their deposits and sends everything to the counterpart
//! `pallet-ct-migrator` over XCM. Temporary pallet; removed once the migration is complete.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod accounts;
pub mod hrmp;
pub mod proxy;
pub mod registrar;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use pallet::*;

use alloc::{vec, vec::Vec};
use frame_support::{
	defensive,
	pallet_prelude::*,
	traits::{
		fungible::{Inspect, Mutate, Unbalanced},
		EnsureOrigin, ReservableCurrency, Time,
	},
};
use frame_system::pallet_prelude::*;
use migrator_types::{
	with_rollback, PortableAccount, PortableHold, PortableHoldReason, PortableHrmpChannel,
	PortableHrmpRequest, PortableParaInfo, PortableProxy, PortableProxyType,
};
use polkadot_parachain_primitives::primitives::{HrmpChannelId, Id as ParaId};
use polkadot_runtime_common::paras_registrar;
use sp_runtime::AccountId32;
use xcm::prelude::*;

const LOG_TARGET: &str = "runtime::rc2-migrator";

pub type MigrationStageOf<T> =
	MigrationStage<<T as frame_system::Config>::AccountId, BlockNumberFor<T>, MomentOf<T>>;

/// Wall-clock type the schedule is expressed in.
pub type MomentOf<T> = <<T as Config>::TimeProvider as Time>::Moment;

/// Maximum number of accounts packed into one XCM message.
///
/// An encoded [`PortableAccount`] is ~65 bytes, keeping the message far below the DMP size limit.
pub const MAX_ACCOUNTS_PER_XCM: u32 = 100;

/// Maximum number of accounts processed per relay-chain block.
///
/// Also bounds the unbenchmarked work of both this pallet's `on_initialize` and the resulting
/// `receive_accounts` calls on the Coretime chain.
pub const MAX_ACCOUNTS_PER_BLOCK: u32 = 300;

/// Batch and per-block limits for the registrar and HRMP stages. Their record counts are small
/// (dozens to hundreds on Polkadot), so one limit serves both.
pub const MAX_RECORDS_PER_XCM: u32 = 50;
pub const MAX_RECORDS_PER_BLOCK: u32 = 100;

/// Maximum beneficiaries in one teleport message to Asset Hub: one `DepositAsset` instruction
/// each, and an XCM message decodes at most 100 instructions.
pub const MAX_TELEPORTS_PER_XCM: u32 = 40;

/// The expected composition of one account's reserved balance. See `ExpectedReserves`.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct ExpectedReserve {
	/// Continues on the Coretime chain as an `UnnamedReserve` hold: registrar deposits recorded
	/// for the account as manager, HRMP channel and request deposits recorded for it as (child)
	/// para sovereign.
	pub ct: u128,
	/// Continues on the Coretime chain as a `ProxyDeposit` hold (resized when the definitions
	/// arrive): proxy deposits of delegators with at least one portable definition.
	pub proxy: u128,
	/// Released and teleported to Asset Hub as free balance: deposits whose purpose ends with
	/// this chain (untranslatable proxy sets, multisig operations, announcements).
	pub refund: u128,
}

/// Progress of the migration. Advanced by `on_initialize`.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub enum MigrationStage<AccountId, BlockNumber, Moment> {
	#[default]
	Pending,
	/// Scheduled to begin at the first block whose predecessor's timestamp is at or past
	/// `start`. A wall clock, so a schedule set weeks ahead does not drift with block times.
	Scheduled {
		start: Moment,
	},
	/// Halts the machine while keeping the migration "ongoing" (call filters stay engaged).
	/// Entered and left only via `force_set_stage`.
	Paused,
	/// Waiting for the Coretime chain to confirm that it is ready to receive data.
	WaitingForCt,
	/// Both chains are locked down and nothing has moved yet: the window for the message queues
	/// to drain and for an operator to halt the migration before any data is sent.
	WarmUp {
		end_at: BlockNumber,
	},
	AccountsInit,
	AccountsOngoing {
		last_key: Option<AccountId>,
	},
	AccountsDone,
	/// Migrates portable proxy definitions to the Coretime chain.
	ProxyInit,
	ProxyOngoing {
		last_key: Option<AccountId>,
	},
	ProxyDone,
	RegistrarInit,
	RegistrarOngoing {
		last_key: Option<ParaId>,
	},
	RegistrarDone,
	HrmpInit,
	HrmpOngoing {
		last_key: Option<HrmpChannelId>,
	},
	HrmpDone,
	/// Empty the configured leftover pots (old treasury, …); the proceeds teleport to
	/// `Config::SweepBeneficiary` on Asset Hub.
	Sweep,
	/// Reap below-ED dust and stale husks, cursored like the data stages — the accounts stage
	/// deliberately leaves every below-ED record behind, so this walks most of the account map.
	SweepDust {
		last_key: Option<AccountId>,
	},
	/// Burn the audited amount of issuance that no account holds (see `Config::TiCorrection`).
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

	pub fn is_ongoing(&self) -> bool {
		!matches!(self, Self::Pending | Self::Scheduled { .. } | Self::MigrationDone)
	}

	/// Whether the migration has begun, and so whether the calls it moves away are closed.
	///
	/// A scheduled migration has not begun: the relay chain serves its users normally right up to
	/// the start block, which is what stops the runtime upgrade itself from being an outage.
	pub fn has_started(&self) -> bool {
		self.is_ongoing() || self.is_finished()
	}
}

/// Payload of a `Transact` sent to the Coretime chain.
///
/// Manual call encoding: the enum indices must match `CtMigrator`'s pallet index in the Coretime
/// `construct_runtime` and the `#[pallet::call_index]` attributes in `pallet-ct-migrator`. The
/// integration test decodes every sent `Transact` with the real Coretime `RuntimeCall`, which
/// catches drift. `Decode` exists so tests can assert on captured messages.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum CtRuntimeCall {
	#[codec(index = 100)]
	CtMigrator(CtMigratorCall),
}

#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum CtMigratorCall {
	#[codec(index = 0)]
	ReceiveAccounts { accounts: Vec<PortableAccount<AccountId32, u128>> },
	#[codec(index = 1)]
	ReceiveRegistrar {
		paras: Vec<PortableParaInfo<AccountId32, u128>>,
		next_free_para_id: Option<u32>,
	},
	#[codec(index = 2)]
	ReceiveHrmp { channels: Vec<PortableHrmpChannel<u128>> },
	#[codec(index = 3)]
	ReconcileBalances { rc_kept: u128, rc_migrated: u128 },
	#[codec(index = 4)]
	ReceiveProxies { proxies: Vec<PortableProxy<AccountId32>> },
	#[codec(index = 5)]
	ReceiveHrmpRequests { requests: Vec<PortableHrmpRequest<u128>> },
	#[codec(index = 6)]
	StartMigration,
	#[codec(index = 7)]
	EndLockdown,
}

/// Balance conservation bookkeeping for the migration.
///
/// `kept + ct_reserved + ct_free + ah_free` must always equal the relay-chain total issuance
/// recorded when the accounts stage started; the invariant checks assert against this.
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
pub struct MigratedBalances<Balance> {
	/// Balance that remains on the relay chain.
	pub kept: Balance,
	/// Deposits burned here and re-established as holds on the Coretime chain.
	pub ct_reserved: Balance,
	/// Free working buffer burned here and minted liquid on the Coretime chain.
	pub ct_free: Balance,
	/// Free balance burned here and teleported to Asset Hub.
	pub ah_free: Balance,
	/// Phantom issuance burned by the `TiCorrection` stage (issuance no account held).
	pub ti_corrected: Balance,
}

impl MigratedBalances<u128> {
	/// Everything that went to the Coretime chain; what `reconcile_balances` reconciles against.
	pub fn migrated_ct(&self) -> u128 {
		self.ct_reserved.saturating_add(self.ct_free)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config:
		frame_system::Config<
			AccountId = AccountId32,
			AccountData = pallet_balances::AccountData<u128>,
		> + pallet_balances::Config<Balance = u128>
		// The `Currency` equalities pin the deposit balance types to u128; the `ProxyType`
		// bound is where the runtime declares which proxy permissions travel to the Coretime
		// chain (untranslatable ones stay here). Multisig is bound only to index its deposits:
		// the one pallet whose calls stay open pre-migration, so new deposits can still appear.
		+ paras_registrar::Config<Currency = pallet_balances::Pallet<Self>>
		+ runtime_parachains::hrmp::Config
		+ pallet_multisig::Config<Currency = pallet_balances::Pallet<Self>>
		+ pallet_proxy::Config<
			Currency = pallet_balances::Pallet<Self>,
			ProxyType: TryInto<PortableProxyType>,
		>
		// Preimage deposits are released before the accounts stage runs: they are named holds,
		// and `can_migrate` refuses any account that has one. The bound is on the pallet rather
		// than a `StorePreimage` seam because the deposits have to be *enumerated*, which only
		// the pallet's storage can do. See `release_preimage_deposits`.
		+ pallet_preimage::Config
	{
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Native currency.
		type Currency: Mutate<Self::AccountId, Balance = u128>
			+ ReservableCurrency<Self::AccountId, Balance = u128>;

		/// Router for XCM messages to the Coretime chain.
		type SendXcm: SendXcm;

		/// Para id of the Coretime chain.
		type CtParaId: Get<u32>;

		/// Para id of Asset Hub, the destination of teleported free balances.
		type AhParaId: Get<u32>;

		/// Working buffer of free balance that follows a migrated deposit to the Coretime chain,
		/// so deposit owners can pay fees and future deposits there without a teleport first.
		type CtFreeBuffer: Get<u128>;

		/// Asset Hub's existential deposit. Free balance below this cannot be teleported into a
		/// fresh account; such dust follows the deposit to the Coretime chain instead.
		type AhExistentialDeposit: Get<u128>;

		/// Leftover module pots to empty in the `Sweep` stage (e.g. the old treasury pot).
		/// Their full balance teleports to `SweepBeneficiary`.
		type SweepAccounts: Get<Vec<AccountId32>>;

		/// Where swept pots and reaped dust land on Asset Hub — the treasury / DAP buffer
		/// account designated by governance.
		type SweepBeneficiary: Get<AccountId32>;

		/// The audited amount of total issuance that no account holds ("phantom issuance"),
		/// burned by the `TiCorrection` stage at the end of the migration.
		///
		/// Governance-legible tunable: measured off-chain ahead of the migration
		/// (`balance_census` prints the exact planck value) and pinned here. The stage burns
		/// `min(this, measured-on-chain)` — anything unaccounted beyond it is left for
		/// investigation, and a measured value *below* it is reported as an anomaly; the stage
		/// never burns issuance that an account actually holds.
		type TiCorrection: Get<u128>;

		/// Wall clock the schedule is compared against, so a schedule set weeks ahead does not
		/// drift with block times.
		type TimeProvider: Time;

		/// The origin that the Coretime chain's messages dispatch with on this chain.
		type CtOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

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

	/// Balance kept on the relay chain versus migrated away. Set up by the accounts stage.
	#[pallet::storage]
	pub type RcMigratedBalance<T: Config> = StorageValue<_, MigratedBalances<u128>, ValueQuery>;

	/// What each account's reserved balance is expected to be made of, built by `AccountsInit`
	/// from the owning pallets' recorded deposit fields. The recorded fields are the routing
	/// source of truth; the anonymous reserves are only trusted up to these amounts, and anything
	/// beyond them travels as an unattributed hold, parked at the destination.
	///
	/// One record per account rather than one map per kind: the three amounts are always built
	/// together and always read together in the withdrawal split.
	#[pallet::storage]
	pub type ExpectedReserves<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, ExpectedReserve, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Sending an XCM message to the Coretime chain failed.
		XcmSendFailed,
		/// The account balance could not be fully withdrawn.
		FailedToWithdrawAccount,
		/// The migrated/kept balance bookkeeping would overflow.
		BalanceAccounting,
		/// The migration can only be scheduled while it is pending.
		AlreadyScheduled,
		/// The migration cannot be scheduled to start in the past.
		StartInPast,
		/// Readiness was confirmed while the machine was not waiting for it.
		NotWaitingForCt,
		/// The migration can only be cancelled while it is scheduled.
		NotScheduled,
		/// An account that is referenced cannot be appointed manager.
		AccountReferenced,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		StageTransition {
			old: MigrationStageOf<T>,
			new: MigrationStageOf<T>,
		},
		/// The manager account was appointed or removed.
		ManagerSet {
			old: Option<T::AccountId>,
			new: Option<T::AccountId>,
		},
		/// A batch of withdrawn accounts was sent to the Coretime chain.
		AccountsBatchSent {
			count: u32,
		},
		/// A batch of free balances was teleported to Asset Hub.
		AccountsTeleported {
			count: u32,
			amount: u128,
		},
		/// An account carried reserve that no pallet's deposit records account for. It travels to
		/// the Coretime chain under its own hold reason and stays parked there for investigation.
		UnattributedReserve {
			who: AccountId32,
			amount: u128,
		},
		/// A proxy deposit was released and refunded: it travels to Asset Hub as free balance.
		DepositRefunded {
			who: AccountId32,
			amount: u128,
		},
		/// A batch of portable proxy sets was sent to the Coretime chain.
		ProxyBatchSent {
			count: u32,
		},
		/// Pending HRMP open-channel requests were sent to the Coretime chain.
		HrmpRequestsSent {
			count: u32,
		},
		/// A leftover pot was emptied; its balance teleports to the sweep beneficiary on AH.
		AccountSwept {
			who: AccountId32,
			amount: u128,
		},
		/// Below-ED dust accounts were reaped; the sum teleports to the sweep beneficiary.
		DustSwept {
			count: u32,
			amount: u128,
		},
		/// Phantom issuance burned: `burned = min(expected, unaccounted)`. Any
		/// `unaccounted - burned` remainder is left on the books for investigation.
		TiCorrected {
			expected: u128,
			unaccounted: u128,
			burned: u128,
		},
		/// The measured unaccounted issuance was BELOW the audited expectation — the phantom
		/// shrank since it was measured, which no known mechanism explains. Observability only;
		/// the correction still burned the measured amount.
		TiCorrectionAnomaly {
			expected: u128,
			unaccounted: u128,
		},
		/// A batch of drained registrar records was sent to the Coretime chain.
		RegistrarBatchSent {
			count: u32,
		},
		/// A batch of drained HRMP channel records was sent to the Coretime chain.
		HrmpBatchSent {
			count: u32,
		},
		/// An account that must survive (a consumer reference forbids reaping — session keys
		/// being the known case) was drained to a zero-balance shell; the balance travels like
		/// any other account's.
		AccountShellDrained {
			who: AccountId32,
			amount: u128,
		},
		/// Zero-balance records held alive only by stale provider references were reaped.
		HusksReaped {
			count: u32,
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
		/// Recovery hook for a lost message or a stage that needs re-running; the stage machine
		/// normally advances itself in `on_initialize`.
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
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
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
				MigrationStage::WarmUp { end_at } if now >= end_at => {
					Self::transition(MigrationStage::AccountsInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::AccountsInit => {
					// Same rollback-and-retry discipline as every other one-shot stage, so a
					// fallible step added here later inherits it instead of silently lacking it.
					Self::migrate_stage_once(
						|| {
							// Before anything is measured: drop every preimage deposit, so
							// accounts that hold one are not skipped by `can_migrate`.
							accounts::AccountsMigrator::<T>::release_preimage_deposits();
							let total_issuance = <T as Config>::Currency::total_issuance();
							RcMigratedBalance::<T>::put(MigratedBalances {
								kept: total_issuance,
								..Default::default()
							});
							let indexed =
								accounts::AccountsMigrator::<T>::build_expected_reserves();
							log::info!(
								target: LOG_TARGET,
								"Indexed expected reserves from {indexed} records"
							);
							Ok(())
						},
						MigrationStage::AccountsOngoing { last_key: None },
					);
					// Full scans of the deposit-owning maps plus the preimage sweep.
					Self::placeholder_weight(MAX_RECORDS_PER_BLOCK)
				},
				MigrationStage::AccountsOngoing { last_key } => {
					// All of this block's withdrawals commit or roll back together, so a failed
					// XCM send cannot leave balances burned but never sent.
					Self::migrate_stage_step(
						|| accounts::AccountsMigrator::<T>::migrate_many(last_key),
						MigrationStage::AccountsDone,
						|last_key| MigrationStage::AccountsOngoing { last_key: Some(last_key) },
					);
					Self::placeholder_weight(MAX_ACCOUNTS_PER_BLOCK)
				},
				// The `*Done` stages are one-block checkpoints rather than direct `*Init`
				// transitions: each boundary is a visible `StageTransition` event the migration
				// monitor keys on, and a clean force-set target for rewinding a single stage.
				MigrationStage::AccountsDone => {
					Self::transition(MigrationStage::ProxyInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::ProxyInit => {
					Self::migrate_stage_once(
						proxy::ProxyMigrator::<T>::drain_announcements,
						MigrationStage::ProxyOngoing { last_key: None },
					);
					T::DbWeight::get().reads_writes(100, 100)
				},
				MigrationStage::ProxyOngoing { last_key } => {
					Self::migrate_stage_step(
						|| proxy::ProxyMigrator::<T>::migrate_many(last_key),
						MigrationStage::ProxyDone,
						|last_key| MigrationStage::ProxyOngoing { last_key: Some(last_key) },
					);
					Self::placeholder_weight(MAX_RECORDS_PER_BLOCK)
				},
				MigrationStage::ProxyDone => {
					Self::transition(MigrationStage::RegistrarInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::RegistrarInit => {
					Self::migrate_stage_once(
						registrar::RegistrarMigrator::<T>::migrate_init,
						MigrationStage::RegistrarOngoing { last_key: None },
					);
					T::DbWeight::get().reads_writes(2, 2)
				},
				MigrationStage::RegistrarOngoing { last_key } => {
					Self::migrate_stage_step(
						|| registrar::RegistrarMigrator::<T>::migrate_many(last_key),
						MigrationStage::RegistrarDone,
						|last_key| MigrationStage::RegistrarOngoing { last_key: Some(last_key) },
					);
					Self::placeholder_weight(MAX_RECORDS_PER_BLOCK)
				},
				MigrationStage::RegistrarDone => {
					Self::transition(MigrationStage::HrmpInit);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::HrmpInit => {
					Self::migrate_stage_once(
						hrmp::HrmpMigrator::<T>::copy_open_requests,
						MigrationStage::HrmpOngoing { last_key: None },
					);
					T::DbWeight::get().reads_writes(200, 200)
				},
				MigrationStage::HrmpOngoing { last_key } => {
					Self::migrate_stage_step(
						|| hrmp::HrmpMigrator::<T>::migrate_many(last_key),
						MigrationStage::HrmpDone,
						|last_key| MigrationStage::HrmpOngoing { last_key: Some(last_key) },
					);
					Self::placeholder_weight(MAX_RECORDS_PER_BLOCK)
				},
				MigrationStage::HrmpDone => {
					Self::transition(MigrationStage::Sweep);
					T::DbWeight::get().reads_writes(1, 1)
				},
				MigrationStage::Sweep => {
					Self::migrate_stage_once(
						Self::sweep_pots,
						MigrationStage::SweepDust { last_key: None },
					);
					T::DbWeight::get().reads_writes(10, 10)
				},
				MigrationStage::SweepDust { last_key } => {
					Self::migrate_stage_step(
						|| Self::sweep_dust(last_key),
						MigrationStage::TiCorrection,
						|last_key| MigrationStage::SweepDust { last_key: Some(last_key) },
					);
					Self::placeholder_weight(MAX_ACCOUNTS_PER_BLOCK)
				},
				MigrationStage::TiCorrection => {
					Self::migrate_stage_once(
						Self::correct_total_issuance,
						MigrationStage::CoolOff { end_at: now + CoolOffPeriod::<T>::get() },
					);
					T::DbWeight::get().reads_writes(10, 10)
				},
				// The Coretime chain holds its lockdown until this signal: the relay chain owns
				// the lifecycle, and the two chains share no clock to hold to on their own.
				MigrationStage::CoolOff { end_at } if now >= end_at => {
					if Self::send_to_ct(CtMigratorCall::EndLockdown).is_ok() {
						Self::transition(MigrationStage::MigrationDone);
					}
					T::DbWeight::get().reads_writes(3, 3)
				},
				_ => T::DbWeight::get().reads(1),
			}
		}

		/// Run one block's worth of a cursor-driven data stage inside a storage transaction and
		/// advance the stage machine from the result. Same semantics for every stage: `Ok(None)`
		/// finishes the stage, `Ok(Some(key))` continues from the cursor next block, `Err` rolls
		/// the whole block back and retries the same key range.
		fn migrate_stage_step<K>(
			migrate: impl FnOnce() -> Result<Option<K>, Error<T>>,
			done: MigrationStageOf<T>,
			ongoing: impl FnOnce(K) -> MigrationStageOf<T>,
		) {
			match with_rollback(migrate) {
				Ok(None) => Self::transition(done),
				Ok(Some(last_key)) => Self::transition(ongoing(last_key)),
				Err(e) => {
					// Stage unchanged: the same key range is retried next block.
					defensive!("Data stage failed, retrying: {:?}", e);
				},
			}
		}

		/// Run a one-shot stage inside a storage transaction and advance to `next` on success.
		/// An `Err` rolls all of the stage's writes back and retries it whole next block.
		fn migrate_stage_once(
			work: impl FnOnce() -> Result<(), Error<T>>,
			next: MigrationStageOf<T>,
		) {
			match with_rollback(work) {
				Ok(()) => Self::transition(next),
				Err(e) => {
					defensive!("Stage failed, retrying: {:?}", e);
				},
			}
		}

		/// One block of a record-drain stage: pull `(key, record)` pairs from `iter`, convert and
		/// remove each via `drain` (`Ok(None)` handles a record entirely locally, with nothing to
		/// send; an `Err` aborts the block for the caller's rollback), flush batches of
		/// [`MAX_RECORDS_PER_XCM`] through `send`, and stop after [`MAX_RECORDS_PER_BLOCK`]
		/// records. Returns the cursor to continue from, or `None` once the map is exhausted.
		pub(crate) fn drain_records<K, V, P>(
			mut iter: impl Iterator<Item = (K, V)>,
			mut drain: impl FnMut(&K, V) -> Result<Option<P>, Error<T>>,
			send: impl Fn(Vec<P>) -> Result<(), Error<T>>,
		) -> Result<Option<K>, Error<T>> {
			let mut batch = Vec::new();
			let mut processed = 0u32;
			let maybe_last_key = loop {
				let Some((key, record)) = iter.next() else { break None };
				processed += 1;
				if let Some(payload) = drain(&key, record)? {
					batch.push(payload);
				}

				if batch.len() >= MAX_RECORDS_PER_XCM as usize {
					send(core::mem::take(&mut batch))?;
				}
				if processed >= MAX_RECORDS_PER_BLOCK {
					break Some(key);
				}
			};

			if !batch.is_empty() {
				send(batch)?;
			}
			Ok(maybe_last_key)
		}

		/// Placeholder until the migrator gets benchmarks: a deliberate overestimate of one
		/// block's work on up to `n` items.
		fn placeholder_weight(n: u32) -> Weight {
			T::DbWeight::get().reads_writes((n * 4) as u64, (n * 4) as u64)
		}

		pub(crate) fn transition(new: MigrationStageOf<T>) {
			let old = RcMigrationStage::<T>::get();
			RcMigrationStage::<T>::put(new.clone());
			log::info!(target: LOG_TARGET, "Stage transition: {old:?} -> {new:?}");
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Send a batch of withdrawn accounts to the Coretime chain.
		pub(crate) fn send_accounts(
			accounts: Vec<PortableAccount<AccountId32, u128>>,
		) -> Result<(), Error<T>> {
			let count = accounts.len() as u32;
			Self::send_to_ct(CtMigratorCall::ReceiveAccounts { accounts })?;
			Self::deposit_event(Event::AccountsBatchSent { count });
			Ok(())
		}

		/// Send a batch of drained registrar records to the Coretime chain.
		pub(crate) fn send_registrar(
			paras: Vec<PortableParaInfo<AccountId32, u128>>,
			next_free_para_id: Option<u32>,
		) -> Result<(), Error<T>> {
			let count = paras.len() as u32;
			Self::send_to_ct(CtMigratorCall::ReceiveRegistrar { paras, next_free_para_id })?;
			Self::deposit_event(Event::RegistrarBatchSent { count });
			Ok(())
		}

		/// Send a batch of portable proxy sets to the Coretime chain.
		pub(crate) fn send_proxies(
			proxies: Vec<PortableProxy<AccountId32>>,
		) -> Result<(), Error<T>> {
			let count = proxies.len() as u32;
			Self::send_to_ct(CtMigratorCall::ReceiveProxies { proxies })?;
			Self::deposit_event(Event::ProxyBatchSent { count });
			Ok(())
		}

		/// Send a batch of pending HRMP open-channel requests to the Coretime chain.
		pub(crate) fn send_hrmp_requests(
			requests: Vec<PortableHrmpRequest<u128>>,
		) -> Result<(), Error<T>> {
			Self::send_to_ct(CtMigratorCall::ReceiveHrmpRequests { requests })
		}

		/// Send a batch of drained HRMP channel records to the Coretime chain.
		pub(crate) fn send_hrmp(channels: Vec<PortableHrmpChannel<u128>>) -> Result<(), Error<T>> {
			let count = channels.len() as u32;
			Self::send_to_ct(CtMigratorCall::ReceiveHrmp { channels })?;
			Self::deposit_event(Event::HrmpBatchSent { count });
			Ok(())
		}

		/// Signal to the Coretime chain that all data has been sent.
		pub(crate) fn send_reconciliation(
			rc_kept: u128,
			rc_migrated: u128,
		) -> Result<(), Error<T>> {
			Self::send_to_ct(CtMigratorCall::ReconcileBalances { rc_kept, rc_migrated })
		}

		/// Empty the configured leftover pots; teleport the proceeds to the sweep beneficiary
		/// on Asset Hub. One-shot: the pot list is a short config item.
		fn sweep_pots() -> Result<(), Error<T>> {
			use frame_support::traits::tokens::{Fortitude, Precision, Preservation};
			let mut total: u128 = 0;

			// The configured pots (old treasury etc.): full free balance, no reserves expected.
			for who in T::SweepAccounts::get() {
				let amount = frame_system::Account::<T>::get(&who).data.free;
				if amount == 0 {
					continue;
				}
				let burned = <T as Config>::Currency::burn_from(
					&who,
					amount,
					Preservation::Expendable,
					Precision::Exact,
					Fortitude::Polite,
				)
				.map_err(|_| Error::<T>::FailedToWithdrawAccount)?;
				// Pot balances book-kept as inactive issuance (the treasury) must reactivate on
				// leaving so the issuance accounting stays consistent. Applied to every pot: one
				// that was never deactivated just floors the counter (reactivate saturates)
				// toward its true end state — zero, since no pot survives the sweep.
				<pallet_balances::Pallet<T> as Unbalanced<T::AccountId>>::reactivate(burned);
				total = total.checked_add(burned).ok_or(Error::<T>::BalanceAccounting)?;
				Self::deposit_event(Event::AccountSwept { who, amount: burned });
			}
			Self::book_and_teleport_swept(total)
		}

		/// One block of the dust pass: reap below-ED accounts and stale husks from the cursor
		/// on, teleporting the block's proceeds. Returns the cursor to continue from, or `None`
		/// once the account map is exhausted.
		///
		/// Below the existential deposit nothing meaningful can still be backed by the balance
		/// on a retiring chain, so any hold or reserve is killed — including the consumer
		/// reference the backing carried — and the account zeroed. The fungible APIs refuse to
		/// touch referenced or held-against accounts, hence the direct write (same pattern as
		/// the accounts-stage shell drain). A record survives only while something still
		/// references it (session key-holders being the known case) or it is a module account.
		fn sweep_dust(last_key: Option<T::AccountId>) -> Result<Option<T::AccountId>, Error<T>> {
			let mut iter = match &last_key {
				Some(last_key) => frame_system::Account::<T>::iter_from_key(last_key.clone()),
				None => frame_system::Account::<T>::iter(),
			};

			let (mut dust_count, mut dust_amount, mut husk_count) = (0u32, 0u128, 0u32);
			let ed = <T as Config>::Currency::minimum_balance();
			let mut processed = 0u32;
			let maybe_last_key = loop {
				let Some((who, info)) = iter.next() else { break None };
				processed += 1;
				let cursor = who.clone();

				let d = &info.data;
				let amount = d.free.saturating_add(d.reserved);
				if amount < ed && !accounts::AccountsMigrator::<T>::is_unmigrated(&who) {
					if amount == 0 {
						// A husk: exists only via a stale provider reference. `dec_providers`
						// refuses whenever something still references the account.
						if info.consumers == 0 &&
							frame_system::Pallet::<T>::dec_providers(&who).is_ok()
						{
							husk_count += 1;
						}
					} else {
						let holds = pallet_balances::Holds::<T>::take(&who);
						let backed = !d.reserved.is_zero() || !holds.is_empty();
						frame_system::Account::<T>::mutate(&who, |a| {
							a.data.free = 0;
							a.data.reserved = 0;
						});
						pallet_balances::TotalIssuance::<T>::mutate(|ti| {
							*ti = ti.saturating_sub(amount)
						});
						// Reserves and holds collectively carry one consumer reference; killing
						// the backing drops it, so the record does not survive as a stale-ref
						// shell.
						if backed && info.consumers > 0 {
							frame_system::Pallet::<T>::dec_consumers(&who);
						}
						let _ = frame_system::Pallet::<T>::dec_providers(&who);
						dust_count += 1;
						dust_amount =
							dust_amount.checked_add(amount).ok_or(Error::<T>::BalanceAccounting)?;
					}
				}

				if processed >= MAX_ACCOUNTS_PER_BLOCK {
					break Some(cursor);
				}
			};
			if husk_count > 0 {
				Self::deposit_event(Event::HusksReaped { count: husk_count });
			}
			if dust_count > 0 {
				Self::deposit_event(Event::DustSwept { count: dust_count, amount: dust_amount });
			}
			Self::book_and_teleport_swept(dust_amount)?;
			Ok(maybe_last_key)
		}

		/// Book `total` swept out of the kept balance and teleport it to the sweep beneficiary.
		fn book_and_teleport_swept(total: u128) -> Result<(), Error<T>> {
			if total == 0 {
				return Ok(());
			}
			RcMigratedBalance::<T>::try_mutate(|t| {
				t.kept = t.kept.checked_sub(total).ok_or(Error::<T>::BalanceAccounting)?;
				t.ah_free = t.ah_free.checked_add(total).ok_or(Error::<T>::BalanceAccounting)?;
				Ok::<(), Error<T>>(())
			})?;
			Self::send_teleport(vec![(T::SweepBeneficiary::get(), total)])
		}

		/// Burn the audited phantom issuance and send the finish signal.
		///
		/// By this stage the accounts and sweep stages have drained every account to zero, so
		/// whatever issuance the ledger still counts is held by nobody — no O(accounts) scan is
		/// needed. (Once the migration manager lands it stays funded through `CoolOff` and its
		/// balance must be subtracted here.) Burns `min(expected, measured)` and reports via
		/// events: a remainder above the expectation stays on the books for investigation, a
		/// measurement below it is an explicit anomaly.
		fn correct_total_issuance() -> Result<(), Error<T>> {
			let expected = T::TiCorrection::get();
			let unaccounted = pallet_balances::TotalIssuance::<T>::get();
			let burned = expected.min(unaccounted);

			if unaccounted < expected {
				log::error!(
					target: LOG_TARGET,
					"TI correction anomaly: expected {expected} unaccounted, measured {unaccounted}"
				);
				Self::deposit_event(Event::TiCorrectionAnomaly { expected, unaccounted });
			}

			// No account holds this balance, so there is nothing to burn *from*: the correction
			// is a direct issuance write, mirrored in the migration tracker so the conservation
			// invariant stays exact.
			pallet_balances::TotalIssuance::<T>::put(unaccounted.saturating_sub(burned));
			RcMigratedBalance::<T>::try_mutate(|t| {
				t.kept = t.kept.checked_sub(burned).ok_or(Error::<T>::BalanceAccounting)?;
				t.ti_corrected =
					t.ti_corrected.checked_add(burned).ok_or(Error::<T>::BalanceAccounting)?;
				Ok::<(), Error<T>>(())
			})?;
			Self::deposit_event(Event::TiCorrected { expected, unaccounted, burned });

			let tracker = RcMigratedBalance::<T>::get();
			Self::send_reconciliation(tracker.kept, tracker.migrated_ct())
		}

		/// Teleport a batch of free balances to their owners on Asset Hub.
		///
		/// A real teleport, not a `Transact`: the balances were already burned during withdrawal
		/// (the relay chain does no teleport tracking, `NoTeleportTracking`), and on Asset Hub
		/// each `DepositAsset` moves the amount out of the checking account — the "DOT out on the
		/// relay chain" ledger — so AH issuance stays constant and the ledger drains in lockstep
		/// with the relay chain. No receiving pallet is needed on Asset Hub.
		pub(crate) fn send_teleport(
			beneficiaries: Vec<(AccountId32, u128)>,
		) -> Result<(), Error<T>> {
			let count = beneficiaries.len() as u32;
			let total: u128 = beneficiaries.iter().map(|(_, amount)| amount).sum();
			// From Asset Hub's perspective DOT is the parent's asset.
			let dot = |amount: u128| Asset {
				id: AssetId(Location::parent()),
				fun: Fungibility::Fungible(amount),
			};

			let mut message = vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				ReceiveTeleportedAsset(dot(total).into()),
			];
			for (who, amount) in beneficiaries {
				message.push(DepositAsset {
					assets: AssetFilter::Definite(dot(amount).into()),
					beneficiary: Location::new(
						0,
						[Junction::AccountId32 { network: None, id: who.into() }],
					),
				});
			}

			let dest = Location::new(0, [Parachain(T::AhParaId::get())]);
			send_xcm::<T::SendXcm>(dest, Xcm(message)).map_err(|e| {
				log::error!(target: LOG_TARGET, "Teleport to AH failed: {e:?}");
				Error::<T>::XcmSendFailed
			})?;

			Self::deposit_event(Event::AccountsTeleported { count, amount: total });
			Ok(())
		}

		/// Send a `pallet-ct-migrator` call to the Coretime chain.
		fn send_to_ct(call: CtMigratorCall) -> Result<(), Error<T>> {
			let call = CtRuntimeCall::CtMigrator(call);
			// `Superuser` converts to Root on the Coretime chain, which system chains grant the
			// relay-chain location; the `receive_*` calls check for Root.
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
