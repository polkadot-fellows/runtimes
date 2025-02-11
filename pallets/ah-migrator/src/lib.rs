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
pub mod bounties;
pub mod call;
pub mod claims;
pub mod conviction_voting;
pub mod indices;
pub mod multisig;
pub mod preimage;
pub mod proxy;
pub mod referenda;
pub mod scheduler;
pub mod staking;
pub mod types;
pub mod vesting;

pub use pallet::*;
pub use pallet_rc_migrator::types::ZeroWeightOr;

use frame_support::{
	pallet_prelude::*,
	storage::{transactional::with_transaction_opaque_err, TransactionOutcome},
	traits::{
		fungible::{InspectFreeze, Mutate, MutateFreeze, MutateHold},
		Defensive, DefensiveTruncateFrom, LockableCurrency, OriginTrait, QueryPreimage,
		ReservableCurrency, StorePreimage, WithdrawReasons as LockWithdrawReasons,
	},
};
use frame_system::pallet_prelude::*;
use pallet_balances::{AccountData, Reasons as LockReasons};
use pallet_rc_migrator::{
	accounts::Account as RcAccount,
	claims::RcClaimsMessageOf,
	conviction_voting::RcConvictionVotingMessageOf,
	indices::RcIndicesIndexOf,
	multisig::*,
	preimage::*,
	proxy::*,
	staking::{
		bags_list::RcBagsListMessage,
		fast_unstake::{FastUnstakeMigrator, RcFastUnstakeMessage},
		nom_pools::*,
	},
	vesting::RcVestingSchedule,
};
use pallet_referenda::TrackIdOf;
use polkadot_runtime_common::claims as pallet_claims;
use referenda::RcReferendumInfoOf;
use sp_application_crypto::Ss58Codec;
use sp_core::H256;
use sp_runtime::{
	traits::{BlockNumberProvider, Convert, TryConvert},
	AccountId32, FixedU128,
};
use sp_std::prelude::*;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::ah-migrator";

type RcAccountFor<T> = RcAccount<
	<T as frame_system::Config>::AccountId,
	<T as pallet_balances::Config>::Balance,
	<T as Config>::RcHoldReason,
	<T as Config>::RcFreezeReason,
>;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum PalletEventName {
	Indices,
	FastUnstake,
	BagsList,
	Vesting,
	Bounties,
}

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	/// Super config trait for all pallets that the migration depends on, providing convenient
	/// access to their items.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountData = AccountData<u128>, AccountId = AccountId32>
		+ pallet_balances::Config<Balance = u128>
		+ pallet_multisig::Config
		+ pallet_claims::Config
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
		+ pallet_bounties::Config
		+ pallet_treasury::Config
		+ pallet_asset_rate::Config
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Native asset registry type.
		type Currency: Mutate<Self::AccountId, Balance = u128>
			+ MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>
			+ InspectFreeze<Self::AccountId, Id = Self::FreezeIdentifier>
			+ MutateFreeze<Self::AccountId>
			+ ReservableCurrency<Self::AccountId, Balance = u128>
			+ LockableCurrency<Self::AccountId, Balance = u128>;
		/// XCM check account.
		type CheckingAccount: Get<Self::AccountId>;
		/// Relay Chain Hold Reasons.
		type RcHoldReason: Parameter;
		/// Relay Chain Freeze Reasons.
		type RcFreezeReason: Parameter;
		/// Relay Chain to Asset Hub Hold Reasons mapping;
		type RcToAhHoldReason: Convert<Self::RcHoldReason, Self::RuntimeHoldReason>;
		/// Relay Chain to Asset Hub Freeze Reasons mapping;
		type RcToAhFreezeReason: Convert<Self::RcFreezeReason, Self::FreezeIdentifier>;
		/// The abridged Relay Chain Proxy Type.
		type RcProxyType: Parameter;
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
		type RcPalletsOrigin: Parameter;
		/// Convert a Relay Chain origin to an Asset Hub one.
		type RcToAhPalletsOrigin: TryConvert<
			Self::RcPalletsOrigin,
			<<Self as frame_system::Config>::RuntimeOrigin as OriginTrait>::PalletsOrigin,
		>;
		/// Preimage registry.
		type Preimage: QueryPreimage<H = <Self as frame_system::Config>::Hashing> + StorePreimage;
		/// Convert a Relay Chain Call to a local AH one.
		type RcToAhCall: for<'a> TryConvert<&'a [u8], <Self as frame_system::Config>::RuntimeCall>;
	}

	/// RC accounts that failed to migrate when were received on the Asset Hub.
	///
	/// This is unlikely to happen, since we dry run the migration, but we keep it for completeness.
	#[pallet::storage]
	pub type RcAccounts<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, RcAccountFor<T>, OptionQuery>;

	#[pallet::error]
	pub enum Error<T> {
		/// The error that should to be replaced by something meaningful.
		TODO,
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
		/// Failed to integrate a vesting schedule.
		FailedToIntegrateVestingSchedule,
		Unreachable,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The event that should to be replaced by something meaningful.
		TODO,

		/// We received a batch of accounts that we are going to integrate.
		AccountBatchReceived {
			/// How many accounts are in the batch.
			count: u32,
		},
		/// We processed a batch of accounts that we received.
		AccountBatchProcessed {
			/// How many accounts were successfully integrated.
			count_good: u32,
			/// How many accounts failed to integrate.
			count_bad: u32,
		},
		/// We received a batch of multisigs that we are going to integrate.
		MultisigBatchReceived {
			/// How many multisigs are in the batch.
			count: u32,
		},
		MultisigBatchProcessed {
			/// How many multisigs were successfully integrated.
			count_good: u32,
			/// How many multisigs failed to integrate.
			count_bad: u32,
		},
		/// We received a batch of claims that we are going to integrate.
		ClaimsBatchReceived {
			/// How many claims are in the batch.
			count: u32,
		},
		/// We processed a batch of claims that we received.
		ClaimsBatchProcessed {
			/// How many claims were successfully integrated.
			count_good: u32,
			/// How many claims failed to integrate.
			count_bad: u32,
		},
		/// We received a batch of proxies that we are going to integrate.
		ProxyProxiesBatchReceived {
			/// How many proxies are in the batch.
			count: u32,
		},
		/// We processed a batch of proxies that we received.
		ProxyProxiesBatchProcessed {
			/// How many proxies were successfully integrated.
			count_good: u32,
			/// How many proxies failed to integrate.
			count_bad: u32,
		},
		/// We received a batch of proxy announcements that we are going to integrate.
		ProxyAnnouncementsBatchReceived {
			/// How many proxy announcements are in the batch.
			count: u32,
		},
		/// We processed a batch of proxy announcements that we received.
		ProxyAnnouncementsBatchProcessed {
			/// How many proxy announcements were successfully integrated.
			count_good: u32,
			/// How many proxy announcements failed to integrate.
			count_bad: u32,
		},
		/// Received a batch of `RcPreimageChunk` that are going to be integrated.
		PreimageChunkBatchReceived {
			/// How many preimage chunks are in the batch.
			count: u32,
		},
		/// We processed a batch of `RcPreimageChunk` that we received.
		PreimageChunkBatchProcessed {
			/// How many preimage chunks were successfully integrated.
			count_good: u32,
			/// How many preimage chunks failed to integrate.
			count_bad: u32,
		},
		/// We received a batch of `RcPreimageRequestStatus` that we are going to integrate.
		PreimageRequestStatusBatchReceived {
			/// How many preimage request status are in the batch.
			count: u32,
		},
		/// We processed a batch of `RcPreimageRequestStatus` that we received.
		PreimageRequestStatusBatchProcessed {
			/// How many preimage request status were successfully integrated.
			count_good: u32,
			/// How many preimage request status failed to integrate.
			count_bad: u32,
		},
		/// We received a batch of `RcPreimageLegacyStatus` that we are going to integrate.
		PreimageLegacyStatusBatchReceived {
			/// How many preimage legacy status are in the batch.
			count: u32,
		},
		/// We processed a batch of `RcPreimageLegacyStatus` that we received.
		PreimageLegacyStatusBatchProcessed {
			/// How many preimage legacy status were successfully integrated.
			count_good: u32,
			/// How many preimage legacy status failed to integrate.
			count_bad: u32,
		},
		/// Received a batch of `RcNomPoolsMessage` that we are going to integrate.
		NomPoolsMessagesBatchReceived {
			/// How many nom pools messages are in the batch.
			count: u32,
		},
		/// Processed a batch of `RcNomPoolsMessage` that we received.
		NomPoolsMessagesBatchProcessed {
			/// How many nom pools messages were successfully integrated.
			count_good: u32,
			/// How many nom pools messages failed to integrate.
			count_bad: u32,
		},
		/// We received a batch of messages that will be integrated into a pallet.
		BatchReceived {
			pallet: PalletEventName,
			count: u32,
		},
		/// We processed a batch of messages for this pallet.
		BatchProcessed {
			pallet: PalletEventName,
			count_good: u32,
			count_bad: u32,
		},
		/// We received a batch of referendums that we are going to integrate.
		ReferendumsBatchReceived {
			/// How many referendums are in the batch.
			count: u32,
		},
		/// We processed a batch of referendums that we received.
		ReferendumsBatchProcessed {
			/// How many referendums were successfully integrated.
			count_good: u32,
			/// How many referendums failed to integrate.
			count_bad: u32,
		},
		ReferendaProcessed,
		SchedulerMessagesReceived {
			/// How many scheduler messages are in the batch.
			count: u32,
		},
		SchedulerMessagesProcessed {
			/// How many scheduler messages were successfully integrated.
			count_good: u32,
			/// How many scheduler messages failed to integrate.
			count_bad: u32,
		},
		/// Should not happen. Manual intervention by the Fellowship required.
		///
		/// Can happen when existing AH and incoming RC vesting schedules have more combined
		/// entries than allowed. This triggers the merging logic which has henceforth failed
		/// with the given inner pallet-vesting error.
		FailedToMergeVestingSchedules {
			/// The account that failed to merge the schedules.
			who: AccountId32,
			/// The first schedule index that failed to merge.
			schedule1: u32,
			/// The second schedule index that failed to merge.
			schedule2: u32,
			/// The index of the pallet-vesting error that occurred.
			pallet_vesting_error_index: Option<u8>,
		},
		ConvictionVotingMessagesReceived {
			/// How many conviction voting messages are in the batch.
			count: u32,
		},
		ConvictionVotingMessagesProcessed {
			/// How many conviction voting messages were successfully integrated.
			count_good: u32,
		},
		AssetRatesReceived {
			/// How many asset rates are in the batch.
			count: u32,
		},
		AssetRatesProcessed {
			/// How many asset rates were successfully integrated.
			count_good: u32,
			/// How many asset rates failed to integrate.
			count_bad: u32,
		},
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		// TODO: Currently, we use `debug_assert!` for easy test checks against a production
		// snapshot.

		/// Receive accounts from the Relay Chain.
		///
		/// The accounts that sent with `pallet_rc_migrator::Pallet::migrate_accounts` function.
		#[pallet::call_index(0)]
		pub fn receive_accounts(
			origin: OriginFor<T>,
			accounts: Vec<RcAccountFor<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_accounts(accounts)?;

			Ok(())
		}

		/// Receive multisigs from the Relay Chain.
		///
		/// This will be called from an XCM `Transact` inside a DMP from the relay chain. The
		/// multisigs were prepared by
		/// `pallet_rc_migrator::multisig::MultisigMigrator::migrate_many`.
		#[pallet::call_index(1)]
		pub fn receive_multisigs(
			origin: OriginFor<T>,
			accounts: Vec<RcMultisigOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_multisigs(accounts).map_err(Into::into)
		}

		/// Receive proxies from the Relay Chain.
		#[pallet::call_index(2)]
		pub fn receive_proxy_proxies(
			origin: OriginFor<T>,
			proxies: Vec<RcProxyOf<T, T::RcProxyType>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_proxies(proxies).map_err(Into::into)
		}

		/// Receive proxy announcements from the Relay Chain.
		#[pallet::call_index(3)]
		pub fn receive_proxy_announcements(
			origin: OriginFor<T>,
			announcements: Vec<RcProxyAnnouncementOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_proxy_announcements(announcements).map_err(Into::into)
		}

		#[pallet::call_index(4)]
		pub fn receive_preimage_chunks(
			origin: OriginFor<T>,
			chunks: Vec<RcPreimageChunk>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_preimage_chunks(chunks).map_err(Into::into)
		}

		#[pallet::call_index(5)]
		pub fn receive_preimage_request_status(
			origin: OriginFor<T>,
			request_status: Vec<RcPreimageRequestStatusOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_preimage_request_statuses(request_status).map_err(Into::into)
		}

		#[pallet::call_index(6)]
		pub fn receive_preimage_legacy_status(
			origin: OriginFor<T>,
			legacy_status: Vec<RcPreimageLegacyStatusOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_preimage_legacy_statuses(legacy_status).map_err(Into::into)
		}

		#[pallet::call_index(7)]
		pub fn receive_nom_pools_messages(
			origin: OriginFor<T>,
			messages: Vec<RcNomPoolsMessage<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_nom_pools_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(8)]
		pub fn receive_vesting_schedules(
			origin: OriginFor<T>,
			schedules: Vec<RcVestingSchedule<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_vesting_schedules(schedules).map_err(Into::into)
		}

		#[pallet::call_index(9)]
		pub fn receive_fast_unstake_messages(
			origin: OriginFor<T>,
			messages: Vec<RcFastUnstakeMessage<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_fast_unstake_messages(messages).map_err(Into::into)
		}

		/// Receive referendum counts, deciding counts, votes for the track queue.
		#[pallet::call_index(10)]
		pub fn receive_referenda_values(
			origin: OriginFor<T>,
			referendum_count: u32,
			// track_id, count
			deciding_count: Vec<(TrackIdOf<T, ()>, u32)>,
			// referendum_id, votes
			track_queue: Vec<(TrackIdOf<T, ()>, Vec<(u32, u128)>)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_referenda_values(referendum_count, deciding_count, track_queue)
				.map_err(Into::into)
		}

		/// Receive referendums from the Relay Chain.
		#[pallet::call_index(11)]
		pub fn receive_referendums(
			origin: OriginFor<T>,
			referendums: Vec<(u32, RcReferendumInfoOf<T, ()>)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_referendums(referendums).map_err(Into::into)
		}

		#[pallet::call_index(12)]
		pub fn receive_claims(
			origin: OriginFor<T>,
			messages: Vec<RcClaimsMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_claims(messages).map_err(Into::into)
		}

		#[pallet::call_index(13)]
		pub fn receive_bags_list_messages(
			origin: OriginFor<T>,
			messages: Vec<RcBagsListMessage<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_bags_list_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(14)]
		pub fn receive_scheduler_messages(
			origin: OriginFor<T>,
			messages: Vec<scheduler::RcSchedulerMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_scheduler_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(15)]
		pub fn receive_indices(
			origin: OriginFor<T>,
			indices: Vec<RcIndicesIndexOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_indices(indices).map_err(Into::into)
		}

		#[pallet::call_index(16)]
		pub fn receive_conviction_voting_messages(
			origin: OriginFor<T>,
			messages: Vec<RcConvictionVotingMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_conviction_voting_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(17)]
		pub fn receive_bounties_messages(
			origin: OriginFor<T>,
			messages: Vec<pallet_rc_migrator::bounties::RcBountiesMessageOf<T>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_bounties_messages(messages).map_err(Into::into)
		}

		#[pallet::call_index(18)]
		pub fn receive_asset_rates(
			origin: OriginFor<T>,
			rates: Vec<(<T as pallet_asset_rate::Config>::AssetKind, FixedU128)>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_asset_rates(rates).map_err(Into::into)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			Weight::zero()
		}
	}

	impl<T: Config> pallet_rc_migrator::types::MigrationStatus for Pallet<T> {
		fn is_ongoing() -> bool {
			// TODO: implement
			true
		}
		fn is_finished() -> bool {
			// TODO: implement
			false
		}
	}
}
