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

//! The operational pallet for the Asset Hub, designed to manage and facilitate the migration of
//! subsystems such as Governance, Staking, Balances from the Relay Chain to the Asset Hub. This
//! pallet works alongside its counterpart, `pallet_rc_migrator`, which handles migration
//! processes on the Relay Chain side.
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

pub mod account;
pub mod asset_rate;
#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;
pub mod bounties;
pub mod call;
pub mod claims;
pub mod conviction_voting;
pub mod crowdloan;
pub mod indices;
pub mod multisig;
pub mod preimage;
pub mod proxy;
pub mod referenda;
pub mod scheduler;
pub mod staking;
pub mod treasury;
pub mod types;
pub mod vesting;
pub mod xcm_config;

pub use pallet::*;
pub use pallet_rc_migrator::{
	types::{
		ExceptResponseFor, ForceSetHead, LeftOrRight, MaxOnIdleOrInner,
		QueuePriority as DmpQueuePriority, RouteInnerWithException,
	},
	weights_ah,
};
pub use weights_ah::WeightInfo;

use frame_support::{
	pallet_prelude::*,
	storage::{transactional::with_transaction_opaque_err, TransactionOutcome},
	traits::{
		fungible::{Inspect, InspectFreeze, Mutate, MutateFreeze, MutateHold, Unbalanced},
		fungibles::{Inspect as FungiblesInspect, Mutate as FungiblesMutate},
		tokens::{Fortitude, Pay, Preservation},
		Contains, Defensive, DefensiveTruncateFrom, LockableCurrency, OriginTrait, QueryPreimage,
		ReservableCurrency, StorePreimage, VariantCount, WithdrawReasons as LockWithdrawReasons,
	},
	weights::WeightMeter,
};
use frame_system::pallet_prelude::*;
use pallet_balances::{AccountData, Reasons as LockReasons};
use pallet_rc_migrator::{
	bounties::RcBountiesMessageOf, claims::RcClaimsMessageOf, crowdloan::RcCrowdloanMessageOf,
	treasury::RcTreasuryMessage, types::MigrationStatus,
};

use cumulus_primitives_core::AggregateMessageOrigin;
use pallet_rc_migrator::{
	accounts::Account as RcAccount,
	conviction_voting::RcConvictionVotingMessageOf,
	indices::RcIndicesIndexOf,
	multisig::*,
	preimage::*,
	proxy::*,
	staking::{
		bags_list::RcBagsListMessage, delegated_staking::RcDelegatedStakingMessageOf,
		fast_unstake::RcFastUnstakeMessage, nom_pools::*, *,
	},
	types::MigrationFinishedData,
	vesting::RcVestingSchedule,
};
use pallet_referenda::TrackIdOf;
use polkadot_runtime_common::{claims as pallet_claims, impls::VersionedLocatableAsset};
use referenda::RcReferendumInfoOf;
use scheduler::RcSchedulerMessageOf;
use sp_application_crypto::Ss58Codec;
use sp_core::H256;
use sp_runtime::{
	traits::{BlockNumberProvider, Convert, One, TryConvert, Zero},
	AccountId32, FixedU128,
};
use sp_std::prelude::*;
use xcm::prelude::*;
use xcm_builder::MintLocation;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::ah-migrator";

type RcAccountFor<T> = RcAccount<
	<T as frame_system::Config>::AccountId,
	<T as pallet_balances::Config>::Balance,
	<T as Config>::RcHoldReason,
	<T as Config>::RcFreezeReason,
>;
pub type RcTreasuryMessageOf<T> = RcTreasuryMessage<
	<T as frame_system::Config>::AccountId,
	pallet_treasury::BalanceOf<T, ()>,
	pallet_treasury::AssetBalanceOf<T, ()>,
	BlockNumberFor<T>,
	VersionedLocatableAsset,
	VersionedLocation,
	<<T as pallet_treasury::Config>::Paymaster as Pay>::Id,
	<<T as pallet_treasury::Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber,
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
	Indices,
	FastUnstake,
	Crowdloan,
	BagsList,
	Vesting,
	Bounties,
	Treasury,
	Balances,
	Multisig,
	Claims,
	ProxyProxies,
	ProxyAnnouncements,
	PreimageChunk,
	PreimageRequestStatus,
	PreimageLegacyStatus,
	NomPools,
	ReferendaValues,
	ReferendaMetadata,
	ReferendaReferendums,
	Scheduler,
	SchedulerAgenda,
	ConvictionVoting,
	AssetRates,
	Staking,
	DelegatedStaking,
}

/// The migration stage on the Asset Hub.
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
pub enum MigrationStage {
	/// The migration has not started but will start in the future.
	#[default]
	Pending,
	/// Migrating data from the Relay Chain.
	DataMigrationOngoing,
	/// The migration is done.
	MigrationDone,
}

impl MigrationStage {
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
		!matches!(self, MigrationStage::Pending | MigrationStage::MigrationDone)
	}
}

/// Helper struct storing certain balances before the migration.
#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Default,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct BalancesBefore<Balance: Default> {
	pub checking_account: Balance,
	pub total_issuance: Balance,
}

pub type BalanceOf<T> = <T as pallet_balances::Config>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Super config trait for all pallets that the migration depends on, providing convenient
	/// access to their items.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountData = AccountData<u128>, AccountId = AccountId32, Hash = H256>
		+ pallet_balances::Config<Balance = u128>
		+ pallet_multisig::Config
		+ pallet_proxy::Config<BlockNumberProvider = <Self as Config>::RcBlockNumberProvider>
		+ pallet_preimage::Config<Hash = H256>
		+ pallet_referenda::Config<
			BlockNumberProvider = <Self as Config>::RcBlockNumberProvider,
			Votes = u128,
		> + pallet_nomination_pools::Config<
			BlockNumberProvider = <Self as Config>::RcBlockNumberProvider,
		> + pallet_fast_unstake::Config
		+ pallet_bags_list::Config<pallet_bags_list::Instance1>
		+ pallet_scheduler::Config<BlockNumberProvider = <Self as Config>::RcBlockNumberProvider>
		+ pallet_vesting::Config
		+ pallet_indices::Config
		+ pallet_conviction_voting::Config
		+ pallet_asset_rate::Config
		+ pallet_timestamp::Config<Moment = u64>
		+ pallet_ah_ops::Config
		+ pallet_claims::Config
		+ pallet_bounties::Config
		+ pallet_treasury::Config
		+ pallet_delegated_staking::Config
	{
		type RuntimeHoldReason: Parameter + VariantCount;
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The origin that can perform permissioned operations like setting the migration stage.
		///
		/// This is generally root and Fellows origins.
		type ManagerOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Native asset registry type.
		type Currency: Mutate<Self::AccountId, Balance = u128>
			+ MutateHold<Self::AccountId, Reason = <Self as Config>::RuntimeHoldReason>
			+ InspectFreeze<Self::AccountId, Id = Self::FreezeIdentifier>
			+ MutateFreeze<Self::AccountId>
			+ Unbalanced<Self::AccountId>
			+ ReservableCurrency<Self::AccountId, Balance = u128>
			+ LockableCurrency<Self::AccountId, Balance = u128>;
		/// All supported assets registry.
		type Assets: FungiblesMutate<Self::AccountId>;
		/// XCM check account.
		///
		/// Note: the account ID is the same for Polkadot/Kusama Relay and Asset Hub Chains.
		type CheckingAccount: Get<Self::AccountId>;
		/// Relay Chain Hold Reasons.
		///
		/// Additionally requires the `Default` implementation for the benchmarking mocks.
		type RcHoldReason: Parameter + Default + MaxEncodedLen;
		/// Relay Chain Freeze Reasons.
		///
		/// Additionally requires the `Default` implementation for the benchmarking mocks.
		type RcFreezeReason: Parameter + Default + MaxEncodedLen;
		/// Relay Chain to Asset Hub Hold Reasons mapping.
		type RcToAhHoldReason: Convert<Self::RcHoldReason, <Self as Config>::RuntimeHoldReason>;
		/// Relay Chain to Asset Hub Freeze Reasons mapping.
		type RcToAhFreezeReason: Convert<Self::RcFreezeReason, Self::FreezeIdentifier>;
		/// The abridged Relay Chain Proxy Type.
		///
		/// Additionally requires the `Default` implementation for the benchmarking mocks.
		type RcProxyType: Parameter + Default;
		/// Convert a Relay Chain Proxy Type to a local AH one.
		type RcToProxyType: TryConvert<Self::RcProxyType, <Self as pallet_proxy::Config>::ProxyType>;
		/// Convert a Relay Chain block number delay to an Asset Hub one.
		///
		/// Note that we make a simplification here by assuming that both chains have the same block
		/// number type.
		type RcToAhDelay: Convert<BlockNumberFor<Self>, BlockNumberFor<Self>>;
		/// Access the block number of the Relay Chain.
		type RcBlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;
		/// Some part of the Relay Chain origins used in Governance.
		///
		/// Additionally requires the `Default` implementation for the benchmarking mocks.
		type RcPalletsOrigin: Parameter + Default + DecodeWithMemTracking;
		/// Convert a Relay Chain origin to an Asset Hub one.
		type RcToAhPalletsOrigin: TryConvert<
			Self::RcPalletsOrigin,
			<<Self as frame_system::Config>::RuntimeOrigin as OriginTrait>::PalletsOrigin,
		>;
		/// Preimage registry.
		type Preimage: QueryPreimage<H = <Self as frame_system::Config>::Hashing> + StorePreimage;
		/// Convert a Relay Chain Call to a local AH one.
		type RcToAhCall: for<'a> TryConvert<&'a [u8], <Self as frame_system::Config>::RuntimeCall>;
		/// Send UMP message.
		type SendXcm: SendXcm;
		/// Weight information for extrinsics in this pallet.
		type AhWeightInfo: WeightInfo;
		/// Asset Hub Treasury accounts migrating to the new treasury account address (same account
		/// address that was used on the Relay Chain).
		///
		/// The provided asset ids should be manageable by the [`Self::Assets`] registry. The asset
		/// list should not include the native asset.
		type TreasuryAccounts: Get<(
			Self::AccountId,
			Vec<<Self::Assets as FungiblesInspect<Self::AccountId>>::AssetId>,
		)>;
		/// Convert the Relay Chain Treasury Spend (AssetKind, Beneficiary) parameters to the
		/// Asset Hub (AssetKind, Beneficiary) parameters.
		type RcToAhTreasurySpend: Convert<
			(VersionedLocatableAsset, VersionedLocation),
			Result<
				(
					<Self as pallet_treasury::Config>::AssetKind,
					<Self as pallet_treasury::Config>::Beneficiary,
				),
				(),
			>,
		>;

		/// Calls that are allowed during the migration.
		type AhIntraMigrationCalls: Contains<<Self as frame_system::Config>::RuntimeCall>;

		/// Calls that are allowed after the migration finished.
		type AhPostMigrationCalls: Contains<<Self as frame_system::Config>::RuntimeCall>;

		/// Means to force a next queue within the message queue processing DMP and HRMP queues.
		type MessageQueue: ForceSetHead<AggregateMessageOrigin>;

		/// The priority pattern for DMP queue processing during migration [Config::MessageQueue].
		///
		/// This configures how frequently the DMP queue gets priority over other queues
		/// (like HRMP). The tuple (dmp_priority_blocks, round_robin_blocks) defines a repeating
		/// cycle where:
		/// - `dmp_priority_blocks` consecutive blocks: DMP queue gets priority
		/// - `round_robin_blocks` consecutive blocks: round-robin processing of all queues
		/// - Then the cycle repeats
		///
		/// For example, (18, 2) means a cycle of 20 blocks that repeats.
		///
		/// This configuration can be overridden by a storage item [`DmpQueuePriorityConfig`].
		type DmpQueuePriorityPattern: Get<(BlockNumberFor<Self>, BlockNumberFor<Self>)>;
	}

	/// RC accounts that failed to migrate when were received on the Asset Hub.
	///
	/// This is unlikely to happen, since we dry run the migration, but we keep it for completeness.
	#[pallet::storage]
	pub type RcAccounts<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, RcAccountFor<T>, OptionQuery>;

	/// The Asset Hub migration state.
	#[pallet::storage]
	pub type AhMigrationStage<T: Config> = StorageValue<_, MigrationStage, ValueQuery>;

	/// Helper storage item to store the total balance / total issuance of native token at the start
	/// of the migration. Since teleports are disabled during migration, the total issuance will not
	/// change for other reason than the migration itself.
	#[pallet::storage]
	pub type AhBalancesBefore<T: Config> = StorageValue<_, BalancesBefore<T::Balance>, ValueQuery>;

	/// The priority of the DMP queue during migration.
	///
	/// Controls how the DMP (Downward Message Passing) queue is processed relative to other queues
	/// during the migration process. This helps ensure timely processing of migration messages.
	/// The default priority pattern is defined in the pallet configuration, but can be overridden
	/// by a storage value of this type.
	#[pallet::storage]
	pub type DmpQueuePriorityConfig<T: Config> =
		StorageValue<_, DmpQueuePriority<BlockNumberFor<T>>, ValueQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// Failed to unreserve deposit.
		FailedToUnreserveDeposit,
		/// Failed to process an account data from RC.
		FailedToProcessAccount,
		/// Some item could not be inserted because it already exists.
		InsertConflict,
		/// Failed to convert RC type to AH type.
		FailedToConvertType,
		/// Failed to fetch preimage.
		PreimageNotFound,
		/// Failed to convert RC call to AH call.
		FailedToConvertCall,
		/// Failed to bound a call.
		FailedToBoundCall,
		/// Failed to send XCM message.
		XcmError,
		/// Failed to integrate a vesting schedule.
		FailedToIntegrateVestingSchedule,
		/// Checking account overflow or underflow.
		FailedToCalculateCheckingAccount,
		/// Vector did not fit into its compile-time bound.
		FailedToBoundVector,
		/// The DMP queue priority is already set to the same value.
		DmpQueuePriorityAlreadySet,
		/// Invalid parameter.
		InvalidParameter,
		/// Preimage missing.
		PreimageMissing,
		/// Preimage too big.
		PreimageTooBig,
		/// Preimage chunk missing.
		PreimageChunkMissing,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A stage transition has occurred.
		StageTransition {
			/// The old stage before the transition.
			old: MigrationStage,
			/// The new stage after the transition.
			new: MigrationStage,
		},
		/// We received a batch of messages that will be integrated into a pallet.
		BatchReceived { pallet: PalletEventName, count: u32 },
		/// We processed a batch of messages for this pallet.
		BatchProcessed { pallet: PalletEventName, count_good: u32, count_bad: u32 },
		/// The Asset Hub Migration started and is active until `AssetHubMigrationFinished` is
		/// emitted.
		///
		/// This event is equivalent to `StageTransition { new: DataMigrationOngoing, .. }` but is
		/// easier to understand. The activation is immediate and affects all events happening
		/// afterwards.
		AssetHubMigrationStarted,
		/// The Asset Hub Migration finished.
		///
		/// This event is equivalent to `StageTransition { new: MigrationDone, .. }` but is easier
		/// to understand. The finishing is immediate and affects all events happening
		/// afterwards.
		AssetHubMigrationFinished,
		/// Whether the DMP queue was prioritized for the next block.
		DmpQueuePrioritySet {
			/// Indicates if DMP queue was successfully set as priority.
			/// If `false`, it means we're in the round-robin phase of our priority pattern
			/// (see [`Config::DmpQueuePriorityPattern`]), where no queue gets priority.
			prioritized: bool,
			/// Current block number within the pattern cycle (1 to period).
			cycle_block: BlockNumberFor<T>,
			/// Total number of blocks in the pattern cycle
			cycle_period: BlockNumberFor<T>,
		},
		/// The DMP queue priority config was set.
		DmpQueuePriorityConfigSet {
			/// The old priority pattern.
			old: DmpQueuePriority<BlockNumberFor<T>>,
			/// The new priority pattern.
			new: DmpQueuePriority<BlockNumberFor<T>>,
		},
		/// The balances before the migration were recorded.
		BalancesBeforeRecordSet { checking_account: T::Balance, total_issuance: T::Balance },
		/// The balances before the migration were consumed.
		BalancesBeforeRecordConsumed { checking_account: T::Balance, total_issuance: T::Balance },
		/// A referendum was cancelled because it could not be mapped.
		ReferendumCanceled { id: u32 },
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Receive accounts from the Relay Chain.
		///
		/// The accounts sent with `pallet_rc_migrator::Pallet::migrate_accounts` function.
		#[pallet::call_index(0)]
		#[pallet::weight({
			let mut total = Weight::zero();
			let weight_of = |account: &RcAccountFor<T>| if account.is_liquid() {
				T::AhWeightInfo::receive_liquid_accounts
			} else {
				T::AhWeightInfo::receive_accounts
			};
			for account in accounts.iter() {
				let weight = if total.is_zero() {
					weight_of(account)(1)
				} else {
					weight_of(account)(1).saturating_sub(weight_of(account)(0))
				};
				total = total.saturating_add(weight);
			}
			total
		})]
		pub fn receive_accounts(
			origin: OriginFor<T>,
			accounts: Vec<RcAccountFor<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_accounts(accounts).map_err(Into::into)
		}

		/// Receive multisigs from the Relay Chain.
		///
		/// This will be called from an XCM `Transact` inside a DMP from the relay chain. The
		/// multisigs were prepared by
		/// `pallet_rc_migrator::multisig::MultisigMigrator::migrate_many`.
		#[pallet::call_index(1)]
		#[pallet::weight(T::AhWeightInfo::receive_multisigs(accounts.len() as u32))]
		pub fn receive_multisigs(
			origin: OriginFor<T>,
			accounts: Vec<RcMultisigOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_multisigs(accounts).map_err(Into::into)
		}

		/// Receive proxies from the Relay Chain.
		#[pallet::call_index(2)]
		#[pallet::weight(T::AhWeightInfo::receive_proxy_proxies(proxies.len() as u32))]
		pub fn receive_proxy_proxies(
			origin: OriginFor<T>,
			proxies: Vec<RcProxyOf<T, T::RcProxyType>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_proxies(proxies).map_err(Into::into)
		}

		/// Receive proxy announcements from the Relay Chain.
		#[pallet::call_index(3)]
		#[pallet::weight(T::AhWeightInfo::receive_proxy_announcements(announcements.len() as u32))]
		pub fn receive_proxy_announcements(
			origin: OriginFor<T>,
			announcements: Vec<RcProxyAnnouncementOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_proxy_announcements(announcements).map_err(Into::into)
		}

		#[pallet::call_index(4)]
		#[pallet::weight({
			let mut total = Weight::zero();
			for chunk in chunks.iter() {
				total = total.saturating_add(
					T::AhWeightInfo::receive_preimage_chunk(
						chunk.chunk_byte_offset / chunks::CHUNK_SIZE,
					),
				);
			}
			total
		})]
		pub fn receive_preimage_chunks(
			origin: OriginFor<T>,
			chunks: Vec<RcPreimageChunk>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_preimage_chunks(chunks).map_err(Into::into)
		}

		#[pallet::call_index(5)]
		#[pallet::weight(T::AhWeightInfo::receive_preimage_request_status(request_status.len() as u32))]
		pub fn receive_preimage_request_status(
			origin: OriginFor<T>,
			request_status: Vec<RcPreimageRequestStatusOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_preimage_request_statuses(request_status).map_err(Into::into)
		}

		#[pallet::call_index(6)]
		#[pallet::weight(T::AhWeightInfo::receive_preimage_legacy_status(legacy_status.len() as u32))]
		pub fn receive_preimage_legacy_status(
			origin: OriginFor<T>,
			legacy_status: Vec<RcPreimageLegacyStatusOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_preimage_legacy_statuses(legacy_status).map_err(Into::into)
		}

		#[pallet::call_index(7)]
		#[pallet::weight(T::AhWeightInfo::receive_nom_pools_messages(messages.len() as u32))]
		pub fn receive_nom_pools_messages(
			origin: OriginFor<T>,
			messages: Vec<RcNomPoolsMessage<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_nom_pools_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(8)]
		#[pallet::weight(T::AhWeightInfo::receive_vesting_schedules(schedules.len() as u32))]
		pub fn receive_vesting_schedules(
			origin: OriginFor<T>,
			schedules: Vec<RcVestingSchedule<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_vesting_schedules(schedules).map_err(Into::into)
		}

		#[pallet::call_index(9)]
		#[pallet::weight(T::AhWeightInfo::receive_fast_unstake_messages(messages.len() as u32))]
		pub fn receive_fast_unstake_messages(
			origin: OriginFor<T>,
			messages: Vec<RcFastUnstakeMessage<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_fast_unstake_messages(messages).map_err(Into::into)
		}

		/// Receive referendum counts, deciding counts, votes for the track queue.
		#[pallet::call_index(10)]
		#[pallet::weight(T::AhWeightInfo::receive_referenda_values())]
		pub fn receive_referenda_values(
			origin: OriginFor<T>,
			// we accept a vector here only to satisfy the signature of the
			// `pallet_rc_migrator::Pallet::send_chunked_xcm_and_track` function and avoid
			// introducing a send function for non-vector data or rewriting the referenda pallet
			// migration.
			mut values: Vec<(
				// referendum_count
				Option<u32>,
				// deciding_count (track_id, count)
				Vec<(TrackIdOf<T, ()>, u32)>,
				// track_queue (referendum_id, votes)
				Vec<(TrackIdOf<T, ()>, Vec<(u32, u128)>)>,
			)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			ensure!(values.len() == 1, Error::<T>::InvalidParameter);

			let (referendum_count, deciding_count, track_queue) =
				values.pop().ok_or(Error::<T>::InvalidParameter)?;

			Self::do_receive_referenda_values(referendum_count, deciding_count, track_queue)
				.map_err(Into::into)
		}

		/// Receive referendums from the Relay Chain.
		#[pallet::call_index(11)]
		#[pallet::weight({
			let mut total = Weight::zero();
			for (_, info) in referendums.iter() {
				let weight = match info {
					pallet_referenda::ReferendumInfo::Ongoing(status) => {
						let len = status.proposal.len().defensive_unwrap_or(
							// should not happen, but we pick some sane call length.
							512,
						);
						T::AhWeightInfo::receive_single_active_referendums(len)
					},
					_ =>
						if total.is_zero() {
							T::AhWeightInfo::receive_complete_referendums(1)
						} else {
							T::AhWeightInfo::receive_complete_referendums(1)
								.saturating_sub(T::AhWeightInfo::receive_complete_referendums(0))
						},
				};
				total = total.saturating_add(weight);
			}
			total
		})]
		pub fn receive_referendums(
			origin: OriginFor<T>,
			referendums: Vec<(u32, RcReferendumInfoOf<T, ()>)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_referendums(referendums).map_err(Into::into)
		}
		#[pallet::call_index(12)]
		#[pallet::weight(T::AhWeightInfo::receive_claims(messages.len() as u32))]
		pub fn receive_claims(
			origin: OriginFor<T>,
			messages: Vec<RcClaimsMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_claims(messages).map_err(Into::into)
		}

		#[pallet::call_index(13)]
		#[pallet::weight(T::AhWeightInfo::receive_bags_list_messages(messages.len() as u32))]
		pub fn receive_bags_list_messages(
			origin: OriginFor<T>,
			messages: Vec<RcBagsListMessage<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_bags_list_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(14)]
		#[pallet::weight(T::AhWeightInfo::receive_scheduler_lookup(messages.len() as u32))]
		pub fn receive_scheduler_messages(
			origin: OriginFor<T>,
			messages: Vec<RcSchedulerMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_scheduler_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(15)]
		#[pallet::weight(T::AhWeightInfo::receive_indices(indices.len() as u32))]
		pub fn receive_indices(
			origin: OriginFor<T>,
			indices: Vec<RcIndicesIndexOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_indices(indices).map_err(Into::into)
		}

		#[pallet::call_index(16)]
		#[pallet::weight(T::AhWeightInfo::receive_conviction_voting_messages(messages.len() as u32))]
		pub fn receive_conviction_voting_messages(
			origin: OriginFor<T>,
			messages: Vec<RcConvictionVotingMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_conviction_voting_messages(messages).map_err(Into::into)
		}
		#[pallet::call_index(17)]
		#[pallet::weight(T::AhWeightInfo::receive_bounties_messages(messages.len() as u32))]
		pub fn receive_bounties_messages(
			origin: OriginFor<T>,
			messages: Vec<RcBountiesMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_bounties_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(18)]
		#[pallet::weight(T::AhWeightInfo::receive_asset_rates(rates.len() as u32))]
		pub fn receive_asset_rates(
			origin: OriginFor<T>,
			rates: Vec<(<T as pallet_asset_rate::Config>::AssetKind, FixedU128)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_asset_rates(rates).map_err(Into::into)
		}
		#[pallet::call_index(19)]
		#[pallet::weight(T::AhWeightInfo::receive_crowdloan_messages(messages.len() as u32))]
		pub fn receive_crowdloan_messages(
			origin: OriginFor<T>,
			messages: Vec<RcCrowdloanMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_crowdloan_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(20)]
		#[pallet::weight(T::AhWeightInfo::receive_referenda_metadata(metadata.len() as u32))]
		pub fn receive_referenda_metadata(
			origin: OriginFor<T>,
			metadata: Vec<(u32, <T as frame_system::Config>::Hash)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_referenda_metadata(metadata).map_err(Into::into)
		}
		#[pallet::call_index(21)]
		#[pallet::weight(T::AhWeightInfo::receive_treasury_messages(messages.len() as u32))]
		pub fn receive_treasury_messages(
			origin: OriginFor<T>,
			messages: Vec<RcTreasuryMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_treasury_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(22)]
		#[pallet::weight({
			let mut total = Weight::zero();
			for (_, agenda) in messages.iter() {
				for maybe_task in agenda {
					let Some(task) = maybe_task else {
						continue;
					};
					let preimage_len = task.call.len().defensive_unwrap_or(
						// should not happen, but we assume some sane call length.
						512,
					);
					total = total.saturating_add(
						T::AhWeightInfo::receive_single_scheduler_agenda(preimage_len),
					);
				}
			}
			total
		})]
		pub fn receive_scheduler_agenda_messages(
			origin: OriginFor<T>,
			messages: Vec<(BlockNumberFor<T>, Vec<Option<scheduler::RcScheduledOf<T>>>)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_scheduler_agenda_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(23)]
		#[pallet::weight(T::AhWeightInfo::receive_delegated_staking_messages(messages.len() as u32))]
		pub fn receive_delegated_staking_messages(
			origin: OriginFor<T>,
			messages: Vec<RcDelegatedStakingMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_delegated_staking_messages(messages).map_err(Into::into)
		}

		#[cfg(feature = "ahm-staking-migration")]
		#[pallet::call_index(30)]
		#[pallet::weight({1})] // TODO: weight
		pub fn receive_staking_messages(
			origin: OriginFor<T>,
			messages: Vec<T::RcStakingMessage>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_staking_messages(messages).map_err(Into::into)
		}

		/// Set the migration stage.
		///
		/// This call is intended for emergency use only and is guarded by the
		/// [`Config::ManagerOrigin`].
		#[pallet::call_index(100)]
		#[pallet::weight(T::AhWeightInfo::force_set_stage())]
		pub fn force_set_stage(origin: OriginFor<T>, stage: MigrationStage) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;
			Self::transition(stage);
			Ok(())
		}

		/// Start the data migration.
		///
		/// This is typically called by the Relay Chain to start the migration on the Asset Hub and
		/// receive a handshake message indicating the Asset Hub's readiness.
		#[pallet::call_index(101)]
		#[pallet::weight(T::AhWeightInfo::start_migration())]
		pub fn start_migration(origin: OriginFor<T>) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;

			Self::migration_start_hook().map_err(Into::into)
		}

		/// Set the DMP queue priority configuration.
		///
		/// Can only be called by the `ManagerOrigin`.
		#[pallet::call_index(102)]
		#[pallet::weight(T::AhWeightInfo::set_dmp_queue_priority())]
		pub fn set_dmp_queue_priority(
			origin: OriginFor<T>,
			new: DmpQueuePriority<BlockNumberFor<T>>,
		) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;
			let old = DmpQueuePriorityConfig::<T>::get();
			if old == new {
				return Err(Error::<T>::DmpQueuePriorityAlreadySet.into());
			}
			ensure!(
				new.get_priority_blocks().map_or(true, |blocks| !blocks.is_zero()),
				Error::<T>::InvalidParameter
			);
			DmpQueuePriorityConfig::<T>::put(new.clone());
			Self::deposit_event(Event::DmpQueuePriorityConfigSet { old, new });
			Ok(())
		}

		/// Finish the migration.
		///
		/// This is typically called by the Relay Chain to signal the migration has finished.
		///
		/// The `data` parameter might be `None` if we are running the migration for a second time
		/// for some pallets and have already performed the checking account balance correction,
		/// so we do not need to do it this time.
		#[pallet::call_index(110)]
		#[pallet::weight(T::AhWeightInfo::finish_migration())]
		pub fn finish_migration(
			origin: OriginFor<T>,
			data: Option<MigrationFinishedData<T::Balance>>,
		) -> DispatchResult {
			<T as Config>::ManagerOrigin::ensure_origin(origin)?;

			Self::migration_finish_hook(data).map_err(Into::into)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let mut weight = Weight::from_parts(0, 0);

			if Self::is_ongoing() {
				weight = weight.saturating_add(T::AhWeightInfo::force_dmp_queue_priority());
			}

			weight.saturating_add(T::AhWeightInfo::on_finalize())
		}

		fn on_finalize(now: BlockNumberFor<T>) {
			if Self::is_ongoing() {
				Self::force_dmp_queue_priority(now);
			}
		}

		fn integrity_test() {
			let (dmp_priority_blocks, _) = T::DmpQueuePriorityPattern::get();
			assert!(!dmp_priority_blocks.is_zero(), "the `dmp_priority_blocks` should be non-zero");
		}
	}

	impl<T: Config> Pallet<T> {
		/// Auxiliary logic to be done before the migration starts.
		pub fn migration_start_hook() -> Result<(), Error<T>> {
			Self::send_xcm(types::RcMigratorCall::StartDataMigration)?;

			// Accounts
			let checking_account = T::CheckingAccount::get();
			let balances_before = BalancesBefore {
				checking_account: <T as Config>::Currency::total_balance(&checking_account),
				total_issuance: <T as Config>::Currency::total_issuance(),
			};
			log::info!(
				target: LOG_TARGET,
				"start_migration(): checking_account_balance {:?}, total_issuance {:?}",
				balances_before.checking_account, balances_before.total_issuance
			);
			AhBalancesBefore::<T>::put(&balances_before);

			Self::deposit_event(Event::BalancesBeforeRecordSet {
				checking_account: balances_before.checking_account,
				total_issuance: balances_before.total_issuance,
			});

			Self::transition(MigrationStage::DataMigrationOngoing);
			Ok(())
		}

		/// Auxiliary logic to be done after the migration finishes.
		pub fn migration_finish_hook(
			data: Option<MigrationFinishedData<T::Balance>>,
		) -> Result<(), Error<T>> {
			// Accounts
			if let Some(data) = data {
				if let Err(err) = Self::finish_accounts_migration(data.rc_balance_kept) {
					defensive!("Account migration failed: {:?}", err);
				}
			}

			// We have to go into the Done state, otherwise the chain will be blocked
			Self::transition(MigrationStage::MigrationDone);
			Ok(())
		}

		/// Execute a stage transition and log it.
		fn transition(new: MigrationStage) {
			let old = AhMigrationStage::<T>::get();

			if new == MigrationStage::DataMigrationOngoing {
				defensive_assert!(
					old == MigrationStage::Pending,
					"Data migration can only enter from Pending"
				);
				Self::deposit_event(Event::AssetHubMigrationStarted);
			}
			if new == MigrationStage::MigrationDone {
				defensive_assert!(
					old == MigrationStage::DataMigrationOngoing,
					"MigrationDone can only enter from DataMigrationOngoing"
				);
				Self::deposit_event(Event::AssetHubMigrationFinished);
			}

			AhMigrationStage::<T>::put(&new);
			log::info!(
				target: LOG_TARGET,
				"[Block {:?}] AH stage transition: {:?} -> {:?}",
				frame_system::Pallet::<T>::block_number(),
				&old,
				&new
			);
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Send a single XCM message.
		pub fn send_xcm(call: types::RcMigratorCall) -> Result<(), Error<T>> {
			log::debug!(target: LOG_TARGET, "Sending XCM message");

			let call = types::RcPalletConfig::RcmController(call);

			let message = Xcm(vec![
				Instruction::UnpaidExecution {
					weight_limit: WeightLimit::Unlimited,
					check_origin: None,
				},
				Instruction::Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: call.encode().into(),
				},
			]);

			if let Err(err) = send_xcm::<T::SendXcm>(Location::parent(), message.clone()) {
				log::error!(target: LOG_TARGET, "Error while sending XCM message: {:?}", err);
				return Err(Error::XcmError);
			};

			Ok(())
		}

		pub fn teleport_tracking() -> Option<(T::AccountId, MintLocation)> {
			let stage = AhMigrationStage::<T>::get();
			if stage.is_finished() {
				Some((T::CheckingAccount::get(), MintLocation::Local))
			} else {
				None
			}
		}

		/// Force the DMP queue priority for the next block.
		pub fn force_dmp_queue_priority(now: BlockNumberFor<T>) {
			let (dmp_priority_blocks, round_robin_blocks) = match DmpQueuePriorityConfig::<T>::get()
			{
				DmpQueuePriority::Config => T::DmpQueuePriorityPattern::get(),
				DmpQueuePriority::OverrideConfig(dmp_priority_blocks, round_robin_blocks) =>
					(dmp_priority_blocks, round_robin_blocks),
				DmpQueuePriority::Disabled => return,
			};

			let period = dmp_priority_blocks + round_robin_blocks;
			if period.is_zero() {
				return;
			}
			let current_block = now % period;

			let is_set = if current_block < dmp_priority_blocks {
				// it is safe to force set the queue without checking if the DMP queue is empty, as
				// the implementation handles these checks internally.
				let dmp = AggregateMessageOrigin::Parent;
				match T::MessageQueue::force_set_head(&mut WeightMeter::new(), &dmp) {
					Ok(is_set) => is_set,
					Err(_) => {
						defensive!("Failed to force set DMP queue priority");
						false
					},
				}
			} else {
				false
			};

			Self::deposit_event(Event::DmpQueuePrioritySet {
				prioritized: is_set,
				cycle_block: current_block + BlockNumberFor::<T>::one(),
				cycle_period: period,
			});
		}
	}

	impl<T: Config> MigrationStatus for Pallet<T> {
		fn is_ongoing() -> bool {
			AhMigrationStage::<T>::get().is_ongoing()
		}
		fn is_finished() -> bool {
			AhMigrationStage::<T>::get().is_finished()
		}
	}
}

impl<T: Config> Contains<<T as frame_system::Config>::RuntimeCall> for Pallet<T> {
	fn contains(call: &<T as frame_system::Config>::RuntimeCall) -> bool {
		let stage = AhMigrationStage::<T>::get();

		// We have to return whether the call is allowed:
		const ALLOWED: bool = true;
		const FORBIDDEN: bool = false;

		// Once the migration is finished, forbid calls not in the `RcPostMigrationCalls` set.
		if stage.is_finished() && !T::AhPostMigrationCalls::contains(call) {
			return FORBIDDEN;
		}

		// If the migration is ongoing, forbid calls not in the `RcIntraMigrationCalls` set.
		if stage.is_ongoing() && !T::AhIntraMigrationCalls::contains(call) {
			return FORBIDDEN;
		}

		// Otherwise, allow the call.
		// This also implicitly allows _any_ call if the migration has not yet started.
		ALLOWED
	}
}
