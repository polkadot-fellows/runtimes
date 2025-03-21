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

//! Account balance migration.

use crate::*;
use pallet_rc_migrator::accounts::AccountsMigrator;

impl<T: Config> Pallet<T> {
	pub fn do_receive_accounts(
		accounts: Vec<RcAccount<T::AccountId, T::Balance, T::RcHoldReason, T::RcFreezeReason>>,
	) -> Result<(), Error<T>> {
		log::info!(target: LOG_TARGET, "Integrating {} accounts", accounts.len());

		Self::deposit_event(Event::<T>::BatchReceived {
			pallet: PalletEventName::Balances,
			count: accounts.len() as u32,
		});
		let (mut count_good, mut count_bad) = (0, 0);

		for account in accounts {
			let res = with_transaction_opaque_err::<(), RcAccountFor<T>, _>(|| {
				match Self::do_receive_account(account.clone()) {
					Ok(()) => TransactionOutcome::Commit(Ok(())),
					Err(_) => TransactionOutcome::Rollback(Err(account)),
				}
			})
			.expect("Always returning Ok; qed");

			if let Err(account) = res {
				// unlikely to happen cause we dry run migration, but we keep it for completeness.
				count_bad += 1;
				let who = account.who.clone();
				log::error!(target: LOG_TARGET, "Saving the failed account data: {:?}", who.to_ss58check());
				RcAccounts::<T>::insert(&who, account);
			} else {
				count_good += 1;
			}
		}

		Self::deposit_event(Event::<T>::BatchProcessed {
			pallet: PalletEventName::Balances,
			count_good,
			count_bad,
		});
		Ok(())
	}

	/// MAY CHANGED STORAGE ON ERROR RETURN
	pub fn do_receive_account(
		account: RcAccount<T::AccountId, T::Balance, T::RcHoldReason, T::RcFreezeReason>,
	) -> Result<(), Error<T>> {
		if !Self::has_existential_deposit(&account) {
			frame_system::Pallet::<T>::inc_providers(&account.who);
		}

		let who = account.who;
		let total_balance = account.free + account.reserved;
		let minted = match <T as pallet::Config>::Currency::mint_into(&who, total_balance) {
			Ok(minted) => minted,
			Err(e) => {
				log::error!(
					target: LOG_TARGET,
					"Failed to mint into account {}: {:?}",
					who.to_ss58check(),
					e
				);
				return Err(Error::<T>::FailedToProcessAccount);
			},
		};
		debug_assert!(minted == total_balance);

		for hold in account.holds {
			if let Err(e) = <T as pallet::Config>::Currency::hold(
				&T::RcToAhHoldReason::convert(hold.id),
				&who,
				hold.amount,
			) {
				log::error!(
					target: LOG_TARGET,
					"Failed to hold into account {}: {:?}",
					who.to_ss58check(),
					e
				);
				return Err(Error::<T>::FailedToProcessAccount);
			}
		}

		if let Err(e) = <T as pallet::Config>::Currency::reserve(&who, account.unnamed_reserve) {
			log::error!(
				target: LOG_TARGET,
				"Failed to reserve into account {}: {:?}",
				who.to_ss58check(),
				e
			);
			return Err(Error::<T>::FailedToProcessAccount);
		}

		for freeze in account.freezes {
			if let Err(e) = <T as pallet::Config>::Currency::set_freeze(
				&T::RcToAhFreezeReason::convert(freeze.id),
				&who,
				freeze.amount,
			) {
				log::error!(
					target: LOG_TARGET,
					"Failed to freeze into account {}: {:?}",
					who.to_ss58check(),
					e
				);
				return Err(Error::<T>::FailedToProcessAccount);
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
				return Err(Error::<T>::FailedToProcessAccount);
			}
		}
		for _ in 0..account.providers {
			frame_system::Pallet::<T>::inc_providers(&who);
		}

		Ok(())
	}

	/// Returns true if the account has an existential deposit and it does not need an extra
	/// provider reference to exist.
	pub fn has_existential_deposit(
		account: &RcAccount<T::AccountId, T::Balance, T::RcHoldReason, T::RcFreezeReason>,
	) -> bool {
		frame_system::Pallet::<T>::providers(&account.who) > 0 ||
			<T as pallet::Config>::Currency::balance(&account.who).saturating_add(account.free) >=
				<T as pallet::Config>::Currency::minimum_balance()
	}

	pub fn finish_accounts_migration(rc_balance_kept: T::Balance) -> Result<(), Error<T>> {
		use frame_support::traits::Currency;
		let checking_account = T::CheckingAccount::get();
		// current value is the migrated checking balance of RC
		let rc_balance_before =
			<<T as pallet::Config>::Currency as Currency<_>>::total_balance(&checking_account);
		let ah_issuance_before = AhTotalIssuanceBefore::<T>::get();

		// set it to the correct value:
		let balance_after = rc_balance_before
			.checked_sub(ah_issuance_before) // amount teleported to other system chains
			.ok_or(Error::<T>::FailedToCalculateCheckingAccount)?
			.checked_add(rc_balance_kept) // total amount teleported to other chains
			.ok_or(Error::<T>::FailedToCalculateCheckingAccount)?;
		<T as Config>::Currency::make_free_balance_be(&checking_account, balance_after);
		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for AccountsMigrator<T> {
	// rc_total_issuance_before
	type RcPrePayload = BalanceOf<T>;
	type AhPrePayload = ();

	/// Run some checks on asset hub before the migration and store intermediate payload.
	///
	/// The expected output should contain the data stored in asset hub before the migration.
	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		let check_account = T::CheckingAccount::get();
		let checking_balance = <T as Config>::Currency::total_balance(&check_account);
		//assert_eq!(checking_balance, 100000000); // TODO Adrian
	}

	/// Run some checks after the migration and use the intermediate payload.
	///
	/// The expected input should contain the data just transferred out of the relay chain, to allow
	/// the check that data has been correctly migrated to asset hub. It should also contain the
	/// data previously stored in asset hub, allowing for more complex logical checks on the
	/// migration outcome.
	fn post_check(rc_total_issuance_before: Self::RcPrePayload, _: Self::AhPrePayload) {
		let ah_total_issuance = <T as Config>::Currency::total_issuance();
		// assert RC total issuance before == AH total issuance after
		// assert_eq!(rc_total_issuance_before, ah_total_issuance); // TODO Adrian
	}
}
