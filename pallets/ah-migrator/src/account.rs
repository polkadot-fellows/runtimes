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
use frame_support::traits::DefensiveSaturating;

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
		let total_balance = account.free.saturating_add(account.reserved);
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

		log::trace!(
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
		let balances_before = AhBalancesBefore::<T>::get();
		// current value is the AH checking balance + migrated checking balance of RC
		let checking_balance =
			<<T as pallet::Config>::Currency as Currency<_>>::total_balance(&checking_account);

		/* Arithmetics explanation:
		At this point, because checking account was completely migrated:
			`checking_balance` = ah_check_before + rc_check_before
			(0) rc_check_before = `checking_balance` - ah_check_before

		Invariants:
			(1) rc_check_before = sum_total_before(ah, bh, collectives, coretime, people)
			(2) rc_check_before = sum_total_before(bh, collectives, coretime, people) + ah_total_before
		Because teleports are disabled for RC and AH during migration, we can say:
			(3) sum_total_before(bh, collectives, coretime, people) = sum_total_after(bh, collectives, coretime, people)
		Ergo use (3) in (2):
			(4) rc_check_before = sum_total_after(bh, collectives, coretime, people) + ah_total_before

		We want:
			ah_check_after = sum_total_after(rc, bh, collectives, coretime, people)
			ah_check_after = sum_total_after(bh, collectives, coretime, people) + rc_balance_kept
		Use (3):
			ah_check_after = sum_total_before(bh, collectives, coretime, people) + rc_balance_kept
			ah_check_after = sum_total_before(ah, bh, collectives, coretime, people) - ah_total_before + rc_balance_kept
		Use (1):
			ah_check_after = rc_check_before - ah_total_before + rc_balance_kept
		Use (0):
			ah_check_after = `checking_balance` - ah_check_before - ah_total_before + rc_balance_kept
			ah_check_after = `checking_balance` + rc_balance_kept - ah_total_before - ah_check_before
		*/
		// set it to the correct value:
		let balance_after = checking_balance
			.checked_add(rc_balance_kept)
			.ok_or(Error::<T>::FailedToCalculateCheckingAccount)?
			.checked_sub(balances_before.total_issuance)
			.ok_or(Error::<T>::FailedToCalculateCheckingAccount)?
			.checked_sub(balances_before.checking_account)
			.ok_or(Error::<T>::FailedToCalculateCheckingAccount)?;
		<T as Config>::Currency::make_free_balance_be(&checking_account, balance_after);
		Ok(())
	}
}

#[cfg(feature = "std")]
pub mod tests {
	use super::*;
	use pallet_rc_migrator::accounts::tests::{
		AccountsMigrationChecker, BalanceSummary, ChainType,
	};

	use std::collections::BTreeMap;

	impl<T: Config> crate::types::AhMigrationCheck for AccountsMigrationChecker<T> {
		// The RC payload is a mapping from account to a summary of their balances, including holds,
		// reserves, locks, and freezes. The second item is the total issuance on the relay chain
		// before migration.
		// The AH payload is a mapping from account to (ah_reserved_pre, ah_free_pre), i.e., the
		// reserved balance on Asset Hub before migration and the free balance on Asset Hub before
		// migration.
		type RcPrePayload = (BTreeMap<T::AccountId, BalanceSummary>, u128);
		type AhPrePayload = BTreeMap<T::AccountId, (u128, u128)>;

		/// Run some checks on asset hub before the migration and store intermediate payload.
		///
		/// The expected output should contain the data stored in asset hub before the migration.
		fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
			// Assert storage "Balances::Locks::ah_pre::empty"
			assert!(
				pallet_balances::Locks::<T>::iter().next().is_none(),
				"No locks should exist on Asset Hub before migration"
			);

			// Assert storage "Balances::Reserves::ah_pre::empty"
			assert!(
				pallet_balances::Reserves::<T>::iter().next().is_none(),
				"No reserves should exist on Asset Hub before migration"
			);

			// Assert storage "Balances::Freezes::ah_pre::empty"
			assert!(
				pallet_balances::Freezes::<T>::iter().next().is_none(),
				"No freezes should exist on Asset Hub before migration"
			);

			let check_account = T::CheckingAccount::get();
			let checking_balance = <T as Config>::Currency::total_balance(&check_account);
			// AH checking account has incorrect 0.01 DOT balance because of the DED airdrop which
			// added DOT ED to all existing AH accounts.
			// This is fine, we can just ignore/accept this small amount.
			defensive_assert!(checking_balance == <T as Config>::Currency::minimum_balance());

			let mut ah_pre_payload = BTreeMap::new();
			for (account, _) in frame_system::Account::<T>::iter() {
				let free = <T as Config>::Currency::balance(&account);
				let reserved = <T as Config>::Currency::reserved_balance(&account);
				ah_pre_payload.insert(account, (reserved, free));
			}
			ah_pre_payload
		}

		/// Run some checks after the migration and use the intermediate payload.
		///
		/// The expected input should contain the data just transferred out of the relay chain, to
		/// allow the check that data has been correctly migrated to asset hub. It should also
		/// contain the data previously stored in asset hub, allowing for more complex logical
		/// checks on the migration outcome.
		fn post_check(rc_pre_payload: Self::RcPrePayload, ah_pre_payload: Self::AhPrePayload) {
			// Check that no failed accounts remain in storage
			assert!(
				RcAccounts::<T>::iter().next().is_none(),
				"Failed accounts should not remain in storage after migration"
			);

			let (account_summaries, _) = rc_pre_payload;
			for (who, summary) in account_summaries {
				let ah_free_post = <T as Config>::Currency::balance(&who);
				let ah_reserved_post = <T as Config>::Currency::reserved_balance(&who);
				let mut ah_free_before = 0;
				let mut ah_reserved_before = 0;
				if let Some((ah_reserved_pre, ah_free_pre)) = ah_pre_payload.get(&who) {
					ah_free_before = *ah_free_pre;
					ah_reserved_before = *ah_reserved_pre;
				}

				let frozen = frame_system::Account::<T>::get(&who).data.frozen;
				let mut holds_enc = Vec::new();
				for hold in pallet_balances::Holds::<T>::get(&who) {
					holds_enc.push((
						Self::hold_id_encoding(hold.id.encode(), ChainType::AH),
						hold.amount,
					));
				}
				let mut freezes_enc = Vec::new();
				for freeze in pallet_balances::Freezes::<T>::get(&who) {
					freezes_enc.push((
						Self::freeze_id_encoding(freeze.id.encode(), ChainType::AH),
						freeze.amount,
					));
				}
				let mut locks_enc = Vec::new();
				for lock in pallet_balances::Locks::<T>::get(&who) {
					locks_enc.push((
						Self::lock_id_encoding(lock.id, ChainType::AH),
						lock.amount,
						lock.reasons as u8,
					));
				}

				let rc_migrated_balance =
					summary.migrated_free.saturating_add(summary.migrated_reserved);
				let ah_migrated_balance = ah_free_post
					.saturating_sub(ah_free_before)
					.saturating_add(ah_reserved_post.saturating_sub(ah_reserved_before));
				let ah_ed: u128 = <T as Config>::Currency::minimum_balance();

				// In case the balance migrated to AH is less than the existential deposit, minting
				// the balance may fail. Therefore, we just check that the difference between
				// the balance migrated from the RC to AH and the balance delta on AH before and
				// after migration is less than AH existential deposit.
				assert!(rc_migrated_balance.saturating_sub(ah_migrated_balance) <= ah_ed,
					"Total balance mismatch for account {:?} between RC pre-migration and AH post-migration",
					who.to_ss58check()
				);

				// There are several `unreserve` operations on AH after migration (e.g., unreserve
				// deposits for multisigs because they are not migrated to AH, adjust deposits for
				// preimages, ...). Therefore, we just check that the change in reserved balance on
				// AH after migration is less than the migrated reserved balance from RC.
				assert!(
					ah_reserved_post.saturating_sub(ah_reserved_before) <= summary.migrated_reserved,
					"Change in reserved balance on AH after migration for account {:?} is greater than the migrated reserved balance from RC", 
					who.to_ss58check()
				);

				// There should be no frozen balance on AH before the migration so we just need to
				// check that the frozen balance on AH after migration is the same as on RC
				// before migration.
				assert_eq!(
					summary.frozen,
					frozen,
					"Frozen balance mismatch for account {:?} between RC pre-migration and AH post-migration",
					who.to_ss58check()
				);

				// Holds migrated from RC may be merged on AH after migration. Therefore, we need to
				// check that, for each hold reason, the sum of the hold amounts on AH after
				// migration is the same as the sum of the hold amounts on RC before migration.
				for (hold_id, hold_amount) in holds_enc {
					let mut rc_hold_amount_for_id: u128 = 0;
					for (id, amount) in summary.holds.clone() {
						if id == hold_id {
							rc_hold_amount_for_id = rc_hold_amount_for_id.saturating_add(amount);
						}
					}
					assert_eq!(
						rc_hold_amount_for_id,
						hold_amount,
						"Hold amount mismatch for account {:?} and hold id {:?} between RC pre-migration and AH post-migration",
						who.to_ss58check(),
						hold_id
					);
				}

				// There should be no locks on AH before the migration so we just need to check that
				// the locks on AH after migration are the same as on RC before migration.
				assert_eq!(
					summary.locks,
					locks_enc,
					"Locks mismatch for account {:?} between RC pre-migration and AH post-migration",
					who.to_ss58check()
				);

				// There should be no freezes on AH before the migration so we just need to check
				// that the freezes on AH after migration are the same as on RC before
				// migration.
				assert_eq!(
					summary.freezes,
					freezes_enc,
					"Freezes mismatch for account {:?} between RC pre-migration and AH post-migration",
					who.to_ss58check()
				);
			}
		}
	}
}
