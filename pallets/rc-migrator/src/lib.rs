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
pub mod preimage;
pub mod proxy;
pub mod referenda;
pub mod staking;
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
		Contains, Defensive, LockableCurrency, ReservableCurrency,
	},
	weights::WeightMeter,
};
use frame_system::{pallet_prelude::*, AccountInfo};
use pallet_balances::AccountData;
use polkadot_parachain_primitives::primitives::Id as ParaId;
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::hrmp;
use sp_core::{crypto::Ss58Codec, H256};
use sp_runtime::AccountId32;
use sp_std::prelude::*;
use storage::TransactionOutcome;
use types::AhWeightInfo;
use weights::WeightInfo;
use xcm::prelude::*;

use accounts::AccountsMigrator;
use multisig::MultisigMigrator;
use preimage::{
	PreimageChunkMigrator, PreimageLegacyRequestStatusMigrator, PreimageRequestStatusMigrator,
};
use proxy::*;
use staking::{
	fast_unstake::{FastUnstakeMigrator, FastUnstakeStage},
	nom_pools::{NomPoolsMigrator, NomPoolsStage},
};
use referenda::ReferendaStage;
use staking::nom_pools::{NomPoolsMigrator, NomPoolsStage};
use types::PalletMigration;

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

pub type MigrationStageOf<T> = MigrationStage<<T as frame_system::Config>::AccountId>;

#[derive(Encode, Decode, Clone, Default, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
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
	FastUnstakeMigrationInit,
	FastUnstakeMigrationOngoing {
		next_key: Option<FastUnstakeStage<AccountId>>,
	},
	FastUnstakeMigrationDone,
	ReferendaMigrationInit,
	ReferendaMigrationOngoing {
		last_key: Option<ReferendaStage>,
	},
	ReferendaMigrationDone,
	MigrationDone,
}

pub type MigrationStageFor<T> = MigrationStage<<T as frame_system::Config>::AccountId>;

impl<T> MigrationStage<T> {
	/// Whether the migration is finished.
	///
	/// This is **not** the same as `!self.is_ongoing()`.
	pub fn is_finished(&self) -> bool {
		matches!(self, MigrationStage::MigrationDone)
	}

	/// Whether the migration is ongoing.
	///
	/// This is **not** the same as `!self.is_finished()`.
	pub fn is_ongoing(&self) -> bool {
		!matches!(self, MigrationStage::Pending | MigrationStage::MigrationDone)
	}
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
		+ pallet_preimage::Config<Hash = H256>
		+ pallet_referenda::Config<Votes = u128>
		+ pallet_nomination_pools::Config
		+ pallet_fast_unstake::Config
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
		/// Contains all calls that are allowed during the migration.
		///
		/// The calls in here will be available again after the migration.
		type RcIntraMigrationCalls: Contains<<Self as frame_system::Config>::RuntimeCall>;
		/// Contains all calls that are allowed after the migration finished.
		type RcPostMigrationCalls: Contains<<Self as frame_system::Config>::RuntimeCall>;
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
					// toggle for testing
					Self::transition(MigrationStage::ProxyMigrationInit);
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
						match MultisigMigrator::<T>::migrate_many(last_key, &mut weight_counter) {
							Ok(last_key) => TransactionOutcome::Commit(Ok(last_key)),
							Err(e) => TransactionOutcome::Rollback(Err(e)),
						}
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
							log::error!(target: LOG_TARGET, "Error while migrating proxies: {:?}", e);
							defensive!("Error while migrating proxies");
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
							log::error!(target: LOG_TARGET, "Error while migrating proxy announcements: {:?}", e);
							defensive!("Error while migrating proxy announcements");
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
							log::error!(target: LOG_TARGET, "Error while migrating preimages: {:?}", e);
							defensive!("Error while migrating preimages");
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
							log::error!(target: LOG_TARGET, "Error while migrating preimage request status: {:?}", e);
							defensive!("Error while migrating preimage request status");
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
							log::error!(target: LOG_TARGET, "Error while migrating legacy preimage request status: {:?}", e);
							defensive!("Error while migrating legacy preimage request status");
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
							log::error!(target: LOG_TARGET, "Error while migrating nom pools: {:?}", e);
							defensive!("Error while migrating nom pools");
						},
					}
				},
				MigrationStage::NomPoolsMigrationDone => {
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
							log::error!(target: LOG_TARGET, "Error while migrating fast unstake: {:?}", e);
							defensive!("Error while migrating fast unstake");
						},
					}
				},
				MigrationStage::FastUnstakeMigrationDone => {
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
							log::error!(target: LOG_TARGET, "Error while migrating referenda: {:?}", err);
						},
					}
				},
				MigrationStage::ReferendaMigrationDone => {
					Self::transition(MigrationStage::MigrationDone);
				},
				MigrationStage::MigrationDone => (),
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

		/// Split up the items into chunks of `MAX_XCM_SIZE` and send them as separate XCM
		/// transacts.
		///
		/// Will modify storage in the error path.
		/// This is done to avoid exceeding the XCM message size limit.
		pub fn send_chunked_xcm<E: Encode>(
			mut items: Vec<E>,
			create_call: impl Fn(Vec<E>) -> types::AhMigratorCall<T>,
		) -> Result<(), Error<T>> {
			log::info!(target: LOG_TARGET, "Received {} items to batch send via XCM", items.len());
			items.reverse();

			while !items.is_empty() {
				let mut remaining_size: u32 = MAX_XCM_SIZE;
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

				log::info!(target: LOG_TARGET, "Sending XCM batch of {} items", batch.len());
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

		/// Send a single XCM message.
		pub fn send_xcm(call: types::AhMigratorCall<T>) -> Result<(), Error<T>> {
			log::info!(target: LOG_TARGET, "Sending XCM message");

			let call = types::AssetHubPalletConfig::<T>::AhmController(call);

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

			Ok(())
		}
	}
}

impl<T: Config> Contains<<T as frame_system::Config>::RuntimeCall> for Pallet<T> {
	fn contains(call: &<T as frame_system::Config>::RuntimeCall) -> bool {
		let stage = RcMigrationStage::<T>::get();

		// We have to return whether the call is allowed:
		const ALLOWED: bool = true;
		const FORBIDDEN: bool = false;

		if stage.is_finished() && !T::RcIntraMigrationCalls::contains(call) {
			return FORBIDDEN;
		}

		if stage.is_ongoing() && !T::RcPostMigrationCalls::contains(call) {
			return FORBIDDEN;
		}

		ALLOWED
	}
}
