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
pub mod multisig;
pub mod proxy;
pub mod types;
mod weights;
pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	sp_runtime::traits::AccountIdConversion,
	storage::transactional::with_transaction_opaque_err,
	traits::{
		fungible::{Inspect, InspectFreeze, Mutate, MutateFreeze, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
		Defensive, LockableCurrency, ReservableCurrency,
	},
	weights::WeightMeter,
};
use frame_system::{pallet_prelude::*, AccountInfo};
use pallet_balances::AccountData;
use polkadot_parachain_primitives::primitives::Id as ParaId;
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::hrmp;
use sp_core::crypto::Ss58Codec;
use sp_runtime::{traits::TryConvert, AccountId32};
use sp_std::prelude::*;
use storage::TransactionOutcome;
use types::AhWeightInfo;
use weights::WeightInfo;
use xcm::prelude::*;

use accounts::AccountsMigrator;
use multisig::MultisigMigrator;
use proxy::*;
use types::PalletMigration;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::rc-migrator";

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum MigrationStage<AccountId> {
	/// The migration has not yet started but will start in the next block.
	#[default]
	Pending,
	/// Initializing the account migration process.
	AccountsMigrationInit,
	// TODO: Initializing?
	/// Migrating account balances.
	AccountsMigrationOngoing {
		// Last migrated account
		last_key: Option<AccountId>,
	},
	/// Accounts migration is done. Ready to go to the next one.
	///
	/// Note that this stage does not have any logic attached to itself. It just exists to make it
	/// easier to swap out what stage should run next for testing.
	AccountsMigrationDone,
	MultisigMigrationInit,
	MultisigMigrationOngoing {
		/// Last migrated key of the `Multisigs` double map.
		last_key: Option<(AccountId, [u8; 32])>,
	},
	MultisigMigrationDone,
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
	MigrationDone,
}

type AccountInfoFor<T> =
	AccountInfo<<T as frame_system::Config>::Nonce, <T as frame_system::Config>::AccountData>;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	/// Paras Registrar Pallet
	type ParasRegistrar<T> = paras_registrar::Pallet<T>;

	/// Super config trait for all pallets that the migration depends on, providing convenient
	/// access to their items.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountData = AccountData<u128>, AccountId = AccountId32>
		+ pallet_balances::Config<Balance = u128>
		+ hrmp::Config
		+ paras_registrar::Config
		+ pallet_multisig::Config
		+ pallet_proxy::Config
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
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The error that should to be replaced by something meaningful.
		TODO,
		OutOfWeight,
		/// Failed to send XCM message to AH.
		XcmError,
		/// Failed to withdraw account from RC for migration to AH.
		FailedToWithdrawAccount,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A stage transition has occurred.
		StageTransition {
			/// The old stage before the transition.
			old: MigrationStage<T::AccountId>,
			/// The new stage after the transition.
			new: MigrationStage<T::AccountId>,
		},
	}

	/// The Relay Chain migration state.
	#[pallet::storage]
	pub type RcMigrationStage<T: Config> =
		StorageValue<_, MigrationStage<T::AccountId>, ValueQuery>;

	/// Helper storage item to obtain and store the known accounts that should be kept partially on
	/// fully on Relay Chain.
	#[pallet::storage]
	pub type RcAccounts<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, accounts::AccountState<T::Balance>, OptionQuery>;

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
		/// TODO
		#[pallet::call_index(0)]
		#[pallet::weight({1})]
		pub fn do_something(_origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let mut weight_counter = WeightMeter::with_limit(T::MaxRcWeight::get());
			let stage = RcMigrationStage::<T>::get();
			weight_counter.consume(T::DbWeight::get().reads(1));

			match stage {
				MigrationStage::Pending => {
					// TODO: not complete

					Self::transition(MigrationStage::AccountsMigrationInit);
					//Self::transition(MigrationStage::ProxyMigrationInit);
				},
				MigrationStage::AccountsMigrationInit => {
					// TODO: weights
					let _ = AccountsMigrator::<T>::obtain_rc_accounts();

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
							// TODO publish event
							Self::transition(MigrationStage::AccountsMigrationDone);
						},
						Ok(Some(last_key)) => {
							// accounts migration continues with the next block
							// TODO publish event
							Self::transition(MigrationStage::AccountsMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						Err(err) => {
							defensive!("Error while migrating accounts: {:?}", err);
							log::error!(target: LOG_TARGET, "Error while migrating accounts: {:?}", err);
							// stage unchanged, retry.
						},
					}
				},
				MigrationStage::AccountsMigrationDone => {
					// Note: swap this out for faster testing to skip some migrations
					Self::transition(MigrationStage::MultisigMigrationInit);
				},
				MigrationStage::MultisigMigrationInit => {
					Self::transition(MigrationStage::MultisigMigrationOngoing { last_key: None });
				},
				MigrationStage::MultisigMigrationOngoing { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						TransactionOutcome::Commit(MultisigMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						))
					})
					.expect("Always returning Ok; qed");

					match res {
						Ok(None) => {
							// multisig migration is completed
							// TODO publish event
							Self::transition(MigrationStage::MultisigMigrationDone);
						},
						Ok(Some(last_key)) => {
							// multisig migration continues with the next block
							// TODO publish event
							Self::transition(MigrationStage::MultisigMigrationOngoing {
								last_key: Some(last_key),
							});
						},
						e => {
							log::error!(target: LOG_TARGET, "Error while migrating multisigs: {:?}", e);
							defensive!("Error while migrating multisigs");
						},
					}
				},
				MigrationStage::MultisigMigrationDone => {
					Self::transition(MigrationStage::ProxyMigrationInit);
				},
				MigrationStage::ProxyMigrationInit => {
					Self::transition(MigrationStage::ProxyMigrationProxies { last_key: None });
				},
				MigrationStage::ProxyMigrationProxies { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						TransactionOutcome::Commit(ProxyProxiesMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						))
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
							log::error!(target: LOG_TARGET, "Error while migrating proxies: {:?}", e);
							defensive!("Error while migrating proxies");
						},
					}
				},
				MigrationStage::ProxyMigrationAnnouncements { last_key } => {
					let res = with_transaction_opaque_err::<Option<_>, Error<T>, _>(|| {
						TransactionOutcome::Commit(ProxyAnnouncementMigrator::<T>::migrate_many(
							last_key,
							&mut weight_counter,
						))
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
							log::error!(target: LOG_TARGET, "Error while migrating proxy announcements: {:?}", e);
							defensive!("Error while migrating proxy announcements");
						},
					}
				},
				MigrationStage::ProxyMigrationDone => {
					Self::transition(MigrationStage::MigrationDone);
				},
				MigrationStage::MigrationDone => {
					todo!()
				},
			};

			weight_counter.consumed()
		}
	}

	impl<T: Config> Pallet<T> {
		/// Execute a stage transition and log it.
		fn transition(new: MigrationStage<T::AccountId>) {
			let old = RcMigrationStage::<T>::get();
			RcMigrationStage::<T>::put(&new);
			log::info!(target: LOG_TARGET, "[Block {:?}] Stage transition: {:?} -> {:?}", frame_system::Pallet::<T>::block_number(), &old, &new);
			Self::deposit_event(Event::StageTransition { old, new });
		}

		/// Split up the items into chunks of `MAX_MSG_SIZE` and send them as separate XCM
		/// transacts.
		///
		/// Will modify storage in the error path.
		/// This is done to avoid exceeding the XCM message size limit.
		pub fn send_chunked_xcm<E: Encode>(
			mut items: Vec<E>,
			create_call: impl Fn(Vec<E>) -> types::AhMigratorCall<T>,
		) -> Result<(), Error<T>> {
			log::info!(target: LOG_TARGET, "Received {} items to batch send via XCM", items.len());

			const MAX_MSG_SIZE: u32 = 50_000; // Soft message size limit. Hard limit is about 64KiB
									 // Reverse in place so that we can use `pop` later on
			items.reverse();

			while !items.is_empty() {
				let mut remaining_size: u32 = MAX_MSG_SIZE;
				let mut batch = Vec::new();

				while !items.is_empty() {
					// Taking from the back as optimization is fine since we reversed
					let item = items.last().unwrap(); // FAIL-CI no unwrap
					let msg_size = item.encoded_size() as u32;
					if msg_size > remaining_size {
						break;
					}
					remaining_size -= msg_size;

					batch.push(items.pop().unwrap()); // FAIL-CI no unwrap
				}

				log::info!(target: LOG_TARGET, "Sending batch of {} proxies", batch.len());
				let call = types::AssetHubPalletConfig::<T>::AhmController(create_call(batch));

				let message = Xcm(vec![
					Instruction::UnpaidExecution {
						weight_limit: WeightLimit::Unlimited,
						check_origin: None,
					},
					Instruction::Transact {
						origin_kind: OriginKind::Superuser,
						require_weight_at_most: Weight::from_all(1), // TODO
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
			}

			Ok(())
		}
	}
}
