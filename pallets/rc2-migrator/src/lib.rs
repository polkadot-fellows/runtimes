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

pub mod accounts;
pub mod hrmp;
pub mod multisig;
pub mod proxy;
pub mod registrar;
pub mod sweep;
pub mod ti_correction;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub use multisig::{ManagerMultisig, ManagerMultisigVote};
pub use pallet::*;

use accounts::{ExpectedReserve, MAX_ACCOUNTS_PER_BLOCK};
use alloc::{boxed::Box, vec, vec::Vec};
use frame_support::{
	defensive,
	dispatch::GetDispatchInfo,
	pallet_prelude::*,
	storage::with_storage_layer,
	traits::{
		fungible::{Inspect, Mutate},
		tokens::{Fortitude, Precision, Preservation},
		EnsureOrigin, ReservableCurrency, Time,
	},
	weights::WeightMeter,
};
use frame_system::pallet_prelude::*;
use migrator_types::{
	PortableAccount, PortableHrmpChannel, PortableHrmpRequest, PortableParaInfo, PortableProxy,
	PortableProxyType, QueuePriority,
};
use pallet_message_queue::ForceSetHead;
use polkadot_parachain_primitives::primitives::{HrmpChannelId, Id as ParaId};
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::inclusion::{AggregateMessageOrigin, UmpQueueId};
use sp_runtime::{
	traits::{Dispatchable, One, Saturating, Zero},
	AccountId32, MultiSignature,
};
use xcm::prelude::*;

/// Weight allowed to `receive_query_response` when a batch's report lands back here.
///
/// The call reads one map and writes one stage; the margin is deliberate, since a report that
/// cannot pay its own execution is a batch whose outcome is lost.
const REPORT_RESPONSE_WEIGHT: Weight = Weight::from_parts(1_000_000_000, 100_000);

const LOG_TARGET: &str = "runtime::rc2-migrator";

pub type MigrationStageOf<T> =
	MigrationStage<<T as frame_system::Config>::AccountId, BlockNumberFor<T>, MomentOf<T>>;

/// Total balance kept on the Relay Chain and total migrated, by destination.
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
pub struct MigratedBalances {
	/// Issuance still on the Relay Chain. Seeded with the total issuance when the accounts stage
	/// starts, and falls as the stages burn balance here. Zero once the migration ends.
	pub kept: u128,
	/// Deposits burned here and re-established as holds on the Coretime chain.
	pub ct_reserved: u128,
	/// Free working buffer burned here and minted liquid on the Coretime chain.
	pub ct_free: u128,
	/// Free balance burned here and teleported to Asset Hub.
	pub ah_free: u128,
	/// Phantom issuance burned by the `TiCorrection` stage (issuance no account held).
	pub ti_corrected: u128,
}

impl MigratedBalances {
	/// Everything that went to the Coretime chain; what `reconcile_balances` reconciles against.
	pub fn migrated_ct(&self) -> u128 {
		self.ct_reserved.saturating_add(self.ct_free)
	}
}

/// Wall-clock type the schedule is expressed in.
pub type MomentOf<T> = <<T as Config>::TimeProvider as Time>::Moment;

/// Maximum number of accounts packed into one XCM message.
///
/// An encoded [`PortableAccount`] is ~65 bytes, keeping the message far below the DMP size limit.
pub const MAX_ACCOUNTS_PER_XCM: u32 = 100;

/// Batch and per-block limits for the registrar and HRMP stages. Their record counts are small
/// (dozens to hundreds on Polkadot), so one limit serves both.
pub const MAX_RECORDS_PER_XCM: u32 = 50;
pub const MAX_RECORDS_PER_BLOCK: u32 = 100;

/// Maximum beneficiaries in one teleport message to Asset Hub: one `DepositAsset` instruction
/// each, and an XCM message decodes at most 100 instructions.
pub const MAX_TELEPORTS_PER_XCM: u32 = 40;

/// The migration stage of the Relay Chain. Advanced by `on_initialize`, except where noted, and
/// only while [`Paused`] is clear.
///
/// Variants are in the order the migration progresses through them.
///
/// Nothing halts the machine on its own: a batch the Coretime chain refuses is recorded and the
/// migration continues, because stopping leaves both chains locked down with no way forward.
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
		/// The block number at which the warm-up period will end. It is absolute and a pause
		/// does not move it.
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
		/// The block number at which the post migration cool-off period will end. It is absolute
		/// and a pause does not move it.
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

	/// Whether this stage must wait for the Coretime chain to confirm the batches it has sent.
	///
	/// Every data stage destroys what it sends, so a second batch built on an unconfirmed first
	/// widens a gap neither chain can see. Listed the other way round — what does *not* wait —
	/// so that a data stage added later inherits the wait instead of silently skipping it:
	/// the stages below either send nothing or are the single signals whose own stage already
	/// gates what follows them.
	pub fn waits_for_confirmation(&self) -> bool {
		!matches!(
			self,
			Self::Pending |
				Self::Scheduled { .. } |
				Self::WaitingForCt |
				Self::WarmUp { .. } |
				Self::CoolOff { .. } |
				Self::MigrationDone
		)
	}
}

/// `CtMigrator`'s pallet index in the Coretime (receiver) chain.
pub const CT_MIGRATOR_PALLET_INDEX: u8 = 255;

/// Call encoding for the Coretime chain runtime, reduced to the pallet this chain dispatches into.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum CtRuntimeCall {
	CtMigrator(CtMigratorCall) = CT_MIGRATOR_PALLET_INDEX,
}

/// Call encoding for the calls needed from the ct-migrator pallet.
///
/// Indices are the `#[pallet::call_index]`es in `pallet-ct-migrator`; 2 (`force_set_stage`) and
/// 3 (`set_manager`) are never sent from here.
// Held in `UnconfirmedBatches` until the Coretime chain confirms it, hence the storage derives.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum CtMigratorCall {
	#[codec(index = 0)]
	StartMigration,
	#[codec(index = 1)]
	EndLockdown,
	#[codec(index = 4)]
	ReceiveAccounts { accounts: Vec<PortableAccount<AccountId32, u128>> },
	#[codec(index = 5)]
	ReconcileBalances { rc_kept: u128, rc_migrated: u128 },
	#[codec(index = 6)]
	ReceiveProxies { proxies: Vec<PortableProxy<AccountId32>> },
	#[codec(index = 7)]
	ReceiveRegistrar {
		paras: Vec<PortableParaInfo<AccountId32, u128>>,
		next_free_para_id: Option<u32>,
	},
	#[codec(index = 8)]
	ReceiveHrmp { channels: Vec<PortableHrmpChannel<u128>> },
	#[codec(index = 9)]
	ReceiveHrmpRequests { requests: Vec<PortableHrmpRequest<u128>> },
}

/// A batch sent to the Coretime chain that has not been confirmed.
///
/// Carries the payload so it can be re-sent: the relay chain burns what a batch contains before
/// sending it, so once a batch is in flight this is the only copy of that state anywhere.
#[derive(
	Encode, Decode, DecodeWithMemTracking, CloneNoBound, PartialEq, Eq, DebugNoBound, TypeInfo,
)]
#[scale_info(skip_type_params(T))]
pub struct UnconfirmedBatch<T: pallet::Config> {
	/// The call as it was sent, ready to re-send verbatim.
	pub call: CtMigratorCall,
	/// The stage that sent it. Names the batch for an operator and scopes what a retry affects.
	pub stage: MigrationStageOf<T>,
	/// Block it was last sent at; the timeout is measured from here, so a retry resets it.
	pub sent_at: BlockNumberFor<T>,
}

/// Registers a query whose response dispatches a call back into this pallet.
///
/// A seam over `pallet_xcm::new_notify_query` rather than a `pallet_xcm::Config` bound: the bound
/// would drag an XCM executor into every mock that only ever needs a query id.
pub trait NotifyQueryHandler<T: pallet::Config> {
	/// Register a query answered by `responder`, to be reported to
	/// [`pallet::Call::receive_query_response`], and return its id.
	fn new_notify_query(responder: Location, timeout: BlockNumberFor<T>) -> u64;
}

impl<T, R> NotifyQueryHandler<T> for R
where
	T: pallet::Config,
	R: pallet_xcm::Config<RuntimeCall: From<pallet::Call<T>>>,
	BlockNumberFor<T>: Into<BlockNumberFor<R>>,
{
	fn new_notify_query(responder: Location, timeout: BlockNumberFor<T>) -> u64 {
		pallet_xcm::Pallet::<R>::new_notify_query(
			responder,
			<R as pallet_xcm::Config>::RuntimeCall::from(
				pallet::Call::<T>::receive_query_response { query_id: 0, response: Response::Null },
			),
			timeout.into(),
			Location::here(),
		)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Bound to `pallet_balances` rather than the fungible traits because the accounts stage
	/// edits `frame_system::Account` and `pallet_balances::TotalIssuance` directly: a migrating
	/// account is burned whole, including one that other pallets still reference, and no
	/// fungible API allows that.
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

		/// Working buffer of free balance that follows a migrated deposit to the Coretime chain,
		/// so deposit owners can pay fees and future deposits there without a teleport first.
		#[pallet::constant]
		type CtFreeBuffer: Get<u128>;

		/// Asset Hub's existential deposit. Free balance below this cannot be teleported into a
		/// fresh account; such dust follows the deposit to the Coretime chain instead.
		#[pallet::constant]
		type AhExistentialDeposit: Get<u128>;

		/// Native currency.
		type Currency: Mutate<Self::AccountId, Balance = u128>
			+ ReservableCurrency<Self::AccountId, Balance = u128>;

		/// Para id of Asset Hub, the destination of teleported free balances.
		type AhParaId: Get<u32>;

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

		/// Calls the manager multisig may dispatch once it reaches its threshold.
		type RuntimeCall: Parameter
			+ Dispatchable<RuntimeOrigin = <Self as frame_system::Config>::RuntimeOrigin>
			+ GetDispatchInfo;

		/// Members of a multisig that can submit unsigned txs and act as the manager.
		type MultisigMembers: Get<Vec<AccountId32>>;

		/// Threshold of `MultisigMembers`.
		type MultisigThreshold: Get<u32>;

		/// Limit the number of votes of each participant per round.
		type MultisigMaxVotesPerRound: Get<u32>;

		/// Round the vote counter starts at. Must differ per network.
		type MultisigStartRound: Get<u32>;

		/// The message queue, so the Coretime chain's upward queue can be put first.
		type MessageQueue: ForceSetHead<AggregateMessageOrigin>;

		/// `(priority_blocks, round_robin_blocks)` for the Coretime chain's upward queue while the
		/// migration runs; see [`QueuePriority`]. Overridable per migration through
		/// [`CtUmpQueuePriorityConfig`].
		type CtUmpQueuePriorityPattern: Get<(BlockNumberFor<Self>, BlockNumberFor<Self>)>;

		/// How long a batch sent to the Coretime chain may go unanswered before it is reported.
		/// Counted from the block the batch was sent; see [`UnconfirmedBatches`].
		#[pallet::constant]
		type XcmResponseTimeout: Get<BlockNumberFor<Self>>;

		/// How many batches may be outstanding before the machine stops extracting more data.
		///
		/// The relay chain destroys what it sends, so it must not run arbitrarily far ahead of
		/// what the Coretime chain has acknowledged. Overridable per migration through
		/// [`UnprocessedMsgBuffer`].
		#[pallet::constant]
		type UnprocessedMsgBuffer: Get<u32>;

		/// Registers the notify query that carries a batch's dispatch result back from the
		/// Coretime chain. Normally `pallet-xcm`; see [`NotifyQueryHandler`].
		type NotifyQueryHandler: NotifyQueryHandler<Self>;

		/// The origin a query response from the Coretime chain dispatches with. Normally
		/// `pallet_xcm::EnsureResponse`, which is a different origin from [`Self::CtOrigin`]: a
		/// report answers a query this chain registered, rather than being a call the Coretime
		/// chain chose to make.
		type ResponseOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
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
	/// can not set the manager account id via `set_manager` call. Kept funded through the
	/// migration and reaped when it ends.
	#[pallet::storage]
	pub type Manager<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// Whether the migration is paused.
	///
	/// The stage is untouched, so the migration still counts as ongoing. While paused the machine
	/// may be repositioned with `force_set_stage`, and `resume_migration` continues from whatever
	/// stage it then holds. Inbound signals such as `ct_ready` and batch reports are still
	/// recorded; only `on_initialize` stands still.
	///
	/// Only set while the stage is ongoing. Forcing the stage out of the run clears it.
	///
	/// Different from v1's `MigrationStage::MigrationPaused` variant: an independent flag, so the
	/// stage paused at is kept.
	#[pallet::storage]
	pub type Paused<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Balance kept on the Relay Chain versus migrated away. Set up by the accounts stage.
	#[pallet::storage]
	pub type RcMigratedBalance<T: Config> = StorageValue<_, MigratedBalances, ValueQuery>;

	/// What each account's reserved balance is expected to be made of, built from the owning
	/// pallets' recorded deposit fields before any account is withdrawn. The recorded fields are
	/// the routing source of truth; the anonymous reserves are only trusted up to these amounts,
	/// and anything beyond them travels as an unattributed hold, parked at the destination.
	///
	/// One record per account rather than one map per kind: the three amounts are always built
	/// together and always read together in the withdrawal split.
	#[pallet::storage]
	pub type ExpectedReserves<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, ExpectedReserve, ValueQuery>;

	/// Batches sent to the Coretime chain whose dispatch result has not come back yet, by the
	/// notify-query id `send_to_ct` registered for each.
	///
	/// The machine holds while more than [`UnprocessedMsgBuffer`] entries are outstanding: the
	/// relay chain burns state before it sends it, so it must not run far ahead of what the
	/// Coretime chain has acknowledged. An entry is removed when
	/// [`Pallet::receive_query_response`] reports success; a failure or a timeout is reported and
	/// the entry stays, so [`Pallet::retry_batch`] can re-send it.
	///
	/// The payload is kept, not just its id: the relay chain has already destroyed what the batch
	/// carries, so this map is the only remaining copy. [`Pallet::retry_batch`] re-sends it.
	#[pallet::storage]
	#[pallet::unbounded]
	pub type UnconfirmedBatches<T: Config> =
		StorageMap<_, Twox64Concat, u64, UnconfirmedBatch<T>, OptionQuery>;

	/// How many batches are outstanding, so the per-block gate need not walk the map.
	#[pallet::storage]
	pub type UnconfirmedBatchCount<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Overrides [`Config::UnprocessedMsgBuffer`] for this migration; `None` uses the constant.
	#[pallet::storage]
	pub type UnprocessedMsgBuffer<T: Config> = StorageValue<_, u32, OptionQuery>;

	/// The multisig members that voted to execute a specific call.
	#[pallet::storage]
	#[pallet::unbounded]
	pub type ManagerMultisigs<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::RuntimeCall, Vec<AccountId32>, ValueQuery>;

	/// The current round of the multisig voting. Votes are only valid for the current round.
	#[pallet::storage]
	pub type ManagerMultisigRound<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// How often each member voted in the current round. Cleared at the end of each round.
	#[pallet::storage]
	pub type ManagerVotesInCurrentRound<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId32, u32, ValueQuery>;

	/// How the Coretime chain's upward queue is prioritised while the migration runs.
	#[pallet::storage]
	pub type CtUmpQueuePriorityConfig<T: Config> =
		StorageValue<_, QueuePriority<BlockNumberFor<T>>, ValueQuery>;

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
		/// The account balance could not be fully withdrawn.
		FailedToWithdrawAccount,
		/// The migrated/kept balance bookkeeping would overflow.
		BalanceAccounting,
		/// The unsigned multisig vote did not validate.
		UnsignedValidationFailed,
		/// The vote carries a round that is no longer open.
		RoundStale,
		/// The member has used up its votes for this round.
		MaxVotesPerRound,
		/// The member has already voted for this call in this round.
		DuplicateVote,
		/// The queue priority is already what was asked for.
		QueuePriorityAlreadySet,
		/// A priority pattern must give the queue at least one block.
		ZeroPriorityBlocks,
		/// The response names a query this pallet is not waiting on.
		UnknownQuery,
		/// The response to a batch was not a dispatch result.
		UnexpectedResponse,
	}

	impl<T> From<accounts::Error> for Error<T> {
		fn from(e: accounts::Error) -> Self {
			match e {
				accounts::Error::FailedToWithdrawAccount => Error::FailedToWithdrawAccount,
				accounts::Error::BalanceAccounting => Error::BalanceAccounting,
			}
		}
	}

	impl<T> From<sweep::Error> for Error<T> {
		fn from(e: sweep::Error) -> Self {
			match e {
				sweep::Error::FailedToWithdrawAccount => Error::FailedToWithdrawAccount,
				sweep::Error::BalanceAccounting => Error::BalanceAccounting,
			}
		}
	}

	impl<T> From<ti_correction::Error> for Error<T> {
		fn from(e: ti_correction::Error) -> Self {
			match e {
				ti_correction::Error::BalanceAccounting => Error::BalanceAccounting,
			}
		}
	}

	impl<T> From<multisig::Error> for Error<T> {
		fn from(e: multisig::Error) -> Self {
			match e {
				multisig::Error::UnsignedValidationFailed => Error::UnsignedValidationFailed,
				multisig::Error::RoundStale => Error::RoundStale,
				multisig::Error::MaxVotesPerRound => Error::MaxVotesPerRound,
				multisig::Error::DuplicateVote => Error::DuplicateVote,
			}
		}
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
		/// An account carried reserve that no pallet's deposit records account for. It travels to
		/// the Coretime chain under its own hold reason and stays parked there for investigation.
		UnattributedReserve { who: AccountId32, amount: u128 },
		/// A deposit whose purpose ends with this chain was released; it travels to Asset Hub as
		/// free balance.
		DepositRefunded { who: AccountId32, amount: u128 },
		/// An account that a consumer reference forbids reaping (session keys being the known
		/// case) was drained to a zero-balance shell; the balance travels like any other
		/// account's.
		AccountShellDrained { who: AccountId32, amount: u128 },
		/// A batch of withdrawn accounts was sent to the Coretime chain.
		AccountsBatchSent { count: u32 },
		/// A batch of free balances was teleported to Asset Hub.
		AccountsTeleported { count: u32, amount: u128 },
		/// A batch of portable proxy sets was sent to the Coretime chain.
		ProxyBatchSent { count: u32 },
		/// Pending HRMP open-channel requests were sent to the Coretime chain.
		HrmpRequestsSent { count: u32 },
		/// A leftover pot was emptied; its balance teleports to the sweep beneficiary on AH.
		AccountSwept { who: AccountId32, amount: u128 },
		/// Below-ED dust accounts were reaped; the sum teleports to the sweep beneficiary.
		DustSwept { count: u32, amount: u128 },
		/// Phantom issuance burned: `burned = min(expected, unaccounted)`. Any
		/// `unaccounted - burned` remainder is left on the books for investigation.
		TiCorrected { expected: u128, unaccounted: u128, burned: u128 },
		/// The measured unaccounted issuance was BELOW the audited expectation — the phantom
		/// shrank since it was measured, which no known mechanism explains. Observability only;
		/// the correction still burned the measured amount.
		TiCorrectionAnomaly { expected: u128, unaccounted: u128 },
		/// A batch of drained registrar records was sent to the Coretime chain.
		RegistrarBatchSent { count: u32 },
		/// A batch of drained HRMP channel records was sent to the Coretime chain.
		HrmpBatchSent { count: u32 },
		/// Zero-balance records held alive only by stale provider references were reaped.
		HusksReaped { count: u32 },
		/// The manager multisig dispatched a call.
		ManagerMultisigDispatched { res: DispatchResult },
		/// The manager multisig received a vote.
		ManagerMultisigVoted { votes: u32 },
		/// The manager's remaining balance left for Asset Hub and the appointment ended.
		ManagerReaped { who: T::AccountId, amount: u128 },
		/// The Coretime chain's upward queue was put at the head of the service ring.
		CtUmpQueuePrioritised { cycle_block: BlockNumberFor<T>, cycle_period: BlockNumberFor<T> },
		/// The queue priority configuration changed.
		CtUmpQueuePriorityConfigSet {
			old: QueuePriority<BlockNumberFor<T>>,
			new: QueuePriority<BlockNumberFor<T>>,
		},
		/// The outstanding-batch buffer was changed.
		UnprocessedMsgBufferSet { old: Option<u32>, new: Option<u32> },
		/// A batch was re-sent under a new query id; the old one is forgotten.
		BatchRetried { old_query_id: u64, new_query_id: u64, stage: MigrationStageOf<T> },
		/// A batch was abandoned by an operator without being delivered. Its contents are lost:
		/// the relay chain burned them before sending. Recorded so the loss is on the chain.
		BatchAbandoned { query_id: u64, stage: MigrationStageOf<T> },
		/// The Coretime chain integrated a batch; the stage machine may continue.
		BatchConfirmed { query_id: u64, stage: MigrationStageOf<T> },
		/// The Coretime chain rejected a batch. Reported only; the entry stays outstanding for
		/// `retry_batch`.
		BatchFailed { query_id: u64, stage: MigrationStageOf<T>, error: MaybeErrorCode },
		/// A batch went unanswered for [`Config::XcmResponseTimeout`]. Reported once; the entry
		/// stays outstanding for `retry_batch`.
		BatchTimedOut { query_id: u64, stage: MigrationStageOf<T> },
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			Self::progress_migration(now)
		}

		fn on_runtime_upgrade() -> Weight {
			ManagerMultisig::<T>::init_round();
			T::DbWeight::get().reads_writes(1, 1)
		}

		fn on_finalize(now: BlockNumberFor<T>) {
			if RcMigrationStage::<T>::get().is_ongoing() {
				Self::force_ct_ump_queue_priority(now);
			}
		}

		fn integrity_test() {
			let (priority_blocks, _) = T::CtUmpQueuePriorityPattern::get();
			assert!(!priority_blocks.is_zero(), "the Coretime queue must get at least one block");
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
		///
		/// A target outside the run (`Pending`, `Scheduled`, `MigrationDone`) ends the pause with
		/// it, so no `resume_migration` follows. `Scheduled` with a `start` already in the past
		/// starts on the next block.
		#[pallet::call_index(1)]
		#[pallet::weight(T::DbWeight::get().reads_writes(2, 2))]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStageOf<T>) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			ensure!(Paused::<T>::get(), Error::<T>::NotPaused);

			if !stage.is_ongoing() {
				Paused::<T>::kill();
			}
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
			Self::ensure_root_or_admin(origin)?;
			if let Some(ref who) = new {
				ensure!(
					frame_system::Pallet::<T>::consumers(who) == 0,
					Error::<T>::AccountReferenced
				);
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
		/// paused; a scheduled one is cancelled instead. A paused machine never reaches
		/// `MigrationDone`, so the manager is not reaped out from under an active pause.
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

		/// Vote on behalf of any of the members in [`Config::MultisigMembers`].
		///
		/// Unsigned extrinsic, requiring the `payload` to be signed. Members therefore need no
		/// funded account on this chain, which is the point: this chain is being drained.
		///
		/// Each call adds the member to `ManagerMultisigs` under `payload.call`. Once
		/// [`Config::MultisigThreshold`] members have voted for the same call it is dispatched as
		/// the multisig's account, the map is cleared and the round advances, which is what stops
		/// an old round's signatures from being replayed.
		#[pallet::call_index(7)]
		#[pallet::weight(Weight::from_parts(10_000_000, 1000))]
		pub fn vote_manager_multisig(
			origin: OriginFor<T>,
			payload: Box<ManagerMultisigVote<T>>,
			sig: MultiSignature,
		) -> DispatchResult {
			ensure_none(origin)?;

			ManagerMultisig::<T>::vote(&payload, &sig).map_err(Error::<T>::from)?;
			Ok(())
		}

		/// Change how the Coretime chain's upward queue is prioritised while the migration runs.
		#[pallet::call_index(8)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn set_ct_ump_queue_priority(
			origin: OriginFor<T>,
			new: QueuePriority<BlockNumberFor<T>>,
		) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			let old = CtUmpQueuePriorityConfig::<T>::get();
			ensure!(old != new, Error::<T>::QueuePriorityAlreadySet);
			if let QueuePriority::OverrideConfig(priority_blocks, _) = new {
				ensure!(!priority_blocks.is_zero(), Error::<T>::ZeroPriorityBlocks);
			}
			CtUmpQueuePriorityConfig::<T>::put(new);
			Self::deposit_event(Event::CtUmpQueuePriorityConfigSet { old, new });
			Ok(())
		}

		/// The Coretime chain reports what it did with a batch.
		///
		/// Dispatched by `pallet-xcm` when the `ReportTransactStatus` appendix that [`send_to_ct`]
		/// attached answers its notify query, so the origin is the response origin rather than the
		/// Coretime chain's sovereign one.
		///
		/// A success clears the batch. A failure is reported and leaves the entry in
		/// [`UnconfirmedBatches`] for [`Pallet::retry_batch`]; the machine does not stop, because
		/// both chains are locked down until the migration finishes and a stuck machine is worse
		/// than a recorded gap.
		#[pallet::call_index(9)]
		#[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
		pub fn receive_query_response(
			origin: OriginFor<T>,
			query_id: u64,
			response: Response,
		) -> DispatchResult {
			T::ResponseOrigin::ensure_origin(origin)?;

			let batch = UnconfirmedBatches::<T>::get(query_id).ok_or(Error::<T>::UnknownQuery)?;
			let stage = batch.stage;
			let Response::DispatchResult(result) = response else {
				return Err(Error::<T>::UnexpectedResponse.into());
			};

			if result == MaybeErrorCode::Success {
				Self::clear_batch(query_id);
				Self::deposit_event(Event::BatchConfirmed { query_id, stage });
			} else {
				// Reported, not acted on. Halting mid-run is not an available action — both
				// chains are locked down until the migration finishes — so a stuck machine is
				// worse than a recorded gap. The entry stays, so `retry_batch` can re-send it and
				// the outstanding count keeps the machine from racing further ahead.
				log::error!(
					target: LOG_TARGET,
					"Coretime chain rejected the batch sent in stage {stage:?}: {result:?}",
				);
				Self::deposit_event(Event::BatchFailed { query_id, stage, error: result });
			}
			Ok(())
		}

		/// Re-send an outstanding batch to the Coretime chain.
		///
		/// The recovery path for a batch the Coretime chain rejected or never answered. The
		/// payload is re-sent verbatim under a fresh query; the old query id is forgotten, so a
		/// late answer to it is ignored.
		///
		/// Usable while the migration runs — it does not stop for a failed batch — and afterwards,
		/// which is when most of these will be worked through.
		///
		/// Idempotent in the way that matters: a batch the Coretime chain did integrate before the
		/// report was lost is re-applied, and a double-mint would surface in `reconcile_balances`.
		///
		/// Free: cleaning up after the migration's own failure is not the operator's cost.
		#[pallet::call_index(10)]
		#[pallet::weight(T::DbWeight::get().reads_writes(4, 4))]
		pub fn retry_batch(origin: OriginFor<T>, query_id: u64) -> DispatchResultWithPostInfo {
			Self::ensure_admin_or_manager(origin)?;

			let batch = UnconfirmedBatches::<T>::get(query_id).ok_or(Error::<T>::UnknownQuery)?;
			Self::clear_batch(query_id);
			let new_query_id = Self::dispatch_batch(batch.call, batch.stage.clone())?;

			Self::deposit_event(Event::BatchRetried {
				old_query_id: query_id,
				new_query_id,
				stage: batch.stage,
			});
			Ok(Pays::No.into())
		}

		/// Give up on an outstanding batch.
		///
		/// Clears the slot so the outstanding count stops holding the machine back. **The batch's
		/// contents stay lost**: the relay chain burned them before sending, and nothing
		/// re-derives them. The event is the record, and `reconcile_balances` reports the gap.
		///
		/// Root only, and a last resort — [`Pallet::retry_batch`] first. Free, like the retry it
		/// follows.
		#[pallet::call_index(11)]
		#[pallet::weight(T::DbWeight::get().reads_writes(3, 3))]
		pub fn abandon_batch(origin: OriginFor<T>, query_id: u64) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;

			let batch = UnconfirmedBatches::<T>::get(query_id).ok_or(Error::<T>::UnknownQuery)?;
			Self::clear_batch(query_id);
			log::error!(
				target: LOG_TARGET,
				"Batch {query_id} from stage {:?} abandoned; its contents are lost",
				batch.stage,
			);
			Self::deposit_event(Event::BatchAbandoned { query_id, stage: batch.stage });
			Ok(Pays::No.into())
		}

		/// Change how many batches may be outstanding before data extraction pauses for a block.
		#[pallet::call_index(12)]
		#[pallet::weight(T::DbWeight::get().reads_writes(1, 1))]
		pub fn set_unprocessed_msg_buffer(
			origin: OriginFor<T>,
			new: Option<u32>,
		) -> DispatchResult {
			Self::ensure_admin_or_manager(origin)?;
			let old = UnprocessedMsgBuffer::<T>::get();
			UnprocessedMsgBuffer::<T>::set(new);
			Self::deposit_event(Event::UnprocessedMsgBufferSet { old, new });
			Ok(())
		}
	}

	// `ValidateUnsigned` is deprecated in favour of `#[pallet::authorize]`
	// (paritytech/polkadot-sdk#2415). Kept as v1 wrote it; this pallet is deleted after the
	// migration.
	#[allow(deprecated)]
	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::vote_manager_multisig { payload, sig } = call {
				ManagerMultisig::<T>::validate_unsigned(payload, sig)
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}

	impl<T: Config> Pallet<T> {
		/// Ensure that the origin is root or [`Config::AdminOrigin`].
		fn ensure_root_or_admin(origin: OriginFor<T>) -> DispatchResult {
			if ensure_root(origin.clone()).is_err() {
				T::AdminOrigin::ensure_origin(origin)?;
			}
			Ok(())
		}

		/// Ensure that the origin is one accepted by [`Self::ensure_root_or_admin`] or signed by
		/// the [`Manager`] account id or by the manager multisig.
		fn ensure_admin_or_manager(origin: OriginFor<T>) -> DispatchResult {
			if let Ok(who) = ensure_signed(origin.clone()) {
				if Manager::<T>::get().is_some_and(|manager| manager == who) {
					return Ok(());
				}
				if who == ManagerMultisig::<T>::manager_multisig_id() {
					return Ok(());
				}
			}
			Self::ensure_root_or_admin(origin)
		}

		/// Put the Coretime chain's upward queue at the head of the service ring for the next
		/// block if the duty cycle says so. Every other queue is still served, just behind it.
		fn force_ct_ump_queue_priority(now: BlockNumberFor<T>) {
			let (priority_blocks, round_robin_blocks) = match CtUmpQueuePriorityConfig::<T>::get() {
				QueuePriority::Config => T::CtUmpQueuePriorityPattern::get(),
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
			let queue = AggregateMessageOrigin::Ump(UmpQueueId::Para(T::CtParaId::get().into()));
			// `Ok(false)` is an empty queue, `Err` one this chain has never seen -- which a fresh
			// network has until the Coretime chain first speaks. Neither is a fault.
			if T::MessageQueue::force_set_head(&mut WeightMeter::new(), &queue).unwrap_or(false) {
				Self::deposit_event(Event::CtUmpQueuePrioritised {
					cycle_block: cycle_block.saturating_add(One::one()),
					cycle_period: period,
				});
			}
		}

		/// Report every batch whose deadline passes this block.
		///
		/// Runs every block while anything is outstanding, whatever the stage: a batch sent by
		/// the last data stage is still owed an answer while the machine cools off. Reported
		/// once, on the block the deadline passes -- the entry stays outstanding, so anything
		/// else would fire every block for the rest of the migration -- and never acted on: a
		/// migration that stops mid-run leaves both chains locked down with no way forward, so
		/// the gap is recorded and settled after the run.
		fn report_timed_out_batches(now: BlockNumberFor<T>) {
			let timeout = T::XcmResponseTimeout::get();
			for (query_id, batch) in UnconfirmedBatches::<T>::iter() {
				if now.saturating_sub(batch.sent_at) == timeout {
					log::error!(
						target: LOG_TARGET,
						"Batch {query_id} sent in stage {:?} unanswered after {timeout:?} blocks",
						batch.stage,
					);
					Self::deposit_event(Event::BatchTimedOut { query_id, stage: batch.stage });
				}
			}
		}

		/// Whether this block's data extraction must be skipped because more batches are
		/// outstanding than [`UnprocessedMsgBuffer`] allows: the relay chain would run ahead of
		/// what the Coretime chain has acknowledged, and the queues would grow without bound.
		fn should_hold_for_batches() -> bool {
			UnconfirmedBatchCount::<T>::get() > Self::unprocessed_msg_buffer()
		}

		/// How many batches may be outstanding before the machine stops sending more.
		pub fn unprocessed_msg_buffer() -> u32 {
			UnprocessedMsgBuffer::<T>::get().unwrap_or_else(T::UnprocessedMsgBuffer::get)
		}

		/// Drop a batch from the outstanding set, keeping the count in step.
		fn clear_batch(query_id: u64) {
			if UnconfirmedBatches::<T>::take(query_id).is_some() {
				UnconfirmedBatchCount::<T>::mutate(|n| *n = n.saturating_sub(1));
			}
		}

		/// Record a batch as outstanding, keeping the count in step.
		fn track_batch(query_id: u64, batch: UnconfirmedBatch<T>) {
			if !UnconfirmedBatches::<T>::contains_key(query_id) {
				UnconfirmedBatchCount::<T>::mutate(|n| n.saturating_inc());
			}
			UnconfirmedBatches::<T>::insert(query_id, batch);
		}

		// TODO(ahm-v2): proper benchmark
		fn progress_migration(now: BlockNumberFor<T>) -> Weight {
			if UnconfirmedBatchCount::<T>::get() > 0 {
				Self::report_timed_out_batches(now);
			}
			if Paused::<T>::get() {
				return T::DbWeight::get().reads(2);
			}
			let stage = RcMigrationStage::<T>::get();
			// A stage that sends state must not run arbitrarily far ahead of what the Coretime
			// chain has acknowledged; the handshake and closing signals are exempt, each being a
			// single message whose own stage gates what follows it.
			if stage.waits_for_confirmation() && Self::should_hold_for_batches() {
				return T::DbWeight::get().reads(2);
			}

			match stage {
				// The scheduled start is compared against the clock, which at `on_initialize` still
				// holds the previous block's timestamp -- so the migration begins on the first
				// block after the one whose timestamp passed `start`.
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
							accounts::AccountsMigrator::<T>::init();
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
						|| Self::migrate_accounts_block(last_key),
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
						|| {
							proxy::ProxyMigrator::<T>::drain_announcements();
							Ok(())
						},
						MigrationStage::ProxyOngoing { last_key: None },
					);
					T::DbWeight::get().reads_writes(100, 100)
				},
				MigrationStage::ProxyOngoing { last_key } => {
					Self::migrate_stage_step(
						|| Self::migrate_proxies_block(last_key),
						MigrationStage::ProxyDone,
						|last_key| MigrationStage::ProxyOngoing { last_key: Some(last_key) },
					);
					Self::placeholder_weight(proxy::MAX_PROXIES_PER_BLOCK)
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
						|| {
							let total = sweep::SweepMigrator::<T>::sweep_pots()?;
							Self::teleport_swept(total)
						},
						MigrationStage::SweepDust { last_key: None },
					);
					T::DbWeight::get().reads_writes(10, 10)
				},
				MigrationStage::SweepDust { last_key } => {
					Self::migrate_stage_step(
						|| {
							let sweep::BlockSweep { amount, last_key } =
								sweep::SweepMigrator::<T>::sweep_dust(last_key)?;
							Self::teleport_swept(amount)?;
							Ok(last_key)
						},
						MigrationStage::TiCorrection,
						|last_key| MigrationStage::SweepDust { last_key: Some(last_key) },
					);
					Self::placeholder_weight(sweep::MAX_SWEPT_PER_BLOCK)
				},
				MigrationStage::TiCorrection => {
					Self::migrate_stage_once(
						Self::correct_total_issuance,
						MigrationStage::CoolOff {
							end_at: now.saturating_add(CoolOffPeriod::<T>::get()),
						},
					);
					T::DbWeight::get().reads_writes(10, 10)
				},
				// The Coretime chain holds its lockdown until this signal: the relay chain owns
				// the lifecycle, and the two chains share no clock to hold to on their own.
				MigrationStage::CoolOff { end_at } if now >= end_at => {
					Self::migrate_stage_once(
						|| {
							Self::send_to_ct(CtMigratorCall::EndLockdown)?;
							Self::reap_manager()
						},
						MigrationStage::MigrationDone,
					);
					T::DbWeight::get().reads_writes(6, 6)
				},
				// Waiting on the clock or a block height.
				MigrationStage::Scheduled { .. } |
				MigrationStage::WaitingForCt |
				MigrationStage::WarmUp { .. } |
				MigrationStage::CoolOff { .. } => T::DbWeight::get().reads(1),
				// Nothing runs before the schedule or after the end.
				MigrationStage::Pending | MigrationStage::MigrationDone =>
					T::DbWeight::get().reads(1),
			}
		}

		/// One block of the accounts stage: withdraw up to the per-block limit, then ship the
		/// pieces in XCM-sized chunks. Runs inside the caller's transaction, so a failed send
		/// rolls the block's withdrawals back with it.
		pub(crate) fn migrate_accounts_block(
			last_key: Option<T::AccountId>,
		) -> Result<Option<T::AccountId>, Error<T>> {
			let manager = Manager::<T>::get();
			let accounts::BlockWithdrawals { ct, ah, last_key } =
				accounts::AccountsMigrator::<T>::migrate_many(last_key, manager.as_ref())?;
			for chunk in ct.chunks(MAX_ACCOUNTS_PER_XCM as usize) {
				Self::send_accounts(chunk.to_vec())?;
			}
			for chunk in ah.chunks(MAX_TELEPORTS_PER_XCM as usize) {
				Self::send_teleport(chunk.to_vec())?;
			}
			Ok(last_key)
		}

		/// One block of the proxy stage: migrate up to the per-block limit, then ship the
		/// portable sets in XCM-sized chunks.
		pub(crate) fn migrate_proxies_block(
			last_key: Option<T::AccountId>,
		) -> Result<Option<T::AccountId>, Error<T>> {
			let proxy::BlockProxies { proxies, last_key } =
				proxy::ProxyMigrator::<T>::migrate_many(last_key);
			for chunk in proxies.chunks(MAX_RECORDS_PER_XCM as usize) {
				Self::send_proxies(chunk.to_vec())?;
			}
			Ok(last_key)
		}

		/// Teleport what a sweep block burned to the sweep beneficiary on Asset Hub.
		fn teleport_swept(total: u128) -> Result<(), Error<T>> {
			if total == 0 {
				return Ok(());
			}
			Self::send_teleport(vec![(T::SweepBeneficiary::get(), total)])
		}

		/// Burn the audited phantom issuance and send the finish signal.
		///
		/// The manager is the one account still funded at this point; it stays so through
		/// `CoolOff`.
		fn correct_total_issuance() -> Result<(), Error<T>> {
			let manager_balance = Manager::<T>::get()
				.map(|who| <T as Config>::Currency::total_balance(&who))
				.unwrap_or_default();
			ti_correction::TiCorrector::<T>::correct_total_issuance(manager_balance)?;
			let tracker = RcMigratedBalance::<T>::get();
			Self::send_reconciliation(tracker.kept, tracker.migrated_ct())
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
			match with_storage_layer(|| migrate().map_err(DispatchError::from)) {
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
			match with_storage_layer(|| work().map_err(DispatchError::from)) {
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

		/// Execute a stage transition and log it.
		pub(crate) fn transition(new: MigrationStageOf<T>) {
			let old = RcMigrationStage::<T>::mutate(|stage| core::mem::replace(stage, new.clone()));
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

		/// End the manager's appointment and teleport what it has left to the same account on
		/// Asset Hub. Nothing to do if no manager was appointed, or it has nothing left.
		fn reap_manager() -> Result<(), Error<T>> {
			let Some(who) = Manager::<T>::take() else { return Ok(()) };
			let amount = <T as Config>::Currency::reducible_balance(
				&who,
				Preservation::Expendable,
				Fortitude::Polite,
			);
			if amount == 0 {
				Self::deposit_event(Event::ManagerReaped { who, amount });
				return Ok(());
			}
			let burned = <T as Config>::Currency::burn_from(
				&who,
				amount,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite,
			)
			.map_err(|_| Error::<T>::FailedToWithdrawAccount)?;
			RcMigratedBalance::<T>::try_mutate(|t| {
				t.kept = t.kept.checked_sub(burned).ok_or(Error::<T>::BalanceAccounting)?;
				t.ah_free = t.ah_free.checked_add(burned).ok_or(Error::<T>::BalanceAccounting)?;
				Ok::<(), Error<T>>(())
			})?;
			let dest = migrator_types::translate_destination(&who);
			Self::deposit_event(Event::ManagerReaped { who, amount: burned });
			Self::send_teleport(vec![(dest, burned)])
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

		/// Send a `pallet-ct-migrator` call to the Coretime chain as a single XCM `Transact` and
		/// record it as unconfirmed.
		///
		/// The message carries its dispatch result back: a notify query is registered here, and
		/// the `ReportTransactStatus` appendix answers it with the Transact Status Register once
		/// the Coretime chain has run the call. The appendix is what makes a *failed* call
		/// reportable — an appendix runs whether or not the message body succeeded, so the
		/// preceding `ExpectTransactStatus` can abort the body and the report still goes out.
		///
		/// The query id is held in [`UnconfirmedBatches`] until the answer arrives; the stage
		/// machine will not send the next batch while an entry is outstanding.
		fn send_to_ct(call: CtMigratorCall) -> Result<(), Error<T>> {
			let stage = RcMigrationStage::<T>::get();
			Self::dispatch_batch(call, stage).map(|_| ())
		}

		/// Send one batch under a fresh notify query and record it as outstanding.
		///
		/// Shared by the stage machine and [`Pallet::retry_batch`], so a retry is byte-identical
		/// to the original send; only the query id and the timeout clock are new. `stage` is the
		/// stage that produced the payload, which a retry preserves rather than overwriting with
		/// whatever stage the machine has since reached.
		fn dispatch_batch(
			call: CtMigratorCall,
			stage: MigrationStageOf<T>,
		) -> Result<u64, Error<T>> {
			let now = frame_system::Pallet::<T>::block_number();
			let timeout = now.saturating_add(T::XcmResponseTimeout::get());
			let query_id = T::NotifyQueryHandler::new_notify_query(
				Location::new(0, [Parachain(T::CtParaId::get())]),
				timeout,
			);

			// `Superuser` converts to Root on the Coretime chain, which system chains grant the
			// relay-chain location; the `receive_*` calls check for Root.
			let message = Xcm(vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				SetAppendix(Xcm(vec![ReportTransactStatus(QueryResponseInfo {
					destination: Location::parent(),
					query_id,
					max_weight: REPORT_RESPONSE_WEIGHT,
				})])),
				Transact {
					origin_kind: OriginKind::Superuser,
					fallback_max_weight: None,
					call: CtRuntimeCall::CtMigrator(call.clone()).encode().into(),
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
			Self::track_batch(query_id, UnconfirmedBatch { call, stage, sent_at: now });
			Ok(query_id)
		}
	}
}
