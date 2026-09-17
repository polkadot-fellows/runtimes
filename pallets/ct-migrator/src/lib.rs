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

//! Coretime-chain side of the registrar + HRMP migration.
//!
//! Ingests state sent by `pallet-rc2-migrator`, writing through the same code path as fresh
//! registrations so that migrated and newly created state are identical. Temporary pallet;
//! removed once the migration is complete.
//!
//! The portable payload types exchanged between the migrators live in the shared `migrator-types`
//! crate (re-exported here for convenience), so no runtime depends on another chain's pallets
//! just to speak the wire format.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use migrator_types::*;
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use alloc::{vec, vec::Vec};
use cumulus_primitives_core::AggregateMessageOrigin;
use frame_support::{
	defensive_assert,
	pallet_prelude::*,
	traits::{
		fungible::{Inspect, InspectHold, Mutate, MutateHold, Unbalanced, UnbalancedHold},
		tokens::{Fortitude, Precision, Preservation},
		EnsureOrigin,
	},
	weights::WeightMeter,
};
use frame_system::pallet_prelude::*;
use hrmp_primitives::{MigratedChannel, ReceiveMigratedChannels};
use pallet_message_queue::ForceSetHead;
use registrar_primitives::{MigratedPara, MigratedParaState, ReceiveMigratedParas};
use sp_runtime::{
	traits::{One, Saturating, Zero},
	SaturatedConversion,
};
use xcm::prelude::*;

const LOG_TARGET: &str = "runtime::ct-migrator";

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
pub type PortableAccountOf<T> =
	PortableAccount<<T as frame_system::Config>::AccountId, BalanceOf<T>>;
pub type PortableParaInfoOf<T> =
	PortableParaInfo<<T as frame_system::Config>::AccountId, BalanceOf<T>>;
pub type PortableHrmpChannelOf<T> = PortableHrmpChannel<BalanceOf<T>>;
pub type PortableHrmpRequestOf<T> = PortableHrmpRequest<BalanceOf<T>>;
pub type PortableProxyOf<T> = PortableProxy<<T as frame_system::Config>::AccountId>;

/// Progress of the migration. Advanced by messages from `pallet-rc2-migrator`.
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
	#[default]
	Pending,
	DataMigrationOngoing,
	/// All data received and reconciled; this chain stays locked down until the relay chain's
	/// verification window closes.
	CoolOff,
	MigrationDone,
}

impl MigrationStage {
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	pub fn is_ongoing(&self) -> bool {
		matches!(self, Self::DataMigrationOngoing | Self::CoolOff)
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
		///
		/// The `From<PortableHoldReason>` bound is where the runtime declares what each migrated
		/// relay-chain hold becomes locally.
		type RuntimeHoldReason: From<HoldReason> + From<PortableHoldReason>;

		/// How many of this chain's blocks fit in one relay-chain block's time. Used to convert
		/// migrated proxy delays (relay: 6s blocks; this chain: 12s → ratio 2).
		#[pallet::constant]
		type RcBlockTimeRatio: Get<u32>;

		/// Where migrated registrations are handed over. Normally `pallet-registrar-para`.
		///
		/// A seam rather than direct storage writes: which deposit a registration holds in which
		/// state is the receiving pallet's invariant, and rebuilding it out here is how it drifts.
		type RegistrarReceiver: ReceiveMigratedParas<AccountId = Self::AccountId>;

		/// Where migrated HRMP channels are handed over. Normally `pallet-hrmp-para`.
		type HrmpReceiver: ReceiveMigratedChannels;

		/// Router for XCM messages to the relay chain.
		type SendXcm: SendXcm;

		/// The origin that may force the migration stage on this chain.
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// The message queue, so the relay chain's downward queue can be put first.
		type MessageQueue: ForceSetHead<AggregateMessageOrigin>;

		/// `(priority_blocks, round_robin_blocks)` for the relay chain's downward queue while the
		/// migration runs; see [`QueuePriority`]. Overridable through [`DmpQueuePriorityConfig`].
		type DmpQueuePriorityPattern: Get<(BlockNumberFor<Self>, BlockNumberFor<Self>)>;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Balance that was reserved on the relay chain.
		///
		/// Held under this generic reason until the pallet owning the deposit migrates its state
		/// and re-attributes the hold to its own reason.
		#[codec(index = 0)]
		RcMigratedReserve,
		/// A parachain registration deposit migrated from the relay chain.
		///
		/// Placeholder reason: moves to the future registrar pallet's own `HoldReason` when that
		/// pallet lands on this chain.
		#[codec(index = 1)]
		RegistrarDeposit,
		/// An HRMP channel deposit migrated from the relay chain, held on the sibling sovereign
		/// account of the depositing para.
		///
		/// Placeholder reason like [`Self::RegistrarDeposit`], for the future HRMP pallet.
		#[codec(index = 2)]
		HrmpDeposit,
		/// A relay-chain proxy deposit whose definitions travel here. Released when they arrive:
		/// the recreated entry is re-reserved at this chain's rates and the rest becomes free.
		#[codec(index = 3)]
		ProxyDeposit,
		/// Relay-chain reserve that no pallet's deposit records accounted for. Parked here for
		/// investigation — nothing was allowed to stay behind on the relay chain — and never
		/// re-attributed by any stage.
		#[codec(index = 4)]
		UnattributedReserve,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type CtMigrationStage<T: Config> = StorageValue<_, MigrationStage, ValueQuery>;

	/// How the relay chain's downward queue is prioritised while the migration runs.
	#[pallet::storage]
	pub type DmpQueuePriorityConfig<T: Config> =
		StorageValue<_, QueuePriority<BlockNumberFor<T>>, ValueQuery>;

	/// Total balance minted on this chain by the accounts stage.
	///
	/// Reconciled against the relay chain's burned total in `reconcile_balances`.
	#[pallet::storage]
	pub type CtMintedTotal<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	// The migrated registrar and HRMP records are handed straight to the pallets that own them
	// (`Config::RegistrarReceiver` / `Config::HrmpReceiver`) rather than being parked in stand-in
	// storage here. A record that cannot be integrated fails its whole batch instead of being
	// parked, so there is no partial state to reconcile afterwards.

	/// Per-para shortfall between the registrar-recorded deposit and what actually arrived held.
	///
	/// Reconciliation rule: re-attribute `min(recorded, held)`, park the difference here — the
	/// migration never invents balance for deposit records that were not backed by a reserve on
	/// the relay chain (a known on-chain anomaly).
	#[pallet::storage]
	pub type ParkedDepositShortfalls<T: Config> =
		StorageMap<_, Twox64Concat, u32, BalanceOf<T>, OptionQuery>;

	/// Total re-attributed from `RcMigratedReserve` to `RegistrarDeposit` holds.
	#[pallet::storage]
	pub type ReattributedDeposits<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// Total re-attributed from `RcMigratedReserve` to `HrmpDeposit` holds.
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
		/// Failed to integrate a migrated account.
		FailedToProcessAccount,
		/// Failed to flip a migrated reserve to its attributed hold reason.
		FailedToReattribute,
		/// Failed to integrate a migrated proxy set.
		FailedToProcessProxy,
		/// Sending an XCM message to the relay chain failed.
		XcmSendFailed,
		/// The migration has already received everything it is going to.
		AlreadyFinished,
		/// The relay chain signalled the end of a migration that has not finished sending.
		NotReconciled,
		/// The queue priority is already what was asked for.
		QueuePriorityAlreadySet,
		/// A priority pattern must give the queue at least one block.
		ZeroPriorityBlocks,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		StageTransition {
			old: MigrationStage,
			new: MigrationStage,
		},
		/// A batch of migrated accounts was integrated.
		AccountsReceived {
			count: u32,
		},
		/// A batch of migrated registrar records was integrated.
		RegistrarReceived {
			count: u32,
		},
		/// A registrar deposit could not be fully re-attributed; the shortfall is parked.
		DepositShortfallParked {
			para_id: u32,
			shortfall: BalanceOf<T>,
		},
		/// A batch of migrated HRMP channel records was integrated.
		HrmpReceived {
			count: u32,
		},
		/// An HRMP deposit could not be fully re-attributed; the shortfall is parked.
		HrmpShortfallParked {
			sender: u32,
			recipient: u32,
			shortfall: BalanceOf<T>,
		},
		/// A batch of migrated proxy sets was integrated.
		ProxiesReceived {
			count: u32,
		},
		/// A batch of pending HRMP open-channel requests was integrated.
		HrmpRequestsReceived {
			count: u32,
		},
		/// The relay chain signalled that all data has been sent.
		MigrationFinished {
			rc_kept: BalanceOf<T>,
			rc_migrated: BalanceOf<T>,
			ct_minted: BalanceOf<T>,
		},
		/// The relay chain's downward queue was put at the head of the service ring.
		DmpQueuePrioritised {
			cycle_block: BlockNumberFor<T>,
			cycle_period: BlockNumberFor<T>,
		},
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
		/// Receive a batch of accounts migrated from the relay chain.
		///
		/// Dispatched by `pallet-rc2-migrator` via XCM `Transact` with `OriginKind::Superuser`;
		/// the relay chain location converts to Root here.
		///
		/// Weight is a placeholder until the migrator pallets get benchmarks; the payload is
		/// bounded by the sender's batch limits.
		#[pallet::call_index(0)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(4, 4).saturating_mul(accounts.len() as u64)
		)]
		pub fn receive_accounts(
			origin: OriginFor<T>,
			accounts: Vec<PortableAccountOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_accounts(accounts)?;
			Ok(())
		}

		/// Receive a batch of registrar records migrated from the relay chain.
		///
		/// Stores each record and re-attributes the manager's migrated reserve to a
		/// `RegistrarDeposit` hold. `next_free_para_id` is carried by the stage-init message only.
		#[pallet::call_index(1)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(6, 6).saturating_mul((paras.len() as u64).max(1))
		)]
		pub fn receive_registrar(
			origin: OriginFor<T>,
			paras: Vec<PortableParaInfoOf<T>>,
			next_free_para_id: Option<u32>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_registrar(paras, next_free_para_id)?;
			Ok(())
		}

		/// Receive a batch of HRMP channel records migrated from the relay chain.
		#[pallet::call_index(2)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(2, 2).saturating_mul((channels.len() as u64).max(1))
		)]
		pub fn receive_hrmp(
			origin: OriginFor<T>,
			channels: Vec<PortableHrmpChannelOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_hrmp(channels)?;
			Ok(())
		}

		/// Receive a batch of portable proxy sets migrated from the relay chain.
		///
		/// Delegations are written into the real proxy pallet, merged with any existing local
		/// ones, and backed by a deposit at THIS chain's rates reserved from the delegator's
		/// local balance (the accounts stage provides the working buffer). If the reserve cannot
		/// be taken the entry is still written — access for keyless delegators outranks the
		/// deposit — and the shortfall is logged.
		#[pallet::call_index(4)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(3, 3).saturating_mul((proxies.len() as u64).max(1))
		)]
		pub fn receive_proxies(
			origin: OriginFor<T>,
			proxies: Vec<PortableProxyOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_proxies(proxies)?;
			Ok(())
		}

		/// Receive a batch of pending HRMP open-channel requests migrated from the relay chain.
		///
		/// Each record is stored verbatim and the sender's deposit — which arrived as an
		/// `RcMigratedReserve` hold on the sibling sovereign during the accounts stage — is
		/// re-labelled `HrmpDeposit`, same rule as channel deposits.
		#[pallet::call_index(5)]
		#[pallet::weight(
			T::DbWeight::get().reads_writes(4, 4).saturating_mul((requests.len() as u64).max(1))
		)]
		pub fn receive_hrmp_requests(
			origin: OriginFor<T>,
			requests: Vec<PortableHrmpRequestOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_hrmp_requests(requests)?;
			Ok(())
		}

		/// The relay chain asks whether this chain can receive migrated state.
		///
		/// Answering opens the migration here and unblocks the relay chain's warm-up. A repeat is
		/// answered again without reopening, so a resent signal is harmless.
		#[pallet::call_index(6)]
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
				MigrationStage::CoolOff | MigrationStage::MigrationDone =>
					return Err(Error::<T>::AlreadyFinished.into()),
			}
			Ok(())
		}

		/// The relay chain's verification window has closed: lift this chain's call filters.
		///
		/// Separate from `reconcile_balances` because the two happen at different times. The
		/// reconciliation has to be inspectable *during* the window; the unlock comes after it.
		#[pallet::call_index(7)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn end_lockdown(origin: OriginFor<T>) -> DispatchResult {
			// relay chain origin converts to root.
			ensure_root(origin)?;

			match CtMigrationStage::<T>::get() {
				MigrationStage::CoolOff => Self::transition(MigrationStage::MigrationDone),
				MigrationStage::MigrationDone => (),
				MigrationStage::Pending | MigrationStage::DataMigrationOngoing =>
					return Err(Error::<T>::NotReconciled.into()),
			}
			Ok(())
		}

		/// Set the migration stage directly. See `pallet-rc2-migrator`'s equivalent.
		#[pallet::call_index(8)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStage) -> DispatchResult {
			T::AdminOrigin::ensure_origin(origin)?;

			Self::transition(stage);
			Ok(())
		}

		/// All data has been sent: reconcile it, and hold the lockdown until the relay chain's
		/// verification window closes.
		///
		/// Carries the relay-side balance bookkeeping so this chain can reconcile what it minted
		/// against what the relay chain burned. A mismatch is loudly reported, never hidden: the
		/// stage still advances so the cool-off verification can inspect the discrepancy.
		#[pallet::call_index(3)]
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

		/// Change how the relay chain's downward queue is prioritised while the migration runs.
		#[pallet::call_index(9)]
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

		/// Send a `pallet-rc2-migrator` call to the relay chain.
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

		/// Integrate a batch item-by-item, stopping at the first failure.
		///
		/// An item that cannot be integrated aborts the whole batch: the error propagates out of
		/// the `receive_*` extrinsic, which fails the dispatch, which fails the `Transact` the
		/// relay chain sent. The relay chain learns of it through the `ReportTransactStatus`
		/// appendix on that message and halts rather than sending the next batch.
		///
		/// This is what keeps the two chains' ledgers in step. The relay chain burns a balance
		/// before it sends it; an item silently dropped here would be burned there and never
		/// minted here, and nothing before the end-of-migration reconciliation would notice.
		/// `describe` names the offending item for the log, since the dispatch error alone
		/// carries no payload back to the relay chain.
		fn receive_batch<I, R>(
			items: Vec<I>,
			integrate: impl Fn(&I) -> Result<R, Error<T>>,
			mut on_good: impl FnMut(R),
			describe: impl Fn(&I) -> alloc::string::String,
		) -> Result<u32, Error<T>> {
			let mut count = 0;
			for item in items {
				match integrate(&item) {
					Ok(r) => {
						count += 1;
						on_good(r);
					},
					Err(e) => {
						log::error!(
							target: LOG_TARGET,
							"Failed to integrate {}: {e:?}; failing the batch",
							describe(&item),
						);
						return Err(e);
					},
				}
			}
			Ok(count)
		}

		fn do_receive_accounts(accounts: Vec<PortableAccountOf<T>>) -> Result<(), Error<T>> {
			let stage = CtMigrationStage::<T>::get();
			if stage == MigrationStage::Pending {
				Self::transition(MigrationStage::DataMigrationOngoing);
			}

			let mut minted: BalanceOf<T> = Zero::zero();
			let count = Self::receive_batch(
				accounts,
				Self::do_receive_account,
				|amount| minted = minted.saturating_add(amount),
				|account| alloc::format!("account {:?}", account.who),
			)?;
			if !minted.is_zero() {
				CtMintedTotal::<T>::mutate(|t| *t = t.saturating_add(minted));
			}
			Self::deposit_event(Event::AccountsReceived { count });
			Ok(())
		}

		/// Returns the amount minted for this account; the caller tracks the batch total.
		fn do_receive_account(account: &PortableAccountOf<T>) -> Result<BalanceOf<T>, Error<T>> {
			let who = &account.who;
			let held: BalanceOf<T> = account
				.holds
				.iter()
				.fold(Zero::zero(), |acc: BalanceOf<T>, hold| acc.saturating_add(hold.amount));
			let total = account.free.saturating_add(held);

			// Accounts whose incoming free balance cannot provide the existential deposit get a
			// provider reference so the mint and hold below cannot fail or dust the account.
			if frame_system::Pallet::<T>::providers(who).is_zero() &&
				<T as Config>::Currency::balance(who).saturating_add(account.free) <
					<T as Config>::Currency::minimum_balance()
			{
				frame_system::Pallet::<T>::inc_providers(who);
			}

			let minted = <T as Config>::Currency::mint_into(who, total)
				.map_err(|_| Error::<T>::FailedToProcessAccount)?;
			defensive_assert!(minted == total, "minted what the relay chain burned");

			for hold in &account.holds {
				Self::place_hold(&hold.reason.into(), who, hold.amount)
					.map_err(|_| Error::<T>::FailedToProcessAccount)?;
			}

			Ok(minted)
		}

		/// The inverse of [`Self::place_hold`]: move `amount` from a hold back to free balance.
		///
		/// Same hazard as there, same fix in reverse. `release` decreases the hold first, and if
		/// that takes it to zero while the free part is still sub-ED, pallet-balances dusts the
		/// remainder — which is exactly the shape a deposit holder whose liquid dust travelled
		/// here alongside the deposit is in. Crediting the free part *first* means the hold never
		/// passes through zero while free is below ED. The two primitives are mint-and-burn of
		/// the same amount, so total issuance is untouched, just as `release` would be.
		fn release_hold(
			reason: &T::RuntimeHoldReason,
			who: &T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(), DispatchError> {
			<T as Config>::Currency::increase_balance(who, amount, Precision::Exact)?;
			<T as Config>::Currency::decrease_balance_on_hold(
				reason,
				who,
				amount,
				Precision::Exact,
			)?;
			Ok(())
		}

		/// Release `min(wanted, actually-held)` of `who`'s migrated `RcMigratedReserve` hold to
		/// free balance, returning `(released, shortfall)`.
		///
		/// The reconciliation rule of the whole receive side: recorded deposits are honoured up
		/// to what actually arrived held, and the difference is the caller's to park under its
		/// own key. One implementation so every deposit kind reconciles identically.
		fn release_rc_reserve(
			who: &T::AccountId,
			wanted: BalanceOf<T>,
		) -> Result<(BalanceOf<T>, BalanceOf<T>), Error<T>> {
			let rc_reason: T::RuntimeHoldReason = HoldReason::RcMigratedReserve.into();
			let held = <T as Config>::Currency::balance_on_hold(&rc_reason, who);
			let release = wanted.min(held);
			if !release.is_zero() {
				Self::release_hold(&rc_reason, who, release).map_err(|e| {
					log::error!(
						target: LOG_TARGET,
						"releasing {release:?} of {held:?} held on {who:?} failed: {e:?}",
					);
					Error::<T>::FailedToReattribute
				})?;
			}
			Ok((release, wanted.saturating_sub(held)))
		}

		/// Place `amount` of `who`'s free balance under `reason` without the account ever sitting
		/// at zero reserve mid-operation.
		///
		/// `MutateHold::hold` decreases the free balance before it books the hold; if that leaves
		/// a sub-ED free remainder while nothing is reserved yet, pallet-balances dusts the
		/// remainder. Deposit holders whose liquid dust deliberately travelled here alongside the
		/// deposit are in exactly that shape, so the two steps run in the reverse (safe) order.
		/// The low-level primitives keep total issuance untouched, like `hold` itself.
		fn place_hold(
			reason: &T::RuntimeHoldReason,
			who: &T::AccountId,
			amount: BalanceOf<T>,
		) -> Result<(), DispatchError> {
			<T as Config>::Currency::increase_balance_on_hold(
				reason,
				who,
				amount,
				Precision::Exact,
			)?;
			<T as Config>::Currency::decrease_balance(
				who,
				amount,
				Precision::Exact,
				Preservation::Expendable,
				Fortitude::Force,
			)?;
			Ok(())
		}

		fn do_receive_registrar(
			paras: Vec<PortableParaInfoOf<T>>,
			next_free: Option<u32>,
		) -> Result<(), Error<T>> {
			if let Some(id) = next_free {
				T::RegistrarReceiver::receive_next_free_para_id(id);
			}

			let count = Self::receive_batch(
				paras,
				Self::do_receive_para,
				|()| (),
				|para| alloc::format!("para {}", para.para_id),
			)?;
			Self::deposit_event(Event::RegistrarReceived { count });
			Ok(())
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
		fn do_receive_para(para: &PortableParaInfoOf<T>) -> Result<(), Error<T>> {
			let (release, shortfall) = Self::release_rc_reserve(&para.manager, para.deposit)?;
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

		fn do_receive_proxies(proxies: Vec<PortableProxyOf<T>>) -> Result<(), Error<T>> {
			let count = Self::receive_batch(
				proxies,
				Self::do_receive_proxy,
				|()| (),
				|proxy| alloc::format!("proxies of {:?}", proxy.delegator),
			)?;
			Self::deposit_event(Event::ProxiesReceived { count });
			Ok(())
		}

		fn do_receive_proxy(proxy: &PortableProxyOf<T>) -> Result<(), Error<T>> {
			use frame_support::traits::ReservableCurrency;

			// Resize the migrated relay-chain deposit to this chain's rates: release it whole —
			// making it free balance — and re-reserve below only what the recreated entry needs.
			// The difference stays free on this chain, in the delegator's hands.
			let proxy_reason: T::RuntimeHoldReason = HoldReason::ProxyDeposit.into();
			let migrated =
				<T as Config>::Currency::balance_on_hold(&proxy_reason, &proxy.delegator);
			if !migrated.is_zero() {
				Self::release_hold(&proxy_reason, &proxy.delegator, migrated)
					.map_err(|_| Error::<T>::FailedToProcessProxy)?;
			}
			let delay_ratio = T::RcBlockTimeRatio::get().max(1);

			pallet_proxy::Proxies::<T>::try_mutate(&proxy.delegator, |(defs, deposit)| {
				for delegate in proxy.delegates.iter() {
					let def = pallet_proxy::ProxyDefinition {
						delegate: delegate.delegate.clone(),
						proxy_type: delegate.proxy_type.into(),
						delay: (delegate.delay / delay_ratio).saturated_into(),
					};
					if !defs.contains(&def) {
						defs.try_push(def).map_err(|_| Error::<T>::FailedToProcessProxy)?;
					}
				}

				// Back the entry at this chain's rates (normally from the released deposit
				// above), topping up whatever is already reserved for pre-existing local
				// proxies. Priced by the proxy pallet itself, so a migrated entry can never
				// diverge from what the pallet would charge.
				let required = pallet_proxy::Pallet::<T>::deposit(defs.len() as u32);
				let top_up = required.saturating_sub(*deposit);
				if !top_up.is_zero() {
					match <T as pallet_proxy::Config>::Currency::reserve(&proxy.delegator, top_up) {
						Ok(()) => *deposit = required,
						// Access outranks the deposit; the entry stays under-backed until the
						// owner tops it up.
						Err(_) => log::warn!(
							target: LOG_TARGET,
							"Proxies of {:?} under-backed: could not reserve {top_up:?}",
							proxy.delegator,
						),
					}
				}
				Ok(())
			})
		}

		fn do_receive_hrmp(channels: Vec<PortableHrmpChannelOf<T>>) -> Result<(), Error<T>> {
			let count = Self::receive_batch(
				channels,
				Self::do_receive_channel,
				|()| (),
				|channel| {
					alloc::format!("channel {}->{}", channel.sender, channel.recipient)
				},
			)?;
			Self::deposit_event(Event::HrmpReceived { count });
			Ok(())
		}

		fn do_receive_hrmp_requests(
			requests: Vec<PortableHrmpRequestOf<T>>,
		) -> Result<(), Error<T>> {
			let count = Self::receive_batch(
				requests,
				|request| {
					Self::release_hrmp_deposit(
						request.sender,
						(request.sender, request.recipient, true),
						request.sender_deposit,
					)?;
					// `confirmed` decides how many deposits are owed: an unconfirmed request is
					// the sender's alone, which is exactly the distinction the receiving pallet
					// draws between its `Pending` and `Open` states.
					T::HrmpReceiver::receive_channel(MigratedChannel {
						channel: hrmp_primitives::ChannelId {
							sender: request.sender,
							recipient: request.recipient,
						},
						confirmed: request.confirmed,
					})
					.map_err(|_| Error::<T>::FailedToReattribute)
				},
				|()| (),
				|request| {
					alloc::format!("request {}->{}", request.sender, request.recipient)
				},
			)?;
			Self::deposit_event(Event::HrmpRequestsReceived { count });
			Ok(())
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
			let (release, shortfall) = Self::release_rc_reserve(&sovereign, wanted)?;
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

		fn do_receive_channel(channel: &PortableHrmpChannelOf<T>) -> Result<(), Error<T>> {
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

		fn transition(new: MigrationStage) {
			let old = CtMigrationStage::<T>::get();
			CtMigrationStage::<T>::put(new.clone());
			log::info!(target: LOG_TARGET, "Stage transition: {old:?} -> {new:?}");
			Self::deposit_event(Event::StageTransition { old, new });
		}
	}
}
