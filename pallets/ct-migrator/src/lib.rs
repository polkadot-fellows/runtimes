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
//! is accepted (so idempotent), and a signal out of order is an error. The data batches are gated
//! by root alone.
//!
//! The portable payload types exchanged between the migrators live in the shared `migrator-types`
//! crate (re-exported here for convenience), so no runtime depends on another chain's pallets
//! just to speak the wire format.
//!
//! # Differences from AHM v1
//!
//! This pallet follows `pallet_ah_migrator` (refer:
//! <https://github.com/polkadot-fellows/runtimes/tree/985df25829b3385730ff66acc50161ac57f0692c/pallets/ah-migrator>).
//! - The cool-off is held on the Relay Chain. `CoolOff` here only records that every batch has been
//!   reconciled; `end_lockdown` arrives when the Relay Chain's window ends.
//! - No message-count confirmations or queue-priority controls are sent back.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod accounts;
pub mod proxy;

use proxy::PortableProxyOf;

pub use migrator_types::*;
pub use pallet::*;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use accounts::{AccountsReceiver, PortableAccountOf};
use alloc::{vec, vec::Vec};
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
	pallet_prelude::*,
	storage::with_storage_layer,
	traits::{
		fungible::{Inspect, Mutate, MutateHold},
		EnsureOrigin,
	},
	weights::WeightMeter,
};
use frame_system::pallet_prelude::*;
use hrmp_primitives::{MigratedChannel, ReceiveMigratedChannels};
use pallet_message_queue::ForceSetHead;
use registrar_primitives::{MigratedPara, MigratedParaState, ReceiveMigratedParas};
use sp_runtime::traits::{One, Saturating, Zero};
use xcm::prelude::*;

const LOG_TARGET: &str = "runtime::ct-migrator";

pub type PortableParaInfoOf<T> =
	PortableParaInfo<<T as frame_system::Config>::AccountId, BalanceOf<T>>;
pub type PortableHrmpChannelOf<T> = PortableHrmpChannel<BalanceOf<T>>;
pub type PortableHrmpRequestOf<T> = PortableHrmpRequest<BalanceOf<T>>;

/// The migration stage of the Coretime chain. Advanced by messages from `pallet-rc2-migrator`.
///
/// Variants are appended, not inserted, so the encoding of the ones already shipped never moves;
/// read the order of the machine off the transitions, not off the enum.
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
	/// All data received and reconciled; this chain stays locked down until the Relay Chain's
	/// verification window closes.
	CoolOff,
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
		matches!(self, Self::DataMigrationOngoing | Self::CoolOff)
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
	pub trait Config:
		frame_system::Config
		// Migrated proxy delegations are written into the real proxy pallet so keyless (pure)
		// delegators are dispatchable here from day one. The wire format only carries
		// permissions this chain represents, hence the total `From` bound.
		+ pallet_proxy::Config<ProxyType: From<PortableProxyType>>
	{
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Native currency. Migrated balances are minted here; migrated reserves land as holds.
		type Currency: Mutate<Self::AccountId>
			+ MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>;

		/// The overarching hold reason type.
		type RuntimeHoldReason: From<HoldReason>;

		/// How many of this chain's blocks fit in one relay-chain block's time. Used to convert
		/// migrated proxy delays (relay: 6s blocks; this chain: 12s → ratio 2).
		#[pallet::constant]
		type RcBlockTimeRatio: Get<u32>;

		/// Where migrated registrations are handed over. Normally `pallet-registrar-para`.
		///
		/// A seam rather than direct storage writes: which deposit a registration holds in which
		/// state is the receiving pallet's invariant, and rebuilding it out here is how it drifts.
		type RegistrarReceiver: ReceiveMigratedParas<Self::AccountId>;

		/// Where migrated HRMP channels are handed over. Normally `pallet-hrmp-para`.
		type HrmpReceiver: ReceiveMigratedChannels;

		/// Send UMP message.
		type SendXcm: SendXcm;

		/// The origin that can perform permissioned operations like setting the migration stage.
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// The message queue, so the relay chain's downward queue can be put first.
		type MessageQueue: ForceSetHead<AggregateMessageOrigin>;

		/// `(priority_blocks, round_robin_blocks)` for the relay chain's downward queue while the
		/// migration runs; see [`QueuePriority`]. Overridable through [`DmpQueuePriorityConfig`].
		type DmpQueuePriorityPattern: Get<(BlockNumberFor<Self>, BlockNumberFor<Self>)>;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Registrar or HRMP deposit that was reserved on the Relay Chain.
		///
		/// Held under this reason until the owning pallet receives its state and takes its own
		/// deposit out of it.
		#[codec(index = 0)]
		RcMigratedReserve,
		/// A Relay Chain proxy deposit whose definitions travel here. Released when they arrive:
		/// the recreated entry is re-reserved at this chain's rates and the rest becomes free.
		#[codec(index = 1)]
		ProxyDeposit,
		/// Relay Chain reserve that no pallet's deposit records accounted for. Parked here for
		/// investigation. Nothing was allowed to stay behind on the Relay Chain.
		#[codec(index = 2)]
		UnattributedReserve,
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

	/// How the relay chain's downward queue is prioritised while the migration runs.
	#[pallet::storage]
	pub type DmpQueuePriorityConfig<T: Config> =
		StorageValue<_, QueuePriority<BlockNumberFor<T>>, ValueQuery>;

	/// Accounts that failed to integrate, parked verbatim for recovery after the migration.
	///
	/// A batch never fails on a single bad account: it is rolled back, stored here, and the rest
	/// of the batch continues. Each entry is balance the relay chain burned and this chain never
	/// minted, so this map is both the record of the gap and the data needed to close it.
	#[pallet::storage]
	pub type FailedAccounts<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, PortableAccountOf<T>, OptionQuery>;

	/// Total balance minted on this chain by the accounts stage. Reconciled against the relay
	/// chain's burned total once the migration ends.
	#[pallet::storage]
	pub type CtMintedTotal<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	// The migrated registrar and HRMP records are handed straight to the pallets that own them
	// (`Config::RegistrarReceiver` / `Config::HrmpReceiver`) rather than being parked in stand-in
	// storage here. Only records that *fail* to integrate are kept, in the `Failed*` maps below,
	// so nothing is lost and a failure can be worked through once the migration is over.

	/// Registrar records that failed to integrate, parked verbatim for recovery.
	#[pallet::storage]
	pub type FailedParas<T: Config> =
		StorageMap<_, Twox64Concat, u32, PortableParaInfoOf<T>, OptionQuery>;

	/// HRMP channel records that failed to integrate, parked verbatim for recovery.
	#[pallet::storage]
	pub type FailedHrmpChannels<T: Config> =
		StorageMap<_, Twox64Concat, (u32, u32), PortableHrmpChannelOf<T>, OptionQuery>;

	/// Migrated proxy sets that failed to integrate, parked verbatim for recovery.
	#[pallet::storage]
	pub type FailedProxies<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, PortableProxyOf<T>, OptionQuery>;

	/// Per-para shortfall between the registrar-recorded deposit and what actually arrived held.
	///
	/// Reconciliation rule: re-attribute `min(recorded, held)`, park the difference here — the
	/// migration never invents balance for deposit records that were not backed by a reserve on
	/// the relay chain (a known on-chain anomaly).
	#[pallet::storage]
	pub type ParkedDepositShortfalls<T: Config> =
		StorageMap<_, Twox64Concat, u32, BalanceOf<T>, OptionQuery>;

	/// Total released from `RcMigratedReserve` for registrar deposits, so the registrar pallet
	/// could take its own.
	#[pallet::storage]
	pub type ReattributedDeposits<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Total released from `RcMigratedReserve` for HRMP deposits, so the HRMP pallet could take
	/// its own.
	#[pallet::storage]
	pub type ReattributedHrmpDeposits<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Per-(channel, side) shortfall between the recorded HRMP deposit and what arrived held on
	/// the sibling sovereign. Same reconciliation rule as registrar deposits: `min(recorded,
	/// held)`, difference parked here, never invented.
	#[pallet::storage]
	pub type ParkedHrmpShortfalls<T: Config> =
		StorageMap<_, Twox64Concat, (u32, u32, bool), BalanceOf<T>, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The migration has already finished on this chain.
		AlreadyFinished,
		/// The Relay Chain signalled the end of a migration that never started here.
		NotStarted,
		/// Failed to send XCM message.
		XcmSendFailed,
		/// Failed to release a migrated reserve for the pallet owning the deposit.
		FailedToReattribute,
		/// Failed to integrate a migrated proxy set.
		FailedToProcessProxy,
		/// The Relay Chain signalled the end of a migration that has not finished sending.
		NotReconciled,
		/// The queue priority is already what was asked for.
		QueuePriorityAlreadySet,
		/// A priority pattern must give the queue at least one block.
		ZeroPriorityBlocks,
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
		/// A batch of migrated accounts was processed.
		AccountsReceived { count_good: u32, count_bad: u32 },
		/// A batch of migrated registrar records was processed.
		RegistrarReceived { count_good: u32, count_bad: u32 },
		/// A registrar deposit could not be fully re-attributed; the shortfall is parked.
		DepositShortfallParked { para_id: u32, shortfall: BalanceOf<T> },
		/// A batch of migrated HRMP channel records was processed.
		HrmpReceived { count_good: u32, count_bad: u32 },
		/// An HRMP deposit could not be fully re-attributed; the shortfall is parked.
		HrmpShortfallParked { sender: u32, recipient: u32, shortfall: BalanceOf<T> },
		/// A batch of migrated proxy sets was processed.
		ProxiesReceived { count_good: u32, count_bad: u32 },
		/// A batch of pending HRMP open-channel requests was processed.
		HrmpRequestsReceived { count: u32 },
		/// The relay chain signalled that all data has been sent.
		MigrationFinished {
			rc_kept: BalanceOf<T>,
			rc_migrated: BalanceOf<T>,
			ct_minted: BalanceOf<T>,
		},
		/// The relay chain's downward queue was put at the head of the service ring.
		DmpQueuePrioritised { cycle_block: BlockNumberFor<T>, cycle_period: BlockNumberFor<T> },
		/// The queue priority configuration changed.
		DmpQueuePriorityConfigSet {
			old: QueuePriority<BlockNumberFor<T>>,
			new: QueuePriority<BlockNumberFor<T>>,
		},
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(now: BlockNumberFor<T>) {
			if CtMigrationStage::<T>::get().is_ongoing() {
				Self::force_dmp_queue_priority(now);
			}
		}

		fn integrity_test() {
			let (priority_blocks, _) = T::DmpQueuePriorityPattern::get();
			assert!(
				!priority_blocks.is_zero(),
				"the relay chain's queue must get at least one block"
			);
		}
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
			match CtMigrationStage::<T>::get() {
				// try send xcm before updating stage.
				MigrationStage::Pending => {
					Self::send_to_rc(Rc2MigratorCall::CtReady)?;
					Self::transition(MigrationStage::DataMigrationOngoing);
				},
				MigrationStage::DataMigrationOngoing => Self::send_to_rc(Rc2MigratorCall::CtReady)?,
				MigrationStage::CoolOff | MigrationStage::MigrationDone =>
					return Err(Error::<T>::AlreadyFinished.into()),
			}
			Ok(())
		}

		/// Finish the migration.
		///
		/// This is called by the Relay Chain to signal the migration has finished and this chain's
		/// call filters may lift. Separate from `reconcile_balances` because the two happen at
		/// different times: the reconciliation has to be inspectable *during* the window, the
		/// unlock comes after it.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 1))]
		pub fn end_lockdown(origin: OriginFor<T>) -> DispatchResult {
			Self::ensure_root_or_admin_or_manager(origin)?;

			match CtMigrationStage::<T>::get() {
				MigrationStage::CoolOff => Self::transition(MigrationStage::MigrationDone),
				MigrationStage::MigrationDone => (),
				MigrationStage::Pending => return Err(Error::<T>::NotStarted.into()),
				MigrationStage::DataMigrationOngoing =>
					return Err(Error::<T>::NotReconciled.into()),
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

		/// Receive a batch of accounts migrated from the relay chain.
		///
		/// Weight is a placeholder until the migrator pallets get benchmarks; the payload is
		/// bounded by the sender's batch limits.
		#[pallet::call_index(4)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(4, 4).saturating_mul(accounts.len() as u64)
		)]
		pub fn receive_accounts(
			origin: OriginFor<T>,
			accounts: Vec<PortableAccountOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			if CtMigrationStage::<T>::get() == MigrationStage::Pending {
				Self::transition(MigrationStage::DataMigrationOngoing);
			}
			AccountsReceiver::<T>::receive(accounts);
			Ok(())
		}

		/// All data has been sent: reconcile it, and hold the lockdown until the relay chain's
		/// verification window closes.
		///
		/// Carries the relay-side balance bookkeeping so this chain can reconcile what it minted
		/// against what the relay chain burned. A mismatch is loudly reported, never hidden: the
		/// stage still advances so the cool-off verification can inspect the discrepancy.
		#[pallet::call_index(5)]
		#[pallet::weight(T::DbWeight::get().reads_writes(3, 2))]
		pub fn reconcile_balances(
			origin: OriginFor<T>,
			rc_kept: BalanceOf<T>,
			rc_migrated: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;

			let ct_minted = CtMintedTotal::<T>::get();
			if ct_minted != rc_migrated {
				log::error!(
					target: LOG_TARGET,
					"Minted/burned mismatch: RC burned {rc_migrated:?}, CT minted {ct_minted:?}"
				);
			}
			Self::deposit_event(Event::MigrationFinished { rc_kept, rc_migrated, ct_minted });
			Self::transition(MigrationStage::CoolOff);
			Ok(())
		}

		/// Receive a batch of portable proxy sets migrated from the relay chain.
		///
		/// Delegations are written into the real proxy pallet, merged with any existing local
		/// ones, and backed by a deposit at THIS chain's rates reserved from the delegator's
		/// local balance (the accounts stage provides the working buffer). If the reserve cannot
		/// be taken the entry is still written — access for keyless delegators outranks the
		/// deposit — and the shortfall is logged.
		#[pallet::call_index(6)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(3, 3).saturating_mul((proxies.len() as u64).max(1))
		)]
		pub fn receive_proxies(
			origin: OriginFor<T>,
			proxies: Vec<PortableProxyOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			proxy::ProxyReceiver::<T>::receive(proxies);
			Ok(())
		}

		/// Receive a batch of registrar records migrated from the relay chain.
		///
		/// Releases each manager's migrated reserve so the registrar pallet can take its own
		/// deposit, then hands the record over. `next_free_para_id` is carried by the stage-init
		/// message only.
		#[pallet::call_index(7)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(6, 6).saturating_mul((paras.len() as u64).max(1))
		)]
		pub fn receive_registrar(
			origin: OriginFor<T>,
			paras: Vec<PortableParaInfoOf<T>>,
			next_free_para_id: Option<u32>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_registrar(paras, next_free_para_id);
			Ok(())
		}

		/// Receive a batch of HRMP channel records migrated from the relay chain.
		#[pallet::call_index(8)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(2, 2).saturating_mul((channels.len() as u64).max(1))
		)]
		pub fn receive_hrmp(
			origin: OriginFor<T>,
			channels: Vec<PortableHrmpChannelOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_hrmp(channels);
			Ok(())
		}

		/// Receive a batch of pending HRMP open-channel requests migrated from the relay chain.
		///
		/// Each record is handed over and the sender's deposit — which arrived as an
		/// `RcMigratedReserve` hold on the sibling sovereign during the accounts stage — is
		/// released for the HRMP pallet to take its own, same rule as channel deposits.
		#[pallet::call_index(9)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(4, 4).saturating_mul((requests.len() as u64).max(1))
		)]
		pub fn receive_hrmp_requests(
			origin: OriginFor<T>,
			requests: Vec<PortableHrmpRequestOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_hrmp_requests(requests);
			Ok(())
		}

		/// Change how the relay chain's downward queue is prioritised while the migration runs.
		#[pallet::call_index(10)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn set_dmp_queue_priority(
			origin: OriginFor<T>,
			new: QueuePriority<BlockNumberFor<T>>,
		) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;
			let old = DmpQueuePriorityConfig::<T>::get();
			ensure!(old != new, Error::<T>::QueuePriorityAlreadySet);
			if let QueuePriority::OverrideConfig(priority_blocks, _) = new {
				ensure!(!priority_blocks.is_zero(), Error::<T>::ZeroPriorityBlocks);
			}
			DmpQueuePriorityConfig::<T>::put(new);
			Self::deposit_event(Event::DmpQueuePriorityConfigSet { old, new });
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

		/// Put the relay chain's downward queue at the head of the service ring for the next block
		/// if the duty cycle says so. Every sibling's queue is still served, just behind it.
		fn force_dmp_queue_priority(now: BlockNumberFor<T>) {
			let (priority_blocks, round_robin_blocks) = match DmpQueuePriorityConfig::<T>::get() {
				QueuePriority::Config => T::DmpQueuePriorityPattern::get(),
				QueuePriority::OverrideConfig(priority, round_robin) => (priority, round_robin),
				QueuePriority::Disabled => return,
			};
			let period = priority_blocks.saturating_add(round_robin_blocks);
			if period.is_zero() {
				return;
			}
			let cycle_block = now % period;
			if cycle_block >= priority_blocks {
				return;
			}
			// `Ok(false)` is an empty queue, `Err` one this chain has never seen. Neither is a
			// fault.
			let queue = AggregateMessageOrigin::Parent;
			if T::MessageQueue::force_set_head(&mut WeightMeter::new(), &queue).unwrap_or(false) {
				Self::deposit_event(Event::DmpQueuePrioritised {
					cycle_block: cycle_block.saturating_add(One::one()),
					cycle_period: period,
				});
			}
		}

		/// Send a `pallet-rc2-migrator` call to the Relay Chain as a single XCM `Transact`.
		fn send_to_rc(call: Rc2MigratorCall) -> Result<(), Error<T>> {
			let call = Rc2RuntimeCall::Rc2Migrator(call);
			// This is dispatched as `Origin::Xcm(<this chain>)`; the relay chain's `CtOrigin`
			// checks the message came from here.
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

		fn do_receive_registrar(paras: Vec<PortableParaInfoOf<T>>, next_free: Option<u32>) {
			if let Some(id) = next_free {
				T::RegistrarReceiver::receive_next_free_para_id(id);
			}

			let (count_good, count_bad) = Self::receive_batch(
				paras,
				Self::do_receive_para,
				|()| (),
				|para, e| {
					log::error!(
						target: LOG_TARGET,
						"Failed to integrate para {}: {e:?}; parking it",
						para.para_id,
					);
					FailedParas::<T>::insert(para.para_id, para);
				},
			);
			Self::deposit_event(Event::RegistrarReceived { count_good, count_bad });
		}

		/// Hand one migrated registration to the registrar pallet.
		///
		/// The relay chain's recorded deposit is *released*, not re-attributed: the receiving
		/// pallet holds its deposits as `Consideration` tickets, which can only be minted by
		/// taking funds, and it prices them at this chain's own rates. So the migrated hold
		/// becomes free balance and the pallet takes what it needs back out of it. The rates here
		/// are far lower than the relay chain's, so the remainder stays with the manager.
		///
		/// Anything the migrated hold does not cover is a shortfall: parked and reported, since
		/// the record itself is still correct with a deposit this chain prices lower. A manager
		/// who cannot pay at all is a failure, and fails the whole batch.
		fn do_receive_para(para: &PortableParaInfoOf<T>) -> Result<(), DispatchError> {
			let (release, shortfall) =
				AccountsReceiver::<T>::release_rc_reserve(&para.manager, para.deposit)
					.map_err(|_| Error::<T>::FailedToReattribute)?;
			ReattributedDeposits::<T>::mutate(|t| *t = t.saturating_add(release));
			if !shortfall.is_zero() {
				ParkedDepositShortfalls::<T>::insert(para.para_id, shortfall);
				Self::deposit_event(Event::DepositShortfallParked {
					para_id: para.para_id,
					shortfall,
				});
			}

			T::RegistrarReceiver::receive_para(MigratedPara {
				para_id: para.para_id,
				manager: para.manager.clone(),
				state: if para.registered {
					MigratedParaState::Registered { head_len: para.head_len }
				} else {
					MigratedParaState::Reserved
				},
				// Passed through unchanged, including the unset case: a para the relay chain
				// never locked stays eligible for this chain's own automatic lock.
				locked: para.locked,
			})
			.map_err(|e| {
				log::error!(
					target: LOG_TARGET,
					"para {}: registrar refused it: {e:?} (rc deposit {:?}, released {release:?}, \
					 shortfall {shortfall:?}, manager free {:?})",
					para.para_id,
					para.deposit,
					<T as Config>::Currency::balance(&para.manager),
				);
				Error::<T>::FailedToReattribute
			})?;

			Ok(())
		}

		fn do_receive_hrmp(channels: Vec<PortableHrmpChannelOf<T>>) {
			let (count_good, count_bad) = Self::receive_batch(
				channels,
				Self::do_receive_channel,
				|()| (),
				|channel, e| {
					log::error!(
						target: LOG_TARGET,
						"Failed to integrate channel {}->{}: {e:?}; parking it",
						channel.sender, channel.recipient,
					);
					FailedHrmpChannels::<T>::insert((channel.sender, channel.recipient), channel);
				},
			);
			Self::deposit_event(Event::HrmpReceived { count_good, count_bad });
		}

		fn do_receive_hrmp_requests(requests: Vec<PortableHrmpRequestOf<T>>) {
			let count = requests.len() as u32;
			for request in requests {
				// A failed release parks nothing: the deposit simply stays under
				// `RcMigratedReserve` and surfaces in the parked-shortfall checks.
				if let Err(e) = Self::release_hrmp_deposit(
					request.sender,
					(request.sender, request.recipient, true),
					request.sender_deposit,
				) {
					log::error!(
						target: LOG_TARGET,
						"Failed to release request deposit {}->{}: {e:?}",
						request.sender, request.recipient,
					);
				}
				// `confirmed` decides how many deposits are owed: an unconfirmed request is the
				// sender's alone, which is exactly the distinction the receiving pallet draws
				// between its `Pending` and `Open` states.
				if let Err(e) = T::HrmpReceiver::receive_channel(MigratedChannel {
					channel: hrmp_primitives::ChannelId {
						sender: request.sender,
						recipient: request.recipient,
					},
					confirmed: request.confirmed,
				}) {
					log::error!(
						target: LOG_TARGET,
						"Failed to hand over request {}->{}: {e:?}",
						request.sender, request.recipient,
					);
				}
			}
			Self::deposit_event(Event::HrmpRequestsReceived { count });
		}

		/// Release the HRMP deposit that arrived held on `para`'s sibling sovereign, so the HRMP
		/// pallet can take its own at this chain's rates.
		///
		/// Same reasoning as the registrar's: a `Consideration` ticket can only be minted by
		/// taking funds, so the migrated hold has to become free balance first. A shortfall is
		/// parked and reported exactly as it was when the hold was merely re-labelled.
		fn release_hrmp_deposit(
			para: u32,
			key: (u32, u32, bool),
			wanted: BalanceOf<T>,
		) -> Result<(), Error<T>> {
			let sovereign: T::AccountId = sibling_account(para);
			let (release, shortfall) =
				AccountsReceiver::<T>::release_rc_reserve(&sovereign, wanted)
					.map_err(|_| Error::<T>::FailedToReattribute)?;
			ReattributedHrmpDeposits::<T>::mutate(|t| *t = t.saturating_add(release));
			if !shortfall.is_zero() {
				ParkedHrmpShortfalls::<T>::insert(key, shortfall);
				Self::deposit_event(Event::HrmpShortfallParked {
					sender: key.0,
					recipient: key.1,
					shortfall,
				});
			}
			Ok(())
		}

		fn do_receive_channel(channel: &PortableHrmpChannelOf<T>) -> Result<(), DispatchError> {
			for (para, wanted, side) in [
				(channel.sender, channel.sender_deposit, true),
				(channel.recipient, channel.recipient_deposit, false),
			] {
				Self::release_hrmp_deposit(
					para,
					(channel.sender, channel.recipient, side),
					wanted,
				)?;
			}

			// A channel that exists on the relay chain arrives fully open, so the receiving
			// pallet takes both ends' deposits.
			T::HrmpReceiver::receive_channel(MigratedChannel {
				channel: hrmp_primitives::ChannelId {
					sender: channel.sender,
					recipient: channel.recipient,
				},
				confirmed: true,
			})
			.map_err(|_| Error::<T>::FailedToReattribute)?;
			Ok(())
		}
	}
}

/// What each hold migrated from the Relay Chain becomes on this chain.
impl From<PortableHoldReason> for HoldReason {
	fn from(reason: PortableHoldReason) -> Self {
		match reason {
			PortableHoldReason::UnnamedReserve => HoldReason::RcMigratedReserve,
			PortableHoldReason::ProxyDeposit => HoldReason::ProxyDeposit,
			PortableHoldReason::UnattributedReserve => HoldReason::UnattributedReserve,
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Run `integrate` over every item in its own storage transaction. A failing item is rolled
	/// back and handed to `park`; the other items are unaffected. Returns `(count_good,
	/// count_bad)`.
	pub fn receive_batch<I, R>(
		items: Vec<I>,
		integrate: impl Fn(&I) -> Result<R, DispatchError>,
		mut on_good: impl FnMut(R),
		park: impl Fn(I, DispatchError),
	) -> (u32, u32) {
		let (mut count_good, mut count_bad) = (0, 0);
		for item in items {
			match with_storage_layer(|| integrate(&item)) {
				Ok(r) => {
					count_good += 1;
					on_good(r);
				},
				Err(e) => {
					count_bad += 1;
					park(item, e);
				},
			}
		}
		(count_good, count_bad)
	}
}
