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

//! The helper pallet for the Asset Hub migration meant to be setup on Relay Chain.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod accounts;
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
		LockableCurrency, ReservableCurrency,
	},
	weights::WeightMeter,
};
use frame_system::{pallet_prelude::*, AccountInfo};
use pallet_balances::AccountData;
use polkadot_parachain_primitives::primitives::Id as ParaId;
use polkadot_runtime_common::paras_registrar;
use runtime_parachains::hrmp;
use sp_core::crypto::Ss58Codec;
use sp_runtime::AccountId32;
use sp_std::prelude::*;
use storage::TransactionOutcome;
use types::AhWeightInfo;
use weights::WeightInfo;
use xcm::prelude::*;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::rc-migrator";

#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum MigrationStage<AccountId> {
	/// The migration has not yet started.
	#[default]
	Pending,
	// Initializing
	/// Migrating account balances.
	MigratingAccounts {
		// Last migrated account
		last_key: Option<AccountId>,
	},
	/// Some next stage
	TODO,
}

type AccountInfoFor<T> =
	AccountInfo<<T as frame_system::Config>::Nonce, <T as frame_system::Config>::AccountData>;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	/// Paras Registrar Pallet
	type ParasRegistrar<T> = paras_registrar::Pallet<T>;

	/// Super trait of all pallets the migration depends on.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountData = AccountData<u128>, AccountId = AccountId32>
		+ pallet_balances::Config<Balance = u128>
		+ hrmp::Config
		+ paras_registrar::Config
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
	}

	#[pallet::error]
	pub enum Error<T> {
		/// TODO
		TODO,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(crate) fn deposit_event)]
	pub enum Event<T: Config> {
		/// TODO
		TODO,
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
		pub fn _do_something(_origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			Ok(().into())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let mut weight_counter = WeightMeter::with_limit(T::MaxRcWeight::get());
			let stage = RcMigrationStage::<T>::get();
			weight_counter.consume(T::DbWeight::get().reads(1));

			if stage == MigrationStage::Pending {
				// TODO: not complete

				let _ = Self::obtain_rc_accounts();

				return weight_counter.consumed();
			}

			// TODO init

			if let MigrationStage::MigratingAccounts { last_key } = stage {
				let res =
					with_transaction_opaque_err::<Option<Option<T::AccountId>>, (), _>(|| {
						match Self::migrate_accounts(last_key, &mut weight_counter) {
							Ok(ok) => TransactionOutcome::Commit(Ok(ok)),
							Err(err) => TransactionOutcome::Commit(Err(err)),
						}
					});

				match res {
					Ok(Ok(None)) => {
						// accounts migration is completed
						// TODO publish event
						RcMigrationStage::<T>::put(MigrationStage::TODO);
					},
					Ok(Ok(Some(last_key))) => {
						// accounts migration continues with the next block
						// TODO publish event
						RcMigrationStage::<T>::put(MigrationStage::MigratingAccounts { last_key });
					},
					_ => {
						// TODO handle error
					},
				}
				return weight_counter.consumed();
			};

			return weight_counter.consumed();
		}
	}
}
