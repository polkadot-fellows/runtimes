// This file is part of Substrate.

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
//! subsystems such as Governance, Staking, Balances from the Relay Chain to the Asset Hub. This
//! pallet works alongside its counterpart, `pallet_ah_migrator`, which handles migration
//! processes on the Asset Hub side.
//!
//! This pallet is responsible for controlling the initiation, progression, and completion of the
//! migration process, including managing its various stages and transferring the necessary data.
//! The pallet directly accesses the storage of other pallets for read/write operations while
//! maintaining compatibility with their existing APIs.
//!
//! To simplify development and avoid the need to edit the original pallets, this pallet may
//! duplicate private items such as storage entries from the original pallets. This ensures that the
//! migration logic can be implemented without altering the original implementations.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod accounts;
pub mod claims;
pub mod crowdloan;
pub mod indices;
pub mod multisig;
pub mod preimage;
pub mod proxy;
pub mod referenda;
pub mod staking;
pub mod types;
pub mod vesting;
pub mod weights;
pub mod weights_ah;
pub use pallet::*;
pub mod asset_rate;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod bounties;
pub mod child_bounties;
pub mod conviction_voting;
#[cfg(feature = "kusama-ahm")]
pub mod recovery;
pub mod scheduler;
#[cfg(feature = "kusama-ahm")]
pub mod society;
pub mod treasury;
pub mod xcm_config;

pub use weights::*;

use crate::{
	accounts::MigratedBalances,
	types::{MigrationFinishedData, XcmBatch, XcmBatchAndMeter},
};
use accounts::AccountsMigrator;
use child_bounties::ChildBountiesMigrator;
use claims::{ClaimsMigrator, ClaimsStage};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::AccountIdConversion,
	storage::transactional::with_transaction_opaque_err,
	traits::{
		fungible::{Inspect, InspectFreeze, Mutate, MutateFreeze, MutateHold},
		schedule::DispatchTime,
		tokens::{Fortitude, Pay, Precision, Preservation},
		Contains, Defensive, DefensiveTruncateFrom, EnqueueMessage, LockableCurrency,
		ReservableCurrency, VariantCount,
	},
	weights::{Weight, WeightMeter},
	PalletId,
};
use frame_system::{pallet_prelude::*, AccountInfo};
use indices::IndicesMigrator;
use multisig::MultisigMigrator;
use pallet_balances::AccountData;
use pallet_message_queue::ForceSetHead;
use polkadot_parachain_primitives::primitives::Id as ParaId;
use polkadot_runtime_common::{
	claims as pallet_claims, crowdloan as pallet_crowdloan, impls::VersionedLocatableAsset,
	paras_registrar, slots as pallet_slots,
};
use preimage::{
	PreimageChunkMigrator, PreimageLegacyRequestStatusMigrator, PreimageRequestStatusMigrator,
};
use proxy::*;
use referenda::ReferendaStage;
use runtime_parachains::{
	hrmp,
	inclusion::{AggregateMessageOrigin, UmpQueueId},
};
use sp_core::{crypto::Ss58Codec, H256};
use sp_runtime::{
	traits::{BadOrigin, BlockNumberProvider, Dispatchable, Hash, IdentifyAccount, One, Zero},
	AccountId32, Saturating,
};
use sp_std::prelude::*;
use staking::{
	bags_list::{BagsListMigrator, BagsListStage},
	delegated_staking::{DelegatedStakingMigrator, DelegatedStakingStage},
	nom_pools::{NomPoolsMigrator, NomPoolsStage},
};
use storage::TransactionOutcome;
use types::IntoPortable;
pub use types::{MigrationStatus, PalletMigration, QueuePriority as AhUmpQueuePriority};
use vesting::VestingMigrator;
use weights_ah::WeightInfo as AhWeightInfo;
use xcm::prelude::*;
use xcm_builder::MintLocation;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::rc-migrator";

/// Soft limit on the DMP message size.
///
/// The hard limit should be about 64KiB which means that we stay well below
/// that to avoid any trouble. We can raise this as final preparation for the migration once
/// everything is confirmed to work.
pub const MAX_XCM_SIZE: u32 = 50_000;

/// The maximum number of items that can be migrated in a single block.
///
/// This serves as an additional safety limit beyond the weight accounting of both the Relay Chain
/// and Asset Hub.
pub const MAX_ITEMS_PER_BLOCK: u32 = 1600;

/// The maximum number of XCM messages that can be sent in a single block.
pub const MAX_XCM_MSG_PER_BLOCK: u32 = 10;

/// Out of weight Error. Can be converted to a pallet error for convenience.
pub struct OutOfWeightError;

impl<T: Config> From<OutOfWeightError> for Error<T> {
	fn from(_: OutOfWeightError) -> Self {
		Self::OutOfWeight
	}
}

pub type MigrationStageOf<T> = MigrationStage<
	<T as frame_system::Config>::AccountId,
	BlockNumberFor<T>,
	<T as pallet_bags_list::Config<pallet_bags_list::Instance1>>::Score,
	conviction_voting::alias::ClassOf<T>,
	<T as pallet_asset_rate::Config>::AssetKind,
	scheduler::SchedulerBlockNumberFor<T>,
>;

type AccountInfoFor<T> =
	AccountInfo<<T as frame_system::Config>::Nonce, <T as frame_system::Config>::AccountData>;

/// Migration settings.
#[derive(
	Encode, DecodeWithMemTracking, Decode, Clone, Debug, TypeInfo, MaxEncodedLen, PartialEq,
)]
pub struct MigrationSettings {
	/// The maximum number of items that can be  extracted and migrated in a single block.
	///
	/// Overrides [MAX_ITEMS_PER_BLOCK] and [Self::max_items_per_block] if set.
	pub max_accounts_per_block: Option<u32>,
	/// The maximum number of items that can be  extracted and migrated in a single block.
	///
	/// Overrides [MAX_ITEMS_PER_BLOCK] if set.
	pub max_items_per_block: Option<u32>,
}

/// The maximum number of items that can be extracted and migrated in a single block.
///
/// Returns constant [MAX_ITEMS_PER_BLOCK] if no settings are set in storage by the manager.
pub fn max_items_per_block<T: Config>() -> u32 {
	Settings::<T>::get()
		.and_then(|settings| settings.max_items_per_block)
		.unwrap_or(MAX_ITEMS_PER_BLOCK)
}

/// The maximum number of accounts that can be extracted and migrated in a single block.
///
/// Returns constant [MAX_ITEMS_PER_BLOCK] if no settings are set in storage by the manager.
pub fn max_accounts_per_block<T: Config>() -> u32 {
	Settings::<T>::get()
		.and_then(|settings| settings.max_accounts_per_block)
		.unwrap_or(max_items_per_block::<T>())
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Paras Registrar Pallet
	type ParasRegistrar<T> = paras_registrar::Pallet<T>;

	/// Super config trait for all pallets that the migration depends on, providing convenient
	/// access to their items.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountData = AccountData<u128>, AccountId = AccountId32, Nonce = u32>
		+ pallet_balances::Config<
			RuntimeHoldReason = <Self as Config>::RuntimeHoldReason,
			FreezeIdentifier = <Self as Config>::RuntimeFreezeReason,
			Balance = u128,
		> + hrmp::Config
		+ paras_registrar::Config
		+ pallet_multisig::Config
		+ pallet_proxy::Config
		+ pallet_preimage::Config<Hash = H256>
		+ pallet_referenda::Config<Votes = u128>
		+ pallet_nomination_pools::Config
		+ pallet_fast_unstake::Config<Currency = pallet_balances::Pallet<Self>>
		+ pallet_bags_list::Config<pallet_bags_list::Instance1, Score = u64>
		+ pallet_scheduler::Config
		+ pallet_vesting::Config
		+ pallet_indices::Config
		+ pallet_conviction_voting::Config
		+ pallet_asset_rate::Config
		+ pallet_slots::Config
		+ pallet_crowdloan::Config
		+ pallet_staking::Config<CurrencyBalance = u128>
		+ pallet_claims::Config
		+ pallet_bounties::Config
		+ pallet_child_bounties::Config
		+ pallet_treasury::Config<
			Currency = pallet_balances::Pallet<Self>,
			BlockNumberProvider = Self::TreasuryBlockNumberProvider,
			Paymaster = Self::TreasuryPaymaster,
			AssetKind = VersionedLocatableAsset,
			Beneficiary = VersionedLocation,
		> + pallet_delegated_staking::Config<Currency = pallet_balances::Pallet<Self>>
		+ pallet_xcm::Config
		+ pallet_staking_async_ah_client::Config
	{
		/// The overall runtime origin type.
		type RuntimeOrigin: Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>
			+ IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ From<frame_system::RawOrigin<Self::AccountId>>;
		/// The overall runtime call type.
		type RuntimeCall: From<Call<Self>>
			+ IsType<<Self as pallet_xcm::Config>::RuntimeCall>
			+ Dispatchable<RuntimeOrigin = <Self as Config>::RuntimeOrigin>
			+ Member
			+ Parameter;
		/// The runtime hold reasons.
		type RuntimeHoldReason: Parameter
			+ VariantCount
			+ IntoPortable<Portable = types::PortableHoldReason>;

		/// Config for pallets that are only on Kusama.
		#[cfg(feature = "kusama-ahm")]
		type KusamaConfig: pallet_recovery::Config<
				Currency = pallet_balances::Pallet<Self>,
				BlockNumberProvider = Self::RecoveryBlockNumberProvider,
				MaxFriends = ConstU32<{ recovery::MAX_FRIENDS }>,
			> + frame_system::Config<
				AccountData = AccountData<u128>,
				AccountId = AccountId32,
				Hash = sp_core::H256,
			> + pallet_society::Config<
				Currency = pallet_balances::Pallet<Self>,
				BlockNumberProvider = Self::RecoveryBlockNumberProvider,
				MaxPayouts = ConstU32<{ society::MAX_PAYOUTS }>,
			>;

		/// Block number provider of the recovery pallet.
		#[cfg(feature = "kusama-ahm")]
		type RecoveryBlockNumberProvider: BlockNumberProvider<BlockNumber = u32>;

		/// The proxy types of pure accounts that are kept for free.
		type PureProxyFreeVariants: Contains<<Self as pallet_proxy::Config>::ProxyType>;

		/// Block number provider of the treasury pallet.
		///
		/// This is here to simplify the code of the treasury, bounties and child-bounties migration
		/// code since they all depend on the treasury provided block number. The compiler checks
		/// that this is configured correctly.
		type TreasuryBlockNumberProvider: BlockNumberProvider<BlockNumber = u32>;
		type TreasuryPaymaster: Pay<
			Id = u64,
			Balance = u128,
			Beneficiary = VersionedLocation,
			AssetKind = VersionedLocatableAsset,
		>;

		type SessionDuration: Get<u64>;

		/// The runtime freeze reasons.
		type RuntimeFreezeReason: Parameter
			+ VariantCount
			+ IntoPortable<Portable = types::PortableFreezeReason>;

		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The origin that can perform permissioned operations like setting the migration stage.
		///
		/// This is generally root, Asset Hub and Fellows origins.
		type AdminOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Native asset registry type.
		type Currency: Mutate<Self::AccountId, Balance = u128>
			+ MutateHold<Self::AccountId, Reason = <Self as Config>::RuntimeHoldReason>
			+ InspectFreeze<Self::AccountId, Id = <Self as Config>::RuntimeFreezeReason>
			+ MutateFreeze<Self::AccountId>
			+ ReservableCurrency<Self::AccountId, Balance = u128>
			+ LockableCurrency<Self::AccountId, Balance = u128>;
		/// XCM checking account.
		type CheckingAccount: Get<Self::AccountId>;
		/// Send DMP message.
		type SendXcm: SendXcm;
		/// The maximum weight that this pallet can consume `on_initialize`.
		type MaxRcWeight: Get<Weight>;
		/// The maximum weight that Asset Hub can consume for processing one migration package.
		///
		/// Every data package that is sent from this pallet should not take more than this.
		type MaxAhWeight: Get<Weight>;
		/// Weight information for the functions of this pallet.
		type RcWeightInfo: WeightInfo;
		/// Weight information for the processing the packages from this pallet on the Asset Hub.
		type AhWeightInfo: AhWeightInfo;
		/// The existential deposit on the Asset Hub.
		type AhExistentialDeposit: Get<<Self as pallet_balances::Config>::Balance>;
		/// Contains calls that are allowed during the migration.
		type RcIntraMigrationCalls: Contains<<Self as frame_system::Config>::RuntimeCall>;
		/// Contains calls that are allowed after the migration finished.
		type RcPostMigrationCalls: Contains<<Self as frame_system::Config>::RuntimeCall>;
		/// The hold reason for staking delegation.
		type StakingDelegationReason: Get<<Self as Config>::RuntimeHoldReason>;
		/// The pallet ID for on-demand pallet.
		type OnDemandPalletId: Get<PalletId>;
		/// Maximum number of unprocessed DMP messages allowed before the RC migrator temporarily
		/// pauses sending data messages to the Asset Hub.
		///
		/// The Asset Hub confirms processed message counts back to this pallet. Due to async
		/// backing, there is typically a delay of 1-2 blocks before these confirmations are
		/// received by the RC migrator.
		/// This configuration generally should be influenced by the number of XCM messages sent by
		/// this pallet to the Asset Hub per block and the size of the queue on AH.
		///
		/// This configuration can be overridden by a storage item [`UnprocessedMsgBuffer`].
		type UnprocessedMsgBuffer: Get<u32>;

		/// The timeout for the XCM response.
		type XcmResponseTimeout: Get<BlockNumberFor<Self>>;

		/// Means to force a next queue within the UMPs from different parachains.
		type MessageQueue: ForceSetHead<AggregateMessageOrigin>
			+ EnqueueMessage<AggregateMessageOrigin>;

		/// The priority pattern for AH UMP queue processing during migration
		/// [Config::MessageQueue].
		///
		/// This configures how frequently the AH UMP queue gets priority over other UMP queues.
		/// The tuple (ah_ump_priority_blocks, round_robin_blocks) defines a repeating cycle where:
		/// - `ah_ump_priority_blocks` consecutive blocks: AH UMP queue gets priority
		/// - `round_robin_blocks` consecutive blocks: round-robin processing of all queues
		/// - Then the cycle repeats
		///
		/// For example, (18, 2) means a cycle of 20 blocks that repeats.
		///
		/// This configuration can be overridden by a storage item [`AhUmpQueuePriorityConfig`].
		type AhUmpQueuePriorityPattern: Get<(BlockNumberFor<Self>, BlockNumberFor<Self>)>;

		/// Members of a multisig that can be submit unsigned txs and act as the manager.
		type MultisigMembers: Get<Vec<AccountId32>>;

		/// Threshold of `MultisigMembers`.
		type MultisigThreshold: Get<u32>;

		/// Limit the number of votes of each participant per round.
		type MultisigMaxVotesPerRound: Get<u32>;
	}

	#[pallet::error]
	pub enum Error<T> {
		Unreachable,
		OutOfWeight,
		/// Failed to send XCM message to AH.
		XcmError,
		/// Failed to withdraw account from RC for migration to AH.
		FailedToWithdrawAccount,
		/// Indicates that the specified block number is in the past.
		PastBlockNumber,
		/// Indicates that there is not enough time for staking to lock.
		///
		/// Schedule the migration at least two sessions before the current era ends.
		EraEndsTooSoon,
		/// Balance accounting overflow.
		BalanceOverflow,
		/// Balance accounting underflow.
		BalanceUnderflow,
		/// The query response is invalid.
		InvalidQueryResponse,
		/// The xcm query was not found.
		QueryNotFound,
		/// Failed to send XCM message.
		XcmSendError,
		/// The migration stage is not reachable from the current stage.
		UnreachableStage,
		/// Invalid parameter.
		InvalidParameter,
		/// The AH UMP queue priority configuration is already set.
		AhUmpQueuePriorityAlreadySet,
		/// The account is referenced by some other pallet. It might have freezes or holds.
		AccountReferenced,
		/// The XCM version is invalid.
		BadXcmVersion,
		/// The origin is invalid.
		InvalidOrigin,
		/// The stage transition is invalid.
		InvalidStageTransition,
		/// Unsigned validation failed.
		UnsignedValidationFailed,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A stage transition has occurred.
		StageTransition {
			/// The old stage before the transition.
			old: MigrationStageOf<T>,
			/// The new stage after the transition.
			new: MigrationStageOf<T>,
		},
		/// The Asset Hub Migration started and is active until `AssetHubMigrationFinished` is
		/// emitted.
		///
		/// This event is equivalent to `StageTransition { new: Initializing, .. }` but is easier
		/// to understand. The activation is immediate and affects all events happening
		/// afterwards.
		AssetHubMigrationStarted,
		/// The Asset Hub Migration finished.
		///
		/// This event is equivalent to `StageTransition { new: MigrationDone, .. }` but is easier
		/// to understand. The finishing is immediate and affects all events happening
		/// afterwards.
		AssetHubMigrationFinished,
		/// A query response has been received.
		QueryResponseReceived {
			/// The query ID.
			query_id: u64,
			/// The response.
			response: MaybeErrorCode,
		},
		/// A XCM message has been resent.
		XcmResendAttempt {
			/// The query ID.
			query_id: u64,
			/// The error message.
			send_error: Option<SendError>,
		},
		/// The unprocessed message buffer size has been set.
		UnprocessedMsgBufferSet {
			/// The new size.
			new: u32,
			/// The old size.
			old: u32,
		},
		/// Whether the AH UMP queue was prioritized for the next block.
		AhUmpQueuePrioritySet {
			/// Indicates if AH UMP queue was successfully set as priority.
			/// If `false`, it means we're in the round-robin phase of our priority pattern
			/// (see [`Config::AhUmpQueuePriorityPattern`]), where no queue gets priority.
			prioritized: bool,
			/// Current block number within the pattern cycle (1 to period).
			cycle_block: BlockNumberFor<T>,
			/// Total number of blocks in the pattern cycle
			cycle_period: BlockNumberFor<T>,
		},
		/// The AH UMP queue priority config was set.
		AhUmpQueuePriorityConfigSet {
			/// The old priority pattern.
			old: AhUmpQueuePriority<BlockNumberFor<T>>,
			/// The new priority pattern.
			new: AhUmpQueuePriority<BlockNumberFor<T>>,
		},
		/// The total issuance was recorded.
		MigratedBalanceRecordSet { kept: T::Balance, migrated: T::Balance },
		/// The RC kept balance was consumed.
		MigratedBalanceConsumed { kept: T::Balance, migrated: T::Balance },
		/// The manager account id was set.
		ManagerSet {
			/// The old manager account id.
			old: Option<T::AccountId>,
			/// The new manager account id.
			new: Option<T::AccountId>,
		},
		/// An XCM message was sent.
		XcmSent { origin: Location, destination: Location, message: Xcm<()>, message_id: XcmHash },
		/// The staking elections were paused.
		StakingElectionsPaused,
		/// The accounts to be preserved on Relay Chain were set.
		AccountsPreserved {
			/// The accounts that will be preserved.
			accounts: Vec<T::AccountId>,
		},
		/// The canceller account id was set.
		CancellerSet {
			/// The old canceller account id.
			old: Option<T::AccountId>,
			/// The new canceller account id.
			new: Option<T::AccountId>,
		},
		/// The migration was paused.
		MigrationPaused {
			/// The stage at which the migration was paused.
			pause_stage: MigrationStageOf<T>,
		},
		/// The migration was cancelled.
		MigrationCancelled,
		/// Some pure accounts were indexed for possibly receiving free `Any` proxies.
		PureAccountsIndexed {
			/// The number of indexed pure accounts.
			num_pure_accounts: u32,
		},
		/// The manager multisig dispatched something.
		ManagerMultisigDispatched { res: DispatchResult },
		/// The manager multisig received a vote.
		ManagerMultisigVoted { votes: u32 },
		/// The migration settings were set.
		MigrationSettingsSet {
			/// The old migration settings.
			old: Option<MigrationSettings>,
			/// The new migration settings.
			new: Option<MigrationSettings>,
		},
	}

	/// The Relay Chain migration state.
	#[pallet::storage]
	pub type RcMigrationStage<T: Config> = StorageValue<_, MigrationStageOf<T>, ValueQuery>;

	/// Helper storage item to obtain and store the known accounts that should be kept partially or
	/// fully on Relay Chain.
	#[pallet::storage]
	pub type RcAccounts<T: Config> = CountedStorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		accounts::AccountState<T::Balance>,
		OptionQuery,
	>;

	/// Helper storage item to store the total balance that should be kept on Relay Chain.
	#[pallet::storage]
	pub type RcMigratedBalance<T: Config> =
		StorageValue<_, MigratedBalances<T::Balance>, ValueQuery>;

	/// Helper storage item to store the total balance that should be kept on Relay Chain after
	/// it is consumed from the `RcMigratedBalance` storage item and sent to the Asset Hub.
	///
	/// This let us to take the value from the `RcMigratedBalance` storage item and keep the
	/// `SignalMigrationFinish` stage to be idempotent while preserving these values for tests and
	/// later discoveries.
	#[pallet::storage]
	pub type RcMigratedBalanceArchive<T: Config> =
		StorageValue<_, MigratedBalances<T::Balance>, ValueQuery>;

	/// The pending XCM messages.
	///
	/// Contains data messages that have been sent to the Asset Hub but not yet confirmed.
	///
	/// Unconfirmed messages can be resent by calling the [`Pallet::resend_xcm`] function.
	#[pallet::storage]
	#[pallet::unbounded]
	pub type PendingXcmMessages<T: Config> =
		CountedStorageMap<_, Twox64Concat, (QueryId, T::Hash), Xcm<()>, OptionQuery>;

	/// Accounts that use the proxy pallet to delegate permissions and have no nonce.
	///
	/// Boolean value is whether they have been migrated to the Asset Hub. Needed for idempotency.
	#[pallet::storage]
	pub type PureProxyCandidatesMigrated<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, bool, OptionQuery>;

	/// The pending XCM response queries and their XCM hash referencing the message in the
	/// [`PendingXcmMessages`] storage.
	///
	/// The `QueryId` is the identifier from the [`pallet_xcm`] query handler registry. The XCM
	/// pallet will notify about the status of the message by calling the
	/// [`Pallet::receive_query_response`] function with the `QueryId` and the
	/// response.
	#[pallet::storage]
	pub type PendingXcmQueries<T: Config> =
		StorageMap<_, Twox64Concat, QueryId, T::Hash, OptionQuery>;

	/// Manual override for `type UnprocessedMsgBuffer: Get<u32>`. Look there for docs.
	#[pallet::storage]
	pub type UnprocessedMsgBuffer<T: Config> = StorageValue<_, u32, OptionQuery>;

	/// The priority of the Asset Hub UMP queue during migration.
	///
	/// Controls how the Asset Hub UMP (Upward Message Passing) queue is processed relative to other
	/// queues during the migration process. This helps ensure timely processing of migration
	/// messages. The default priority pattern is defined in the pallet configuration, but can be
	/// overridden by a storage value of this type.
	#[pallet::storage]
	pub type AhUmpQueuePriorityConfig<T: Config> =
		StorageValue<_, AhUmpQueuePriority<BlockNumberFor<T>>, ValueQuery>;

	/// An optional account id of a manager.
	///
	/// This account id has similar privileges to [`Config::AdminOrigin`] except that it
	/// can not set the manager account id via `set_manager` call.
	#[pallet::storage]
	pub type Manager<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// An optional account id of a canceller.
	///
	/// This account id can only stop scheduled migration.
	#[pallet::storage]
	pub type Canceller<T: Config> = StorageValue<_, T::AccountId, OptionQuery>;

	/// The block number at which the migration began and the pallet's extrinsics were locked.
	///
	/// This value is set when entering the `WaitingForAh` stage, i.e., when
	/// `RcMigrationStage::is_ongoing()` becomes `true`.
	#[pallet::storage]
	pub type MigrationStartBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// Block number when migration finished and extrinsics were unlocked.
	///
	/// This is set when entering the `MigrationDone` stage hence when
	/// `RcMigrationStage::is_finished()` becomes `true`.
	#[pallet::storage]
	pub type MigrationEndBlock<T: Config> = StorageValue<_, BlockNumberFor<T>, OptionQuery>;

	/// The duration of the pre migration warm-up period.
	///
	/// This is the duration of the warm-up period before the data migration starts. During this
	/// period, the migration will be in ongoing state and the concerned extrinsics will be locked.
	#[pallet::storage]
	pub type WarmUpPeriod<T: Config> =
		StorageValue<_, DispatchTime<BlockNumberFor<T>>, OptionQuery>;

	/// The duration of the post migration cool-off period.
	///
	/// This is the duration of the cool-off period after the data migration is finished. During
	/// this period, the migration will be still in ongoing state and the concerned extrinsics will
	/// be locked.
	#[pallet::storage]
	pub type CoolOffPeriod<T: Config> =
		StorageValue<_, DispatchTime<BlockNumberFor<T>>, OptionQuery>;

	/// The migration settings.
	#[pallet::storage]
	pub type Settings<T: Config> = StorageValue<_, MigrationSettings, OptionQuery>;

	/// Alias for `Paras` from `paras_registrar`.
	///
	/// The fields of the type stored in the original storage item are private, so we define the
	/// storage alias to get an access to them.
	#[frame_support::storage_alias(pallet_name)]
	pub type Paras<T: Config> = StorageMap<
		ParasRegistrar<T>,
		Twox64Concat,
		ParaId,
		types::ParaInfo<
			<T as frame_system::Config>::AccountId,
			<T as pallet_balances::Config>::Balance,
		>,
	>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(
		Encode,
		Decode,
		DebugNoBound,
		CloneNoBound,
		PartialEqNoBound,
		EqNoBound,
		TypeInfo,
		sp_core::DecodeWithMemTracking,
	)]
	#[scale_info(skip_type_params(T))]
	pub struct ManagerMultisigVote<T: Config> {
		who: sp_runtime::MultiSigner,
		call: <T as Config>::RuntimeCall,
		round: u32,
	}

	impl<T: Config> ManagerMultisigVote<T> {
		pub fn new(
			who: sp_runtime::MultiSigner,
			call: <T as Config>::RuntimeCall,
			round: u32,
		) -> Self {
			Self { who, call, round }
		}

		pub fn encode_with_bytes_wrapper(&self) -> Vec<u8> {
			(b"<Bytes>", self, b"</Bytes>").encode()
		}
	}

	/// The multisig AccountIDs that votes to execute a specific call.
	#[pallet::storage]
	#[pallet::unbounded]
	pub type ManagerMultisigs<T: Config> =
		StorageMap<_, Twox64Concat, <T as Config>::RuntimeCall, Vec<AccountId32>, ValueQuery>;

	/// The current round of the multisig voting.
	///
	/// Votes are only valid for the current round.
	#[pallet::storage]
	pub type ManagerMultisigRound<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// How often each participant voted in the current round.
	///
	/// Will be cleared at the end of each round.
	#[pallet::storage]
	pub type ManagerVotesInCurrentRound<T: Config> =
		StorageMap<_, Blake2_128Concat, AccountId32, u32, ValueQuery>;

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::vote_manager_multisig { payload, sig } = call {
				Self::do_validate_unsigned(payload, sig)
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}
}

/// Returns the weight for a single item in a batch.
///
/// If the next item in the batch is the first one, it includes the base weight of the
/// `weight_of`, otherwise, it does not.
pub fn item_weight_of(weight_of: impl Fn(u32) -> Weight, batch_len: u32) -> Weight {
	if batch_len == 0 {
		weight_of(1)
	} else {
		weight_of(1).saturating_sub(weight_of(0))
	}
}

impl<T: Config> Contains<<T as frame_system::Config>::RuntimeCall> for Pallet<T> {
	fn contains(call: &<T as frame_system::Config>::RuntimeCall) -> bool {
		let stage = RcMigrationStage::<T>::get();

		// We have to return whether the call is allowed:
		const ALLOWED: bool = true;
		const FORBIDDEN: bool = false;

		// Once the migration is finished, forbid calls not in the `RcPostMigrationCalls` set.
		if stage.is_finished() && !T::RcPostMigrationCalls::contains(call) {
			return FORBIDDEN;
		}

		// If the migration is ongoing, forbid calls not in the `RcIntraMigrationCalls` set.
		if stage.is_ongoing() && !T::RcIntraMigrationCalls::contains(call) {
			return FORBIDDEN;
		}

		// Otherwise, allow the call.
		// This also implicitly allows _any_ call if the migration has not yet started.
		ALLOWED
	}
}
