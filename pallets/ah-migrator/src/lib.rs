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

pub mod multisig;
pub mod types;
pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	storage::{transactional::with_transaction_opaque_err, TransactionOutcome},
	traits::{
		fungible::{InspectFreeze, Mutate, MutateFreeze, MutateHold},
		LockableCurrency, ReservableCurrency, WithdrawReasons as LockWithdrawReasons,
	},
};
use frame_system::pallet_prelude::*;
use pallet_balances::{AccountData, Reasons as LockReasons};
use pallet_rc_migrator::{accounts::Account as RcAccount, multisig::*};
use sp_application_crypto::Ss58Codec;
use sp_runtime::{traits::Convert, AccountId32};
use sp_std::prelude::*;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::ah-migrator";

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
			accounts: Vec<RcAccount<T::AccountId, T::Balance, T::RcHoldReason, T::RcFreezeReason>>,
		) -> DispatchResult {
			ensure_root(origin)?;

			Self::do_receive_accounts(accounts)?;
			// TODO: publish event

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
	}

	impl<T: Config> Pallet<T> {
		fn do_receive_accounts(
			accounts: Vec<RcAccount<T::AccountId, T::Balance, T::RcHoldReason, T::RcFreezeReason>>,
		) -> Result<(), Error<T>> {
			log::info!(target: LOG_TARGET, "Integrating {} accounts", accounts.len());

			for account in accounts {
				let _: Result<(), ()> = with_transaction_opaque_err::<(), (), _>(|| {
					match Self::do_receive_account(account) {
						Ok(()) => TransactionOutcome::Commit(Ok(())),
						Err(_) => TransactionOutcome::Rollback(Ok(())),
					}
				})
				.expect("Always returning Ok; qed");
			}

			Ok(())
		}

		/// MAY CHANGED STORAGE ON ERROR RETURN
		fn do_receive_account(
			account: RcAccount<T::AccountId, T::Balance, T::RcHoldReason, T::RcFreezeReason>,
		) -> Result<(), Error<T>> {
			let who = account.who;
			let total_balance = account.free + account.reserved;
			let minted = match <T as pallet::Config>::Currency::mint_into(&who, total_balance) {
				Ok(minted) => minted,
				Err(e) => {
					log::error!(target: LOG_TARGET, "Failed to mint into account {}: {:?}", who.to_ss58check(), e);
					return Err(Error::<T>::TODO);
				},
			};
			debug_assert!(minted == total_balance);

			for hold in account.holds {
				if let Err(e) = <T as pallet::Config>::Currency::hold(
					&T::RcToAhHoldReason::convert(hold.id),
					&who,
					hold.amount,
				) {
					log::error!(target: LOG_TARGET, "Failed to hold into account {}: {:?}", who.to_ss58check(), e);
					return Err(Error::<T>::TODO);
				}
			}

			if let Err(e) = <T as pallet::Config>::Currency::reserve(&who, account.unnamed_reserve)
			{
				log::error!(target: LOG_TARGET, "Failed to reserve into account {}: {:?}", who.to_ss58check(), e);
				return Err(Error::<T>::TODO);
			}

			for freeze in account.freezes {
				if let Err(e) = <T as pallet::Config>::Currency::set_freeze(
					&T::RcToAhFreezeReason::convert(freeze.id),
					&who,
					freeze.amount,
				) {
					log::error!(target: LOG_TARGET, "Failed to freeze into account {}: {:?}", who.to_ss58check(), e);
					return Err(Error::<T>::TODO);
				}
			}

			for lock in account.locks {
				<T as pallet::Config>::Currency::set_lock(
					lock.id,
					&who,
					lock.amount,
					types::map_lock_reason(lock.reasons),
				);
			}

			log::debug!(
				target: LOG_TARGET,
				"Integrating account: {}", who.to_ss58check(),
			);

			// TODO run some post-migration sanity checks

			// Apply all additional consumers that were excluded from the balance stuff above:
			for _ in 0..account.consumers {
				if let Err(e) = frame_system::Pallet::<T>::inc_consumers(&who) {
					log::error!(target: LOG_TARGET, "Failed to inc consumers for account {}: {:?}", who.to_ss58check(), e);
					return Err(Error::<T>::TODO);
				}
			}
			for _ in 0..account.providers {
				frame_system::Pallet::<T>::inc_providers(&who);
			}

			// TODO: publish event
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			Weight::zero()
		}
	}
}
