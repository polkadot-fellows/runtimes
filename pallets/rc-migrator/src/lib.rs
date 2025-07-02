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
pub mod conviction_voting;
pub mod scheduler;
pub mod treasury;
pub mod xcm_config;

pub use weights::*;

use crate::{
	accounts::MigratedBalances,
	types::{MigrationFinishedData, XcmBatch, XcmBatchAndMeter},
};
use accounts::AccountsMigrator;
use claims::{ClaimsMigrator, ClaimsStage};
use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::AccountIdConversion,
	storage::transactional::with_transaction_opaque_err,
	traits::{
		fungible::{Inspect, InspectFreeze, Mutate, MutateFreeze, MutateHold},
		schedule::DispatchTime,
		tokens::{Fortitude, Pay, Precision, Preservation},
		Contains, Defensive, DefensiveTruncateFrom, LockableCurrency, ReservableCurrency,
		VariantCount,
	},
	weights::{Weight, WeightMeter},
	PalletId,
};
use frame_system::{pallet_prelude::*, AccountInfo};
use indices::IndicesMigrator;
use multisig::MultisigMigrator;
use pallet_balances::AccountData;
use polkadot_parachain_primitives::primitives::Id as ParaId;
use polkadot_runtime_common::{
	claims as pallet_claims, crowdloan as pallet_crowdloan, paras_registrar, slots as pallet_slots,
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
	traits::{BadOrigin, One, Zero},
	AccountId32,
};
use sp_std::prelude::*;
use staking::{
	bags_list::{BagsListMigrator, BagsListStage},
	delegated_staking::{DelegatedStakingMigrator, DelegatedStakingStage},
	fast_unstake::{FastUnstakeMigrator, FastUnstakeStage},
	nom_pools::{NomPoolsMigrator, NomPoolsStage},
};
use storage::TransactionOutcome;
pub use types::{
	ForceSetHead, MigrationStatus, PalletMigration, QueuePriority as AhUmpQueuePriority,
};
use vesting::VestingMigrator;
use weights_ah::WeightInfo as AhWeightInfo;
use xcm::prelude::*;
use xcm_builder::MintLocation;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::rc-migrator";

/// Soft limit on the DMP message size.
///
/// The hard limit should be about 64KiB (TODO test) which means that we stay well below that to
/// avoid any trouble. We can raise this as final preparation for the migration once everything is
/// confirmed to work.
pub const MAX_XCM_SIZE: u32 = 50_000;

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

#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum PalletEventName {
	FastUnstake,
	BagsList,
}

pub type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	PartialEq,
	Eq,
)]
pub enum MigrationStage<
	AccountId,
	BlockNumber,
	BagsListScore,
	VotingClass,
	AssetKind,
	SchedulerBlockNumber,
> {
	/// The migration has not yet started but will start in the future.
	#[default]
	Pending,
	/// The migration has been scheduled to start at the given block number.
	Scheduled {
		/// The block number at which the migration will start.
		///
		/// The block number at which we notify the Asset Hub about the start of the migration and
		/// move to `WaitingForAH` stage. After we receive the confirmation, the Relay Chain will
		/// enter the `CoolOff` stage and wait for the cool-off period to end (`cool_off_end`)
		/// before starting to send the migration data to the Asset Hub.
		start: BlockNumber,
		/// The block number at which the cool-off period will end.
		///
		/// After the cool-off period ends, the Relay Chain will start to send the migration data
		/// to the Asset Hub.
		cool_off_end: BlockNumber,
	},
	/// The migration is waiting for confirmation from AH to go ahead.
	///
	/// This stage involves waiting for the notification from the Asset Hub that it is ready to
	/// receive the migration data.
	WaitingForAh {
		/// The block number at which the cool-off period will end.
		///
		/// After the cool-off period ends, the Relay Chain will start to send the migration data
		/// to the Asset Hub.
		cool_off_end: BlockNumber,
	},
	CoolOff {
		/// The block number at which the cool-off period will end.
		///
		/// After the cool-off period ends, the Relay Chain will start to send the migration data
		/// to the Asset Hub.
		cool_off_end: BlockNumber,
	},
	/// The migration is starting and initialization hooks are being executed.
	Starting,
	/// Initializing the account migration process.
	AccountsMigrationInit,
	/// Migrating account balances.
	AccountsMigrationOngoing {
		// Last migrated account
		last_key: Option<AccountId>,
	},
	/// Note that this stage does not have any logic attached to itself. It just exists to make it
	/// easier to swap out what stage should run next for testing.
	AccountsMigrationDone,

	MultisigMigrationInit,
	MultisigMigrationOngoing {
		/// Last migrated key of the `Multisigs` double map.
		last_key: Option<(AccountId, [u8; 32])>,
	},
	MultisigMigrationDone,
	ClaimsMigrationInit,
	ClaimsMigrationOngoing {
		current_key: Option<ClaimsStage<AccountId>>,
	},
	ClaimsMigrationDone,

	ProxyMigrationInit,
	/// Currently migrating the proxies of the proxy pallet.
	ProxyMigrationProxies {
		last_key: Option<AccountId>,
	},
	/// Currently migrating the announcements of the proxy pallet.
	ProxyMigrationAnnouncements {
		last_key: Option<AccountId>,
	},
	ProxyMigrationDone,

	PreimageMigrationInit,
	PreimageMigrationChunksOngoing {
		// TODO type
		last_key: Option<((H256, u32), u32)>,
	},
	PreimageMigrationChunksDone,
	PreimageMigrationRequestStatusOngoing {
		next_key: Option<H256>,
	},
	PreimageMigrationRequestStatusDone,
	PreimageMigrationLegacyRequestStatusInit,
	PreimageMigrationLegacyRequestStatusOngoing {
		next_key: Option<H256>,
	},
	PreimageMigrationLegacyRequestStatusDone,
	PreimageMigrationDone,

	NomPoolsMigrationInit,
	NomPoolsMigrationOngoing {
		next_key: Option<NomPoolsStage<AccountId>>,
	},
	NomPoolsMigrationDone,

	VestingMigrationInit,
	VestingMigrationOngoing {
		next_key: Option<AccountId>,
	},
	VestingMigrationDone,

	FastUnstakeMigrationInit,
	FastUnstakeMigrationOngoing {
		next_key: Option<FastUnstakeStage<AccountId>>,
	},
	FastUnstakeMigrationDone,

	DelegatedStakingMigrationInit,
	DelegatedStakingMigrationOngoing {
		next_key: Option<DelegatedStakingStage<AccountId>>,
	},
	DelegatedStakingMigrationDone,

	IndicesMigrationInit,
	IndicesMigrationOngoing {
		next_key: Option<()>,
	},
	IndicesMigrationDone,

	ReferendaMigrationInit,
	ReferendaMigrationOngoing {
		last_key: Option<ReferendaStage>,
	},
	ReferendaMigrationDone,

	BagsListMigrationInit,
	BagsListMigrationOngoing {
		next_key: Option<BagsListStage<AccountId, BagsListScore>>,
	},
	BagsListMigrationDone,
	SchedulerMigrationInit,
	SchedulerMigrationOngoing {
		last_key: Option<scheduler::SchedulerStage<SchedulerBlockNumber>>,
	},
	SchedulerAgendaMigrationOngoing {
		last_key: Option<SchedulerBlockNumber>,
	},
	SchedulerMigrationDone,
	ConvictionVotingMigrationInit,
	ConvictionVotingMigrationOngoing {
		last_key: Option<conviction_voting::ConvictionVotingStage<AccountId, VotingClass>>,
	},
	ConvictionVotingMigrationDone,
	BountiesMigrationInit,
	BountiesMigrationOngoing {
		last_key: Option<bounties::BountiesStage>,
	},
	BountiesMigrationDone,

	AssetRateMigrationInit,
	AssetRateMigrationOngoing {
		last_key: Option<AssetKind>,
	},
	AssetRateMigrationDone,
	CrowdloanMigrationInit,
	CrowdloanMigrationOngoing {
		last_key: Option<crowdloan::CrowdloanStage>,
	},
	CrowdloanMigrationDone,
	TreasuryMigrationInit,
	TreasuryMigrationOngoing {
		last_key: Option<treasury::TreasuryStage>,
	},
	TreasuryMigrationDone,

	#[cfg(feature = "ahm-staking-migration")]
	StakingMigrationInit,
	#[cfg(feature = "ahm-staking-migration")]
	StakingMigrationOngoing {
		next_key: Option<staking::StakingStage<AccountId>>,
	},
	StakingMigrationDone,

	SignalMigrationFinish,
	MigrationDone,
}

impl<AccountId, BlockNumber, BagsListScore, VotingClass, AssetKind, SchedulerBlockNumber>
	MigrationStage<AccountId, BlockNumber, BagsListScore, VotingClass, AssetKind, SchedulerBlockNumber>
{
	/// Whether the migration is finished.
	///
	/// This is **not** the same as `!self.is_ongoing()` since it may not have started.
	pub fn is_finished(&self) -> bool {
		matches!(self, MigrationStage::MigrationDone)
	}

	/// Whether the migration is ongoing.
	///
	/// This is **not** the same as `!self.is_finished()` since it may not have started.
	pub fn is_ongoing(&self) -> bool {
		!matches!(
			self,
			MigrationStage::Pending |
				MigrationStage::Scheduled { .. } |
				MigrationStage::MigrationDone
		)
	}
}

#[cfg(feature = "std")]
impl<AccountId, BlockNumber, BagsListScore, VotingClass, AssetKind, SchedulerBlockNumber>
	std::str::FromStr
	for MigrationStage<
		AccountId,
		BlockNumber,
		BagsListScore,
		VotingClass,
		AssetKind,
		SchedulerBlockNumber,
	>
{
	type Err = std::string::String;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(match s {
			"skip-accounts" => MigrationStage::AccountsMigrationDone,
			"crowdloan" => MigrationStage::CrowdloanMigrationInit,
			"preimage" => MigrationStage::PreimageMigrationInit,
			"referenda" => MigrationStage::ReferendaMigrationInit,
			"multisig" => MigrationStage::MultisigMigrationInit,
			"voting" => MigrationStage::ConvictionVotingMigrationInit,
			"bounties" => MigrationStage::BountiesMigrationInit,
			"asset_rate" => MigrationStage::AssetRateMigrationInit,
			"indices" => MigrationStage::IndicesMigrationInit,
			"treasury" => MigrationStage::TreasuryMigrationInit,
			"proxy" => MigrationStage::ProxyMigrationInit,
			"nom_pools" => MigrationStage::NomPoolsMigrationInit,
			"scheduler" => MigrationStage::SchedulerMigrationInit,
			other => return Err(format!("Unknown migration stage: {}", other)),
		})
	}
}

type AccountInfoFor<T> =
	AccountInfo<<T as frame_system::Config>::Nonce, <T as frame_system::Config>::AccountData>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Paras Registrar Pallet
	type ParasRegistrar<T> = paras_registrar::Pallet<T>;

	/// Super config trait for all pallets that the migration depends on, providing convenient
	/// access to their items.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountData = AccountData<u128>, AccountId = AccountId32>
		+ pallet_balances::Config<
			RuntimeHoldReason = <Self as Config>::RuntimeHoldReason,
			Balance = u128,
		> + hrmp::Config
		+ paras_registrar::Config
		+ pallet_multisig::Config
		+ pallet_proxy::Config
		+ pallet_preimage::Config<Hash = H256>
		+ pallet_referenda::Config<Votes = u128>
		+ pallet_nomination_pools::Config
		+ pallet_fast_unstake::Config
		+ pallet_bags_list::Config<pallet_bags_list::Instance1>
		+ pallet_scheduler::Config
		+ pallet_vesting::Config
		+ pallet_indices::Config
		+ pallet_conviction_voting::Config
		+ pallet_asset_rate::Config
		+ pallet_slots::Config
		+ pallet_crowdloan::Config
		+ pallet_staking::Config
		+ pallet_claims::Config
		+ pallet_bounties::Config
		+ pallet_treasury::Config
		+ pallet_delegated_staking::Config
		+ pallet_xcm::Config
	{
		/// The overall runtime origin type.
		type RuntimeOrigin: Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>
			+ IsType<<Self as frame_system::Config>::RuntimeOrigin>;
		/// The overall runtime call type.
		type RuntimeCall: From<Call<Self>> + IsType<<Self as pallet_xcm::Config>::RuntimeCall>;
		/// The runtime hold reasons.
		type RuntimeHoldReason: Parameter + VariantCount;
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The origin that can perform permissioned operations like setting the migration stage.
		///
		/// This is generally root, Asset Hub and Fellows origins.
		type ManagerOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Native asset registry type.
		type Currency: Mutate<Self::AccountId, Balance = u128>
			+ MutateHold<Self::AccountId, Reason = <Self as Config>::RuntimeHoldReason>
			+ InspectFreeze<Self::AccountId, Id = Self::FreezeIdentifier>
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
		type MessageQueue: ForceSetHead<AggregateMessageOrigin>;

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
	}

	/// The Relay Chain migration state.
	#[pallet::storage]
	pub type RcMigrationStage<T: Config> = StorageValue<_, MigrationStageOf<T>, ValueQuery>;

	/// Helper storage item to obtain and store the known accounts that should be kept partially or
	/// fully on Relay Chain.
	#[pallet::storage]
	pub type RcAccounts<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, accounts::AccountState<T::Balance>, OptionQuery>;

	/// Helper storage item to store the total balance that should be kept on Relay Chain.
	#[pallet::storage]
	pub type RcMigratedBalance<T: Config> =
		StorageValue<_, MigratedBalances<T::Balance>, ValueQuery>;

	/// The pending XCM messages.
	///
	/// Contains data messages that have been sent to the Asset Hub but not yet confirmed.
	/// The `QueryId` is the identifier from the [`pallet_xcm`] query handler registry. The XCM
	/// pallet will notify about the status of the message by calling the
	/// [`Pallet::receive_query_response`] function with the `QueryId` and the
	/// response.
	///
	/// Unconfirmed messages can be resent by calling the [`Pallet::resend_xcm`] function.
	#[pallet::storage]
	#[pallet::unbounded]
	pub type PendingXcmMessages<T: Config> =
		CountedStorageMap<_, Twox64Concat, QueryId, Xcm<()>, OptionQuery>;

	/// The DMP queue priority.
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

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Set the migration stage.
		///
		/// This call is intended for emergency use only and is guarded by the
		/// [`Config::ManagerOrigin`].
		#[pallet::call_index(0)]
		#[pallet::weight(T::RcWeightInfo::force_set_stage())]
		pub fn force_set_stage(
			origin: OriginFor<T>,
			stage: Box<MigrationStageOf<T>>,
		) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;
			Self::transition(*stage);
			Ok(())
		}

		/// Schedule the migration to start at a given moment.
		///
		/// ### Parameters:
		/// - `start`: The block number at which the migration will start.
		/// - `cool_off_end`: The block number at which the cool-off period will end.
		///
		/// Read [`MigrationStage::Scheduled`] documentation for more details.
		#[pallet::call_index(1)]
		#[pallet::weight(T::RcWeightInfo::schedule_migration())]
		pub fn schedule_migration(
			origin: OriginFor<T>,
			start: DispatchTime<BlockNumberFor<T>>,
			cool_off_end: DispatchTime<BlockNumberFor<T>>,
		) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;
			let now = frame_system::Pallet::<T>::block_number();
			let start = start.evaluate(now);
			let cool_off_end = cool_off_end.evaluate(now);
			ensure!(start > now, Error::<T>::PastBlockNumber);
			ensure!(cool_off_end > start, Error::<T>::PastBlockNumber);
			Self::transition(MigrationStage::Scheduled { start, cool_off_end });
			Ok(())
		}

		/// Start the data migration.
		///
		/// This is typically called by the Asset Hub to indicate it's readiness to receive the
		/// migration data.
		#[pallet::call_index(2)]
		#[pallet::weight(T::RcWeightInfo::start_data_migration())]
		pub fn start_data_migration(origin: OriginFor<T>) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;
			let cool_off_end = match RcMigrationStage::<T>::get() {
				MigrationStage::WaitingForAh { cool_off_end } => cool_off_end,
				stage => {
					defensive!("start_data_migration called in invalid stage: {:?}", stage);
					return Err(Error::<T>::UnreachableStage.into())
				},
			};
			Self::transition(MigrationStage::CoolOff { cool_off_end });
			Ok(())
		}

		/// Receive a query response from the Asset Hub for a previously sent xcm message.
		#[pallet::call_index(3)]
		#[pallet::weight(T::RcWeightInfo::receive_query_response())]
		pub fn receive_query_response(
			origin: OriginFor<T>,
			query_id: QueryId,
			response: Response,
		) -> DispatchResult {
			match <T as Config>::ManagerOrigin::ensure_origin(origin.clone()) {
				Ok(_) => {
					// Origin is valid [`Config::ManagerOrigin`].
				},
				Err(_) => {
					match <T as Config>::RuntimeOrigin::from(origin.clone()).into() {
						Ok(pallet_xcm::Origin::Response(response_origin))
							if response_origin == Location::new(0, Parachain(1000)) =>
						{
							// Origin is valid - this is a response from Asset Hub
						},
						_ => {
							return Err(BadOrigin.into());
						},
					}
				},
			}

			let response = match response {
				Response::DispatchResult(maybe_error) => maybe_error,
				_ => return Err(Error::<T>::InvalidQueryResponse.into()),
			};

			if matches!(response, MaybeErrorCode::Success) {
				log::info!(
					target: LOG_TARGET,
					"Received success response for query id: {}",
					query_id
				);
				PendingXcmMessages::<T>::remove(query_id);
			} else {
				log::error!(
					target: LOG_TARGET,
					"Received error response for query id: {}; response: {:?}",
					query_id,
					response.clone(),
				);
			}

			Self::deposit_event(Event::<T>::QueryResponseReceived { query_id, response });

			Ok(())
		}

		/// Resend a previously sent and unconfirmed XCM message.
		#[pallet::call_index(4)]
		#[pallet::weight(T::RcWeightInfo::resend_xcm())]
		pub fn resend_xcm(origin: OriginFor<T>, query_id: u64) -> DispatchResultWithPostInfo {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;

			let xcm = PendingXcmMessages::<T>::get(query_id).ok_or(Error::<T>::QueryNotFound)?;

			if let Err(err) = send_xcm::<T::SendXcm>(Location::new(0, Parachain(1000)), xcm) {
				log::error!(target: LOG_TARGET, "Error while sending XCM message: {:?}", err);
				Self::deposit_event(Event::<T>::XcmResendAttempt {
					query_id,
					send_error: Some(err),
				});
			} else {
				Self::deposit_event(Event::<T>::XcmResendAttempt { query_id, send_error: None });
			}

			Ok(Pays::No.into())
		}

		/// Set the unprocessed message buffer size.
		///
		/// `None` means to use the configuration value.
		#[pallet::call_index(5)]
		#[pallet::weight(T::RcWeightInfo::set_unprocessed_msg_buffer())]
		pub fn set_unprocessed_msg_buffer(
			origin: OriginFor<T>,
			new: Option<u32>,
		) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;
			let old = Self::get_unprocessed_msg_buffer_size();
			UnprocessedMsgBuffer::<T>::set(new);
			let new = Self::get_unprocessed_msg_buffer_size();
			Self::deposit_event(Event::UnprocessedMsgBufferSet { new, old });
			Ok(())
		}

		/// Set the AH UMP queue priority configuration.
		///
		/// Can only be called by the `ManagerOrigin`.
		#[pallet::call_index(6)]
		#[pallet::weight(T::RcWeightInfo::set_ah_ump_queue_priority())]
		pub fn set_ah_ump_queue_priority(
			origin: OriginFor<T>,
			new: AhUmpQueuePriority<BlockNumberFor<T>>,
		) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;
			let old = AhUmpQueuePriorityConfig::<T>::get();
			if old == new {
				return Err(Error::<T>::AhUmpQueuePriorityAlreadySet.into());
			}
			ensure!(
				new.get_priority_blocks().map_or(true, |blocks| !blocks.is_zero()),
				Error::<T>::InvalidParameter
			);
			AhUmpQueuePriorityConfig::<T>::put(new.clone());
			Self::deposit_event(Event::AhUmpQueuePriorityConfigSet { old, new });
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T>
		where
		crate::BalanceOf<T>:
			From<<<T as polkadot_runtime_common::slots::Config>::Currency as frame_support::traits::Currency<sp_runtime::AccountId32>>::Balance>,
		crate::BalanceOf<T>:
			From<<<<T as polkadot_runtime_common::crowdloan::Config>::Auctioneer as polkadot_runtime_common::traits::Auctioneer<<<<T as frame_system::Config>::Block as sp_runtime::traits::Block>::Header as sp_runtime::traits::Header>::Number>>::Currency as frame_support::traits::Currency<sp_runtime::AccountId32>>::Balance>,
	{
		fn integrity_test() {
			let (ah_ump_priority_blocks, _) = T::AhUmpQueuePriorityPattern::get();
			assert!(!ah_ump_priority_blocks.is_zero(), "the `ah_ump_priority_blocks` should be non-zero");
		}

		fn on_finalize(now: BlockNumberFor<T>) {
			if Self::is_ongoing() {
				Self::force_ah_ump_queue_priority(now);
			}
		}

		fn on_initialize(now: BlockNumberFor<T>) -> Weight {
			let mut weight_counter = WeightMeter::with_limit(T::MaxRcWeight::get());

			let stage = RcMigrationStage::<T>::get();
			weight_counter.consume(T::DbWeight::get().reads(1));

			if stage.is_ongoing() {
				// account the weight of `on_finalize` for the `force_ah_ump_queue_priority` job.
				weight_counter.consume(T::RcWeightInfo::force_ah_ump_queue_priority());
			}

			if Self::has_excess_unconfirmed_dmp(&stage) {
				log::info!(
					target: LOG_TARGET,
					"Excess unconfirmed XCM messages, skipping the data extraction for this block."
				);
				return weight_counter.consumed();
			}

			match stage {
				MigrationStage::Pending => {
					return weight_counter.consumed();
				},
				MigrationStage::Scheduled { start, cool_off_end } =>
					if now >= start {
/* let current_era = pallet_staking::CurrentEra::<T>::get().defensive_unwrap_or(0);
							let active_era = pallet_staking::ActiveEra::<T>::get().map(|a| a.index).defensive_unwrap_or(0);
							// ensure new era is not planned when starting migration.
							if current_era > active_era {
								defensive!("New era is planned, migration cannot start until it is completed");
								Self::transition(MigrationStage::Pending);
								return weight_counter.consumed();
							}
								*/ // FAIL-CI staking check

						match Self::send_xcm(types::AhMigratorCall::<T>::StartMigration, T::AhWeightInfo::start_migration()) {
							Ok(_) => {
								Self::transition(MigrationStage::WaitingForAh { cool_off_end });
							},
							Err(_) => {
								defensive!(
									"Failed to send StartMigration message to AH, \
									retry with the next block"
								);
							},
						}
					},
				MigrationStage::WaitingForAh { .. } => {
					// waiting AH to send a message and to start sending the data.
					log::debug!(target: LOG_TARGET, "Waiting for AH to start the migration");
					// We transition out here in `start_data_migration`
					return weight_counter.consumed();
				},
				MigrationStage::CoolOff { cool_off_end } => {
					// waiting for the cool-off period to end
					if now >= cool_off_end {
						Self::transition(MigrationStage::Starting);
					} else {
						log::info!(
							target: LOG_TARGET,
							"Waiting for the cool-off period to end, cool_off_end: {:?}",
							cool_off_end,
						);
					}
					return weight_counter.consumed();
				},
				MigrationStage::Starting => {
					log::info!(target: LOG_TARGET, "Starting the migration");
					#[cfg(feature = "ahm-staking-migration")]
					pallet_staking_async_ah_client::Pallet::<T>::on_migration_start();

					Self::transition(MigrationStage::AccountsMigrationInit);
				},
				MigrationStage::AccountsMigrationInit => {
					let weight = AccountsMigrator::<T>::obtain_rc_accounts();
					weight_counter.consume(weight);
					RcMigratedBalance::<T>::mutate(|tracker| {
						// initialize `kept` balance as total issuance, we'll substract from it as
						// we migrate accounts
						tracker.kept = <T as Config>::Currency::total_issuance();
					});

					Self::transition(MigrationStage::AccountsMigrationOngoing { last_key: None });
				},
				MigrationStage::AccountsMigrationOngoing { last_key } => {
					let res =
						with_transaction_opaque_err::<Option<T::AccountId>, Error<T>, _>(|| {
							match AccountsMigrator::<T>::migrate_many(last_key, &mut weight_counter)
							{
								Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
								Err(e) => TransactionOutcome::Rollback(Err(e)),
							}
						})
						.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							// accounts migration is completed
							Self::transition(MigrationStage::AccountsMigrationDone);
						},
						Ok(Some(last_key)) => {
							// accounts migration continues with the next block
							Self::transition(MigrationStage::AccountsMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						Err(err) => {
							defensive!("Error while migrating accounts: {:?}", err);
							// stage unchanged, retry.
						},
					}
				},
				MigrationStage::AccountsMigrationDone => {
					AccountsMigrator::<T>::finish_balances_migration();
					// Note: swap this out for faster testing to skip some migrations
					Self::transition(MigrationStage::MultisigMigrationInit);
				},
				MigrationStage::MultisigMigrationInit => {
					Self::transition(MigrationStage::MultisigMigrationOngoing { last_key: None });
				},
				MigrationStage::MultisigMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match MultisigMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							// multisig migration is completed
							Self::transition(MigrationStage::MultisigMigrationDone);
						},
						Ok(Some(last_key)) => {
							// multisig migration continues with the next block
							Self::transition(MigrationStage::MultisigMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						e => {
							defensive!("Error while migrating multisigs: {:?}", e);
						},
					}
				},
				MigrationStage::MultisigMigrationDone => {
					Self::transition(MigrationStage::ClaimsMigrationInit);
				},
				MigrationStage::ClaimsMigrationInit => {
					Self::transition(MigrationStage::ClaimsMigrationOngoing { current_key: None });
				},
				MigrationStage::ClaimsMigrationOngoing { current_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match ClaimsMigrator::<T>::migrate_many(current_key, &mut weight_counter) {
							Ok(current_key) => TransactionOutcome::Commit(Ok(current_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::ClaimsMigrationDone);
						},
						Ok(Some(current_key)) => {
							Self::transition(MigrationStage::ClaimsMigrationOngoing {
								current_key: Some(current_key),
							});
						},
						e => {
							defensive!("Error while migrating claims: {:?}", e);
						},
					}
				},
				MigrationStage::ClaimsMigrationDone => {
					Self::transition(MigrationStage::ProxyMigrationInit);
				},
				MigrationStage::ProxyMigrationInit => {
					Self::transition(MigrationStage::ProxyMigrationProxies { last_key: None });
				},
				MigrationStage::ProxyMigrationProxies { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match ProxyProxiesMigrator::<T>::migrate_many(last_key, &mut weight_counter)
						{
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::ProxyMigrationAnnouncements {
								last_key: None,
							});
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::ProxyMigrationProxies {
								last_key: Some(last_key),
							});
						},
						e => {
							defensive!("Error while migrating proxies: {:?}", e);
						},
					}
				},
				MigrationStage::ProxyMigrationAnnouncements { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match ProxyAnnouncementMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::ProxyMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::ProxyMigrationAnnouncements {
								last_key: Some(last_key),
							});
						},
						e => {
							defensive!("Error while migrating proxy announcements: {:?}", e);
						},
					}
				},
				MigrationStage::ProxyMigrationDone => {
					Self::transition(MigrationStage::PreimageMigrationInit);
				},
				MigrationStage::PreimageMigrationInit => {
					Self::transition(MigrationStage::PreimageMigrationChunksOngoing {
						last_key: None,
					});
				},
				MigrationStage::PreimageMigrationChunksOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match PreimageChunkMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::PreimageMigrationChunksDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::PreimageMigrationChunksOngoing {
								last_key: Some(last_key),
							});
						},
						e => {
							defensive!("Error while migrating preimages: {:?}", e);
						},
					}
				},
				MigrationStage::PreimageMigrationChunksDone => {
					Self::transition(MigrationStage::PreimageMigrationRequestStatusOngoing {
						next_key: None,
					});
				},
				MigrationStage::PreimageMigrationRequestStatusOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match PreimageRequestStatusMigrator::<T>::migrate_many(
							next_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::PreimageMigrationRequestStatusDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(
								MigrationStage::PreimageMigrationRequestStatusOngoing {
									next_key: Some(next_key),
								},
							);
						},
						e => {
							defensive!("Error while migrating preimage request status: {:?}", e);
						},
					}
				},
				MigrationStage::PreimageMigrationRequestStatusDone => {
					Self::transition(MigrationStage::PreimageMigrationLegacyRequestStatusInit);
				},
				MigrationStage::PreimageMigrationLegacyRequestStatusInit => {
					Self::transition(MigrationStage::PreimageMigrationLegacyRequestStatusOngoing {
						next_key: None,
					});
				},
				MigrationStage::PreimageMigrationLegacyRequestStatusOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match PreimageLegacyRequestStatusMigrator::<T>::migrate_many(
							next_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(
								MigrationStage::PreimageMigrationLegacyRequestStatusDone,
							);
						},
						Ok(Some(next_key)) => {
							Self::transition(
								MigrationStage::PreimageMigrationLegacyRequestStatusOngoing {
									next_key: Some(next_key),
								},
							);
						},
						e => {
							defensive!(
								"Error while migrating legacy preimage request status: {:?}",
								e
							);
						},
					}
				},
				MigrationStage::PreimageMigrationLegacyRequestStatusDone => {
					Self::transition(MigrationStage::PreimageMigrationDone);
				},
				MigrationStage::PreimageMigrationDone => {
					Self::transition(MigrationStage::NomPoolsMigrationInit);
				},
				MigrationStage::NomPoolsMigrationInit => {
					Self::transition(MigrationStage::NomPoolsMigrationOngoing { next_key: None });
				},
				MigrationStage::NomPoolsMigrationOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match NomPoolsMigrator::<T>::migrate_many(next_key, &mut weight_counter) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::NomPoolsMigrationDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(MigrationStage::NomPoolsMigrationOngoing {
								next_key: Some(next_key),
							});
						},
						e => {
							defensive!("Error while migrating nom pools: {:?}", e);
						},
					}
				},
				MigrationStage::NomPoolsMigrationDone => {
					Self::transition(MigrationStage::VestingMigrationInit);
				},

				MigrationStage::VestingMigrationInit => {
					Self::transition(MigrationStage::VestingMigrationOngoing { next_key: None });
				},
				MigrationStage::VestingMigrationOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match VestingMigrator::<T>::migrate_many(next_key, &mut weight_counter) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::VestingMigrationDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(MigrationStage::VestingMigrationOngoing {
								next_key: Some(next_key),
							});
						},
						e => {
							defensive!("Error while migrating vesting: {:?}", e);
						},
					}
				},
				MigrationStage::VestingMigrationDone => {
					Self::transition(MigrationStage::FastUnstakeMigrationInit);
				},
				MigrationStage::FastUnstakeMigrationInit => {
					Self::transition(MigrationStage::FastUnstakeMigrationOngoing {
						next_key: None,
					});
				},
				MigrationStage::FastUnstakeMigrationOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match FastUnstakeMigrator::<T>::migrate_many(next_key, &mut weight_counter)
						{
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::FastUnstakeMigrationDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(MigrationStage::FastUnstakeMigrationOngoing {
								next_key: Some(next_key),
							});
						},
						e => {
							defensive!("Error while migrating fast unstake: {:?}", e);
						},
					}
				},
				MigrationStage::FastUnstakeMigrationDone => {
					Self::transition(MigrationStage::DelegatedStakingMigrationInit);
				},
				MigrationStage::DelegatedStakingMigrationInit => {
					Self::transition(MigrationStage::DelegatedStakingMigrationOngoing {
						next_key: None,
					});
				},
				MigrationStage::DelegatedStakingMigrationOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match DelegatedStakingMigrator::<T>::migrate_many(next_key, &mut weight_counter)
						{
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::DelegatedStakingMigrationDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(MigrationStage::DelegatedStakingMigrationOngoing {
								next_key: Some(next_key),
							});
						},
						e => {
							defensive!("Error while migrating delegated staking: {:?}", e);
						},
					}
				},
				MigrationStage::DelegatedStakingMigrationDone => {
					Self::transition(MigrationStage::IndicesMigrationInit);
				},
				MigrationStage::IndicesMigrationInit => {
					Self::transition(MigrationStage::IndicesMigrationOngoing {
						next_key: Some(Default::default()),
					});
				},
				MigrationStage::IndicesMigrationOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match IndicesMigrator::<T>::migrate_many(next_key, &mut weight_counter) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::IndicesMigrationDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(MigrationStage::IndicesMigrationOngoing {
								next_key: Some(next_key),
							});
						},
						e => {
							defensive!("Error while migrating indices: {:?}", e);
						},
					}
				},
				MigrationStage::IndicesMigrationDone => {
					Self::transition(MigrationStage::ReferendaMigrationInit);
				},
				MigrationStage::ReferendaMigrationInit => {
					Self::transition(MigrationStage::ReferendaMigrationOngoing {
						last_key: Some(Default::default()),
					});
				},
				MigrationStage::ReferendaMigrationOngoing { last_key } => {
					let res =
						with_transaction_opaque_err::<Option<ReferendaStage>, Error<T>, _>(|| {
							match referenda::ReferendaMigrator::<T>::migrate_many(
								last_key,
								&mut weight_counter,
							) {
								Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
								Err(e) => TransactionOutcome::Rollback(Err(e)),
							}
						})
						.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::ReferendaMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::ReferendaMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						Err(err) => {
							defensive!("Error while migrating referenda: {:?}", err);
						},
					}
				},
				MigrationStage::ReferendaMigrationDone => {
					Self::transition(MigrationStage::BagsListMigrationInit);
				},
				MigrationStage::BagsListMigrationInit => {
					Self::transition(MigrationStage::BagsListMigrationOngoing { next_key: None });
				},
				MigrationStage::BagsListMigrationOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match BagsListMigrator::<T>::migrate_many(next_key, &mut weight_counter) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::BagsListMigrationDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(MigrationStage::BagsListMigrationOngoing {
								next_key: Some(next_key),
							});
						},
						e => {
							defensive!("Error while migrating bags list: {:?}", e);
						},
					}
				},
				MigrationStage::BagsListMigrationDone => {
					Self::transition(MigrationStage::SchedulerMigrationInit);
				},
				MigrationStage::SchedulerMigrationInit => {
					Self::transition(MigrationStage::SchedulerMigrationOngoing { last_key: None });
				},
				MigrationStage::SchedulerMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match scheduler::SchedulerMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::SchedulerAgendaMigrationOngoing { last_key: None });
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::SchedulerMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						Err(err) => {
							defensive!("Error while migrating scheduler: {:?}", err);
						},
					}
				},
				MigrationStage::SchedulerAgendaMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match scheduler::SchedulerAgendaMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::SchedulerMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::SchedulerAgendaMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						Err(err) => {
							defensive!("Error while migrating scheduler: {:?}", err);
						},
					}
				},
				MigrationStage::SchedulerMigrationDone => {
					Self::transition(MigrationStage::ConvictionVotingMigrationInit);
				},
				MigrationStage::ConvictionVotingMigrationInit => {
					Self::transition(MigrationStage::ConvictionVotingMigrationOngoing {
						last_key: None,
					});
				},
				MigrationStage::ConvictionVotingMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match conviction_voting::ConvictionVotingMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::ConvictionVotingMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::ConvictionVotingMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						Err(err) => {
							defensive!("Error while migrating conviction voting: {:?}", err);
						},
					}
				},
				MigrationStage::ConvictionVotingMigrationDone => {
					Self::transition(MigrationStage::BountiesMigrationInit);
				},
				MigrationStage::BountiesMigrationInit => {
					Self::transition(MigrationStage::BountiesMigrationOngoing { last_key: None });
				},
				MigrationStage::BountiesMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match bounties::BountiesMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::BountiesMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::BountiesMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						e => {
							defensive!("Error while migrating bounties: {:?}", e);
						},
					}
				},
				MigrationStage::BountiesMigrationDone => {
					Self::transition(MigrationStage::AssetRateMigrationInit);
				},
				MigrationStage::AssetRateMigrationInit => {
					Self::transition(MigrationStage::AssetRateMigrationOngoing { last_key: None });
				},
				MigrationStage::AssetRateMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match asset_rate::AssetRateMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::AssetRateMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::AssetRateMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						Err(err) => {
							defensive!("Error while migrating asset rates: {:?}", err);
						},
					}
				},
				MigrationStage::AssetRateMigrationDone => {
					Self::transition(MigrationStage::CrowdloanMigrationInit);
				},
				MigrationStage::CrowdloanMigrationInit => {
					Self::transition(MigrationStage::CrowdloanMigrationOngoing { last_key: None });
				},
				MigrationStage::CrowdloanMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match crowdloan::CrowdloanMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
						Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::CrowdloanMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::CrowdloanMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						e => {
							defensive!("Error while migrating crowdloan: {:?}", e);
						},
					}
				},
				MigrationStage::CrowdloanMigrationDone => {
					Self::transition(MigrationStage::TreasuryMigrationInit);
				},
				MigrationStage::TreasuryMigrationInit => {
					Self::transition(MigrationStage::TreasuryMigrationOngoing { last_key: None });
				},
				MigrationStage::TreasuryMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match treasury::TreasuryMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");	

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::TreasuryMigrationDone);
						},
						Ok(Some(last_key)) => {
							Self::transition(MigrationStage::TreasuryMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						e => {
							defensive!("Error while migrating treasury: {:?}", e);
						},
					}
				},
				MigrationStage::TreasuryMigrationDone => {
					#[cfg(feature = "ahm-staking-migration")]
					Self::transition(MigrationStage::StakingMigrationInit);
					#[cfg(not(feature = "ahm-staking-migration"))]
					Self::transition(MigrationStage::StakingMigrationDone);
				},
				#[cfg(feature = "ahm-staking-migration")]
				MigrationStage::StakingMigrationInit => {
					Self::transition(MigrationStage::StakingMigrationOngoing { next_key: None });
				},
				#[cfg(feature = "ahm-staking-migration")]
				MigrationStage::StakingMigrationOngoing { next_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						match staking::StakingMigrator::<T>::migrate_many(
							next_key,
							&mut weight_counter,
						) {
							Ok(next_key) => TransactionOutcome::Commit(Ok(next_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							Self::transition(MigrationStage::StakingMigrationDone);
						},
						Ok(Some(next_key)) => {
							Self::transition(MigrationStage::StakingMigrationOngoing { next_key: Some(next_key) });
						},
						e => {
							defensive!("Error while migrating staking: {:?}", e);
						},
					}
				},
				MigrationStage::StakingMigrationDone => {
					Self::transition(MigrationStage::SignalMigrationFinish);
				},
				MigrationStage::SignalMigrationFinish => {
					#[cfg(feature = "ahm-staking-migration")]
					pallet_staking_async_ah_client::Pallet::<T>::on_migration_end();

					// Send finish message to AH, TODO: weight
					let tracker = RcMigratedBalance::<T>::get();
					let data = MigrationFinishedData {
						rc_balance_kept: tracker.kept,
					};
					let call = types::AhMigratorCall::<T>::FinishMigration { data };
					if let Err(err) = Self::send_xcm(call, T::AhWeightInfo::finish_migration()) {
						defensive!("Failed to send FinishMigration message to AH, \
								retry with the next block: {:?}", err);
					}

					Self::transition(MigrationStage::MigrationDone);
				},
				MigrationStage::MigrationDone => (),
			};

			weight_counter.consumed()
		}
	}

	impl<T: Config> Pallet<T> {
		/// Returns `true` if the migration is ongoing and the Asset Hub has not confirmed
		/// processing the same number of XCM messages as we have sent to it.
		fn has_excess_unconfirmed_dmp(current: &MigrationStageOf<T>) -> bool {
			if !current.is_ongoing() {
				return false;
			}
			let unprocessed_buffer = Self::get_unprocessed_msg_buffer_size();
			let unconfirmed = PendingXcmMessages::<T>::count();
			if unconfirmed > unprocessed_buffer {
				log::info!(
					target: LOG_TARGET,
					"Excess unconfirmed XCM messages: unconfirmed = {}, unprocessed_buffer = {}",
					unconfirmed,
					unprocessed_buffer
				);
				// TODO: make it possible to reset the counts with an extrinsic.
				return true;
			}
			log::debug!(
				target: LOG_TARGET,
				"No excess unconfirmed XCM messages: unconfirmed = {}, unprocessed_buffer = {}",
				unconfirmed,
				unprocessed_buffer
			);
			false
		}

		/// Get the unprocessed message buffer size.
		pub fn get_unprocessed_msg_buffer_size() -> u32 {
			match UnprocessedMsgBuffer::<T>::get() {
				Some(size) => size,
				None => T::UnprocessedMsgBuffer::get(),
			}
		}

		/// Execute a stage transition and log it.
		fn transition(new: MigrationStageOf<T>) {
			let old = RcMigrationStage::<T>::get();

			if new == MigrationStage::Starting {
				defensive_assert!(
					matches!(
						old,
						MigrationStage::WaitingForAh { .. } |
							MigrationStage::Scheduled { .. } |
							MigrationStage::CoolOff { .. }
					),
					"Data migration can only enter from WaitingForAh or Scheduled"
				);
				Self::deposit_event(Event::AssetHubMigrationStarted);
			}

			if new == MigrationStage::MigrationDone {
				defensive_assert!(
					old == MigrationStage::SignalMigrationFinish,
					"MigrationDone can only enter from SignalMigrationFinish"
				);
				Self::deposit_event(Event::AssetHubMigrationFinished);
			}

			RcMigrationStage::<T>::put(&new);
			log::info!(target: LOG_TARGET, "[Block {:?}] RC Stage transition: {:?} -> {:?}", frame_system::Pallet::<T>::block_number(), &old, &new);
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Split up the items into chunks of `MAX_XCM_SIZE` and send them as separate XCM
		/// transacts.
		///
		/// Sent messages are tracked and require confirmation from the Asset Hub before being
		/// removed. If the number of unconfirmed messages exceeds the buffer limit, the migration
		/// is paused.
		///
		/// ### Parameters:
		/// - items - data items to batch and send with the `create_call`
		/// - create_call - function to create the call from the items
		/// - weight_at_most - function to calculate the weight limit on AH for the call with `n`
		///   elements from `items`
		///
		/// Will modify storage in the error path.
		/// This is done to avoid exceeding the XCM message size limit.
		pub fn send_chunked_xcm_and_track<E: Encode>(
			items: impl Into<XcmBatch<E>>,
			create_call: impl Fn(Vec<E>) -> types::AhMigratorCall<T>,
			weight_at_most: impl Fn(u32) -> Weight,
		) -> Result<u32, Error<T>> {
			let mut items = items.into();
			log::info!(target: LOG_TARGET, "Batching {} items to send via XCM", items.len());
			defensive_assert!(items.len() > 0, "Sending XCM with empty items");
			let mut batch_count = 0;

			while let Some(batch) = items.pop_front() {
				let batch_len = batch.len() as u32;
				log::info!(target: LOG_TARGET, "Sending XCM batch of {} items", batch_len);

				let asset_hub_location = Location::new(0, Parachain(1000));

				let receive_notification_call =
					Call::<T>::receive_query_response { query_id: 0, response: Default::default() };

				let query_id = pallet_xcm::Pallet::<T>::new_notify_query(
					asset_hub_location.clone(),
					<T as Config>::RuntimeCall::from(receive_notification_call),
					frame_system::Pallet::<T>::block_number() + T::XcmResponseTimeout::get(),
					Location::here(),
				);

				let call = types::AssetHubPalletConfig::<T>::AhmController(create_call(batch));
				let message = Xcm(vec![
					Instruction::UnpaidExecution {
						weight_limit: WeightLimit::Unlimited,
						check_origin: None,
					},
					Instruction::Transact {
						origin_kind: OriginKind::Superuser,
						fallback_max_weight: None, // TODO @muharem: is this what you meant?
						call: call.encode().into(),
					},
					SetAppendix(Xcm(vec![ReportTransactStatus(QueryResponseInfo {
						destination: Location::parent(),
						query_id,
						max_weight: T::RcWeightInfo::receive_query_response(),
					})])),
				]);

				if let Err(err) = send_xcm::<T::SendXcm>(asset_hub_location, message.clone()) {
					log::error!(target: LOG_TARGET, "Error while sending XCM message: {:?}", err);
					return Err(Error::XcmError);
				} else {
					PendingXcmMessages::<T>::insert(query_id, message);
					batch_count += 1;
				}
			}

			log::info!(target: LOG_TARGET, "Sent {} XCM batch/es", batch_count);
			Ok(batch_count)
		}

		/// Send a single XCM message.
		///
		/// ### Parameters:
		/// - call - the call to send
		/// - weight_at_most - the weight limit for the call on AH
		pub fn send_xcm(
			call: types::AhMigratorCall<T>,
			weight_at_most: Weight,
		) -> Result<(), Error<T>> {
			log::info!(target: LOG_TARGET, "Sending XCM message");

			let call = types::AssetHubPalletConfig::<T>::AhmController(call);

			let message = Xcm(vec![
				Instruction::UnpaidExecution {
					weight_limit: WeightLimit::Unlimited,
					check_origin: None,
				},
				Instruction::Transact {
					origin_kind: OriginKind::Superuser,
					fallback_max_weight: None, // TODO @muharem: is this what you meant?
					call: call.encode().into(),
				},
			]);

			if let Err(err) = send_xcm::<T::SendXcm>(
				Location::new(0, [Junction::Parachain(1000)]),
				message.clone(),
			) {
				log::error!(target: LOG_TARGET, "Error while sending XCM message: {:?}", err);
				return Err(Error::XcmError);
			};

			Ok(())
		}

		pub fn teleport_tracking() -> Option<(T::AccountId, MintLocation)> {
			let stage = RcMigrationStage::<T>::get();
			if stage.is_finished() || stage.is_ongoing() {
				None
			} else {
				Some((T::CheckingAccount::get(), MintLocation::Local))
			}
		}

		/// Force the AH UMP queue priority for the next block.
		pub fn force_ah_ump_queue_priority(now: BlockNumberFor<T>) {
			let (ah_ump_priority_blocks, round_robin_blocks) =
				match AhUmpQueuePriorityConfig::<T>::get() {
					AhUmpQueuePriority::Config => T::AhUmpQueuePriorityPattern::get(),
					AhUmpQueuePriority::OverrideConfig(
						ah_ump_priority_blocks,
						round_robin_blocks,
					) => (ah_ump_priority_blocks, round_robin_blocks),
					AhUmpQueuePriority::Disabled => return,
				};

			let period = ah_ump_priority_blocks + round_robin_blocks;
			if period.is_zero() {
				return;
			}
			let current_block = now % period;

			let is_set = if current_block < ah_ump_priority_blocks {
				// it is safe to force set the queue without checking if the AH UMP queue is empty,
				// as the implementation handles these checks internally.
				let ah_ump = AggregateMessageOrigin::Ump(UmpQueueId::Para(1000.into()));
				match T::MessageQueue::force_set_head(&mut WeightMeter::new(), &ah_ump) {
					Ok(is_set) => is_set,
					Err(_) => {
						defensive!("Failed to force set AH UMP queue priority");
						false
					},
				}
			} else {
				false
			};

			Self::deposit_event(Event::AhUmpQueuePrioritySet {
				prioritized: is_set,
				cycle_block: current_block + BlockNumberFor::<T>::one(),
				cycle_period: period,
			});
		}
	}

	impl<T: Config> types::MigrationStatus for Pallet<T> {
		fn is_ongoing() -> bool {
			RcMigrationStage::<T>::get().is_ongoing()
		}
		fn is_finished() -> bool {
			RcMigrationStage::<T>::get().is_finished()
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
