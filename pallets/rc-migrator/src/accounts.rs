// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! Account/Balance data migrator module.

use crate::{types::*, *};
use codec::DecodeAll;
use frame_support::{
	traits::tokens::{Balance as BalanceT, IdAmount},
	weights::WeightMeter,
};
use frame_system::Account as SystemAccount;
use pallet_balances::{AccountData, BalanceLock};
use sp_core::ByteArray;
use sp_runtime::{traits::Zero, BoundedVec};
#[cfg(feature = "std")]
use std::collections::BTreeMap;

/// Account type meant to transfer data between RC and AH.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "stable2503", derive(DecodeWithMemTracking))]
pub struct Account<AccountId, Balance, HoldReason, FreezeReason> {
	/// The account address
	pub who: AccountId,
	/// Free balance.
	///
	/// `free` + `reserved` - the total balance to be minted for `who` on the Asset Hub.
	pub free: Balance,
	/// Reserved balance.
	///
	/// This is not used to establish the reserved balance on the Asset Hub, but used to assert the
	/// total reserve balance after applying all `holds` and `unnamed_reserve`.
	pub reserved: Balance,
	/// Frozen balance.
	///
	/// This is not used to establish the reserved balance on the Asset Hub, but used to assert the
	/// total reserve balance after applying all `freezes` and `locks`.
	pub frozen: Balance,
	/// Account holds from Relay Chain.
	///
	/// Expected hold reasons (HoldReason):
	/// - DelegatedStaking: StakingDelegation (only on Kusama)
	/// - Preimage: Preimage
	/// - Staking: Staking - later instead of "staking " lock, moved to staking_async pallet on AH
	pub holds: BoundedVec<IdAmount<HoldReason, Balance>, ConstU32<5>>,
	/// Account freezes from Relay Chain.
	///
	/// Expected freeze reasons (FreezeReason):
	/// - NominationPools: PoolMinBalance
	pub freezes: BoundedVec<IdAmount<FreezeReason, Balance>, ConstU32<5>>,
	/// Account locks from Relay Chain.
	///
	/// Expected lock ids:
	/// - "staking " : pallet-staking locks have been transformed to holds with https://github.com/paritytech/polkadot-sdk/pull/5501
	/// but the conversion was lazy, so there may be some staking locks left
	/// - "vesting " : pallet-vesting
	/// - "pyconvot" : pallet-conviction-voting
	pub locks: BoundedVec<BalanceLock<Balance>, ConstU32<5>>,
	/// Unnamed reserve.
	///
	/// Only unnamed reserves for Polkadot and Kusama (no named ones).
	pub unnamed_reserve: Balance,
	/// Consumer ref count of migrating to Asset Hub pallets except a reference for `reserved` and
	/// `frozen` balance.
	///
	/// Since the `reserved` and `frozen` balances will be known on a receiving side (AH) they will
	/// be calculated there.
	pub consumers: u8,
	/// Provider ref count of migrating to Asset Hub pallets except the reference for existential
	/// deposit.
	///
	/// Since the `free` balance will be known on a receiving side (AH) the ref count will be
	/// calculated there.
	pub providers: u8,
}

impl<AccountId, Balance: Zero, HoldReason, FreezeReason>
	Account<AccountId, Balance, HoldReason, FreezeReason>
{
	/// Check if the total account balance is liquid.
	pub fn is_liquid(&self) -> bool {
		self.unnamed_reserve.is_zero() &&
			self.freezes.is_empty() &&
			self.locks.is_empty() &&
			self.holds.is_empty()
	}
}

/// The state for the Relay Chain accounts.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "stable2503", derive(DecodeWithMemTracking))]
pub enum AccountState<Balance> {
	/// The account should be migrated to AH and removed on RC.
	Migrate,

	/// The account must stay on RC with its balance.
	///
	/// E.g., RC system account.
	Preserve,

	// We might not need the `Part` variation since there are no many cases for `Part` we can just
	// keep the whole account balance on RC
	/// The part of the account must be preserved on RC.
	///
	/// Cases:
	/// - accounts placed deposit for parachain registration (paras_registrar pallet);
	/// - accounts placed deposit for hrmp channel registration (parachains_hrmp pallet);
	/// - accounts storing the keys within the session pallet with a consumer reference.
	Part {
		/// The free balance that must be preserved on RC.
		///
		/// Includes ED.
		free: Balance,
		/// The reserved balance that must be preserved on RC.
		///
		/// In practice reserved by old `Currency` api and has no associated reason.
		reserved: Balance,
		/// The number of consumers that must be preserved on RC.
		///
		/// Generally one consumer reference of reserved balance or/and consumer reference of the
		/// session pallet.
		consumers: u32,
	},
}

impl<Balance: BalanceT> AccountState<Balance> {
	/// Account must be fully preserved on RC.
	pub fn is_preserve(&self) -> bool {
		matches!(self, AccountState::Preserve)
	}
	/// Get the free balance on RC.
	pub fn get_rc_free(&self) -> Balance {
		match self {
			// preserve the `free` balance on RC.
			AccountState::Part { free, .. } => *free,
			// no free balance on RC, migrate the entire account balance.
			AccountState::Migrate => Balance::zero(),
			AccountState::Preserve => {
				defensive!("Account must be preserved on RC");
				Balance::zero()
			},
		}
	}
	/// Get the reserved balance on RC.
	pub fn get_rc_reserved(&self) -> Balance {
		match self {
			AccountState::Part { reserved, .. } => *reserved,
			AccountState::Migrate => Balance::zero(),
			AccountState::Preserve => {
				defensive!("Account must be preserved on RC");
				Balance::zero()
			},
		}
	}
	/// Get the consumer count on RC.
	pub fn get_rc_consumers(&self) -> u32 {
		match self {
			AccountState::Part { consumers, .. } => *consumers,
			// accounts fully migrating to AH will have a consumer count of `0` on Relay Chain since
			// all holds and freezes are removed.
			AccountState::Migrate => 0,
			AccountState::Preserve => {
				defensive!("Account must be preserved on RC");
				0
			},
		}
	}
}

pub type AccountStateFor<T> = AccountState<<T as pallet_balances::Config>::Balance>;
pub type AccountFor<T> = Account<
	<T as frame_system::Config>::AccountId,
	<T as pallet_balances::Config>::Balance,
	<T as pallet_balances::Config>::RuntimeHoldReason,
	<T as pallet_balances::Config>::FreezeIdentifier,
>;

/// Helper struct tracking total balance kept on RC and total migrated.
#[derive(Encode, Decode, Default, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
#[cfg_attr(feature = "stable2503", derive(DecodeWithMemTracking))]
pub struct MigratedBalances<Balance: Default> {
	pub kept: Balance,
	pub migrated: Balance,
}

pub struct AccountsMigrator<T> {
	_phantom: sp_std::marker::PhantomData<T>,
}

impl<T: Config> PalletMigration for AccountsMigrator<T> {
	type Key = T::AccountId;
	type Error = Error<T>;

	/// Migrate accounts from RC to AH.
	///
	/// Parameters:
	/// - `last_key` - the last migrated account from RC to AH if any
	/// - `weight_counter` - the weight meter
	///
	/// Result:
	/// - None - no accounts left to be migrated to AH.
	/// - Some(maybe_last_key) - the last migrated account from RC to AH if any
	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Error<T>> {
		// we should not send more than we allocated on AH for the migration.
		let mut ah_weight = WeightMeter::with_limit(T::MaxAhWeight::get());
		// accounts batch for the current iteration.
		let mut batch = XcmBatchAndMeter::new_from_config::<T>();

		let mut iter = if let Some(ref last_key) = last_key {
			SystemAccount::<T>::iter_from_key(last_key)
		} else {
			SystemAccount::<T>::iter()
		};

		let mut maybe_last_key = last_key;
		loop {
			// account the weight for migrating a single account on Relay Chain.
			if weight_counter.try_consume(T::RcWeightInfo::withdraw_account()).is_err() ||
				weight_counter.try_consume(batch.consume_weight()).is_err()
			{
				log::info!("RC weight limit reached at batch length {}, stopping", batch.len());
				if batch.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			let Some((who, account_info)) = iter.next() else {
				maybe_last_key = None;
				break;
			};

			let withdraw_res =
				with_transaction_opaque_err::<Option<AccountFor<T>>, Error<T>, _>(|| {
					match Self::withdraw_account(
						who.clone(),
						account_info.clone(),
						&mut ah_weight,
						batch.len() as u32,
					) {
						Ok(ok) => TransactionOutcome::Commit(Ok(ok)),
						Err(e) => TransactionOutcome::Rollback(Err(e)),
					}
				})
				.expect("Always returning Ok; qed");

			match withdraw_res {
				// Account does not need to be migrated
				Ok(None) => {
					// if this the last account to handle at this iteration, we skip it next time.
					maybe_last_key = Some(who);
					continue;
				},
				Ok(Some(ah_account)) => {
					// if this the last account to handle at this iteration, we skip it next time.
					maybe_last_key = Some(who);
					batch.push(ah_account)
				},
				// Not enough weight, lets try again in the next block since we made some progress.
				Err(Error::OutOfWeight) if !batch.is_empty() => {
					break;
				},
				// Not enough weight and was unable to make progress, bad.
				Err(Error::OutOfWeight) if batch.is_empty() => {
					defensive!("Not enough weight to migrate a single account");
					return Err(Error::OutOfWeight);
				},
				Err(e) => {
					// if this the last account to handle at this iteration, we skip it next time.
					maybe_last_key = Some(who.clone());
					defensive!("Error while migrating account");
					log::error!(
						target: LOG_TARGET,
						"Error while migrating account: {:?}, error: {:?}",
						who.to_ss58check(),
						e
					);
					continue;
				},
			}
		}

		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(
				batch,
				|batch| types::AhMigratorCall::<T>::ReceiveAccounts { accounts: batch },
				|n| T::AhWeightInfo::receive_liquid_accounts(n),
			)?;
		}

		Ok(maybe_last_key)
	}
}

impl<T: Config> AccountsMigrator<T> {
	/// Migrate a single account out of the Relay chain and return it.
	///
	/// The account on the relay chain is modified as part of this operation.
	pub fn withdraw_account(
		who: T::AccountId,
		account_info: AccountInfoFor<T>,
		ah_weight: &mut WeightMeter,
		batch_len: u32,
	) -> Result<Option<AccountFor<T>>, Error<T>> {
		let account_state = Self::get_account_state(&who);
		if account_state.is_preserve() {
			log::info!(
				target: LOG_TARGET,
				"Preserving account on Relay Chain: '{:?}'",
				who.to_ss58check(),
			);
			return Ok(None);
		}

		log::trace!(
			target: LOG_TARGET,
			"Migrating account '{}'",
			who.to_ss58check(),
		);

		// migrate the target account:
		// - keep `balance`, `holds`, `freezes`, .. in memory
		// - check if there is anything to migrate
		// - release all `holds`, `freezes`, ...
		// - burn from target account the `balance` to be moved from RC to AH
		// - add `balance`, `holds`, `freezes`, .. to the accounts package to be sent via XCM

		let account_data: AccountData<T::Balance> = account_info.data.clone();

		if !Self::can_migrate_account(&who, &account_info) {
			log::info!(target: LOG_TARGET, "Account cannot be migrated '{}'", who.to_ss58check());
			return Ok(None);
		}

		let freezes: Vec<IdAmount<T::FreezeIdentifier, T::Balance>> =
			pallet_balances::Freezes::<T>::get(&who).into();

		for freeze in &freezes {
			if let Err(e) = <T as Config>::Currency::thaw(&freeze.id, &who) {
				log::error!(target: LOG_TARGET,
					"Failed to thaw freeze: {:?} \
					for account: {:?} \
					with error: {:?}",
					freeze.id,
					who.to_ss58check(),
					e
				);
				return Err(Error::FailedToWithdrawAccount);
			}
		}

		let ed = <T as Config>::Currency::minimum_balance();
		let holds: Vec<IdAmount<<T as Config>::RuntimeHoldReason, T::Balance>> =
			pallet_balances::Holds::<T>::get(&who).into();

		for hold in &holds {
			let IdAmount { id, amount } = hold.clone();
			let free = <T as Config>::Currency::balance(&who);

			// When the free balance is below the minimum balance and we attempt to release a hold,
			// the `fungible` implementation would burn the entire free balance while zeroing the
			// hold. To prevent this, we partially release the hold just enough to raise the free
			// balance to the minimum balance, while maintaining some balance on hold. This approach
			// prevents the free balance from being burned.
			// This scenario causes a panic in the test environment - see:
			// https://github.com/paritytech/polkadot-sdk/blob/35e6befc5dd61deb154ff0eb7c180a038e626d66/substrate/frame/balances/src/impl_fungible.rs#L285
			let mut amount = if free < ed && amount.saturating_sub(ed - free) > 0 {
				log::debug!(
					target: LOG_TARGET,
					"Partially releasing hold to prevent the free balance from being burned"
				);
				let partial_amount = ed - free;
				if let Err(e) =
					<T as Config>::Currency::release(&id, &who, partial_amount, Precision::Exact)
				{
					log::error!(target: LOG_TARGET,
						"Failed to partially release hold: {:?} \
						for account: {:?}, \
						partial amount: {:?}, \
						with error: {:?}",
						id,
						who.to_ss58check(),
						partial_amount,
						e
					);
					return Err(Error::FailedToWithdrawAccount);
				}
				amount - partial_amount
			} else {
				amount
			};

			// If the hold amount is greater than the reserved balance (inconsistent state), we just
			// release the entire reserved balance
			let reserved_balance = <T as Config>::Currency::reserved_balance(&who);
			let mut release_precision = Precision::Exact;
			if reserved_balance < amount {
				defensive!(
					"Hold amount for account {:?} is greater than the reserved balance.",
					who.to_ss58check()
				);
				log::debug!(target: LOG_TARGET,
					"Releasing the entire reserved balance on best effort: {:?}",
					reserved_balance,
				);
				amount = reserved_balance;
				release_precision = Precision::BestEffort;
			}
			if let Err(e) = <T as Config>::Currency::release(&id, &who, amount, release_precision) {
				log::error!(target: LOG_TARGET,
					"Failed to release the hold: {:?} \
					for account: {:?}, \
					amount: {:?}, \
					with error: {:?}",
					id,
					who.to_ss58check(),
					amount,
					e
				);
				return Err(Error::FailedToWithdrawAccount);
			}
		}

		let locks: Vec<BalanceLock<T::Balance>> =
			pallet_balances::Locks::<T>::get(&who).into_inner();

		for lock in &locks {
			// Expected lock ids:
			// - "staking " : lazily migrated to holds
			// - "vesting "
			// - "pyconvot"
			<T as Config>::Currency::remove_lock(lock.id, &who);
		}

		// unreserve the unnamed reserve but keep some reserve on RC if needed.
		let unnamed_reserve = <T as Config>::Currency::reserved_balance(&who)
			.checked_sub(account_state.get_rc_reserved())
			.defensive_unwrap_or_default();
		let _ = <T as Config>::Currency::unreserve(&who, unnamed_reserve);

		// ensuring the account can be fully withdrawn from RC to AH requires force-updating
		// the references here. Instead, for accounts meant to be fully migrated to the AH, we will
		// calculate the actual reference counts based on the migrating pallets and transfer the
		// counts to AH. This is done using the `Self::get_consumer_count` and
		// `Self::get_provider_count` functions.
		//
		// check accounts.md for more details.
		SystemAccount::<T>::mutate(&who, |a| {
			a.consumers = account_state.get_rc_consumers();
			// the provider count is set to `1` to allow reaping accounts that provided the ED at
			// the `burn_from` below.
			a.providers = 1;
		});

		let total_balance = <T as Config>::Currency::total_balance(&who);
		let teleport_total = <T as Config>::Currency::reducible_balance(
			&who,
			Preservation::Expendable,
			Fortitude::Polite,
		);

		let teleport_free = account_data
			.free
			.checked_sub(account_state.get_rc_free())
			.defensive_unwrap_or_default();
		let teleport_reserved = account_data
			.reserved
			.checked_sub(account_state.get_rc_reserved())
			.defensive_unwrap_or_default();

		defensive_assert!(
			teleport_total ==
				total_balance - account_state.get_rc_free() - account_state.get_rc_reserved()
		);
		defensive_assert!(teleport_total == teleport_free + teleport_reserved);

		let burned = match <T as Config>::Currency::burn_from(
			&who,
			teleport_total,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite,
		) {
			Ok(burned) => burned,
			Err(e) => {
				log::error!(
					target: LOG_TARGET,
					"Failed to burn balance from account: {}, error: {:?}",
					who.to_ss58check(),
					e
				);
				return Err(Error::FailedToWithdrawAccount);
			},
		};

		debug_assert!(teleport_total == burned);

		Self::update_migrated_balance(&who, teleport_total)?;

		let consumers = Self::get_consumer_count(&who, &account_info);
		let providers = Self::get_provider_count(&who, &account_info, &holds);
		let withdrawn_account = Account {
			who: who.clone(),
			free: teleport_free,
			reserved: teleport_reserved,
			frozen: account_data.frozen,
			holds: BoundedVec::defensive_truncate_from(holds),
			freezes: BoundedVec::defensive_truncate_from(freezes),
			locks: BoundedVec::defensive_truncate_from(locks),
			unnamed_reserve,
			consumers,
			providers,
		};

		// account the weight for receiving a single account on Asset Hub.
		let ah_receive_weight = Self::weight_ah_receive_account(batch_len, &withdrawn_account);
		if ah_weight.try_consume(ah_receive_weight).is_err() {
			log::info!("AH weight limit reached at batch length {}, stopping", batch_len);
			return Err(Error::OutOfWeight);
		}

		Ok(Some(withdrawn_account))
	}

	/// Actions to be done after the accounts migration is finished.
	pub fn finish_balances_migration() {
		pallet_balances::InactiveIssuance::<T>::put(0);
	}

	/// Check if the account can be withdrawn and migrated to AH.
	pub fn can_migrate_account(who: &T::AccountId, account: &AccountInfoFor<T>) -> bool {
		let ed = <T as Config>::Currency::minimum_balance();
		let total_balance = <T as Config>::Currency::total_balance(who);
		if total_balance < ed {
			if account.nonce.is_zero() {
				log::info!(
					target: LOG_TARGET,
					"Possible system non-migratable account detected. \
					Account: '{}', info: {:?}",
					who.to_ss58check(),
					account
				);
			} else {
				log::info!(
					target: LOG_TARGET,
					"Non-migratable account detected. \
					Account: '{}', info: {:?}",
					who.to_ss58check(),
					account
				);
			}
			if !total_balance.is_zero() || !account.data.frozen.is_zero() {
				log::warn!(
					target: LOG_TARGET,
					"Non-migratable account has non-zero balance. \
					Account: '{}', info: {:?}",
					who.to_ss58check(),
					account
				);
			}
			return false;
		}
		true
	}

	/// Get the weight for importing a single account on Asset Hub.
	///
	/// The base weight is only included for the first imported account.
	pub fn weight_ah_receive_account(batch_len: u32, account: &AccountFor<T>) -> Weight {
		let weight_of = if account.is_liquid() {
			T::AhWeightInfo::receive_liquid_accounts
		} else {
			// TODO: use `T::AhWeightInfo::receive_accounts` with xcm v5, where
			// `require_weight_at_most` not required
			T::AhWeightInfo::receive_liquid_accounts
		};
		item_weight_of(weight_of, batch_len)
	}

	/// Consumer ref count of migrating to Asset Hub pallets except a reference for `reserved` and
	/// `frozen` balance.
	///
	/// Since the `reserved` and `frozen` balances will be known on a receiving side (AH) they will
	/// be calculated there.
	///
	/// Check accounts.md for more details.
	pub fn get_consumer_count(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> u8 {
		0
	}

	/// Provider ref count of migrating to Asset Hub pallets except the reference for existential
	/// deposit.
	///
	/// Since the `free` balance will be known on a receiving side (AH) the ref count will be
	/// calculated there.
	///
	/// Check accounts.md for more details.
	pub fn get_provider_count(
		_who: &T::AccountId,
		_info: &AccountInfoFor<T>,
		freezes: &Vec<IdAmount<<T as Config>::RuntimeHoldReason, T::Balance>>,
	) -> u8 {
		if freezes.iter().any(|freeze| freeze.id == T::StakingDelegationReason::get()) {
			// one extra provider for accounts with staking delegation
			1
		} else {
			0
		}
	}

	/// Returns the migration state for the given account.
	///
	/// The state is retrieved from storage if previously set, otherwise defaults to `Migrate`.
	pub fn get_account_state(who: &T::AccountId) -> AccountStateFor<T> {
		if let Some(state) = RcAccounts::<T>::get(who) {
			log::debug!(target: LOG_TARGET, "Account state for '{}': {:?}", who.to_ss58check(), state);
			return state;
		}
		AccountStateFor::<T>::Migrate
	}

	fn update_migrated_balance(
		who: &T::AccountId,
		teleported_balance: T::Balance,
	) -> Result<(), Error<T>> {
		RcMigratedBalance::<T>::mutate(|tracker| {
			tracker.migrated =
				tracker.migrated.checked_add(teleported_balance).ok_or_else(|| {
					log::error!(
						target: LOG_TARGET,
						"Balance overflow when adding balance of {}, balance {:?}, to total migrated {:?}",
						who.to_ss58check(), teleported_balance, tracker.migrated,
					);
					Error::<T>::BalanceOverflow
				})?;
			tracker.kept = tracker.kept.checked_sub(teleported_balance).ok_or_else(|| {
				log::error!(
					target: LOG_TARGET,
					"Balance underflow when subtracting balance of {}, balance {:?}, from total kept {:?}",
					who.to_ss58check(), teleported_balance, tracker.kept,
				);
				Error::<T>::BalanceUnderflow
			})?;
			Ok::<_, Error<T>>(())
		})
	}

	/// Obtain all known accounts that must stay on RC and persist it to the [`RcAccounts`] storage
	/// item.
	///
	/// Should be executed once before the migration starts.
	pub fn obtain_rc_accounts() -> Weight {
		let mut weight = Weight::zero();
		let mut reserves = sp_std::collections::btree_map::BTreeMap::new();
		let mut update_reserves = |id, deposit| {
			if deposit == 0 {
				return;
			}
			reserves
				.entry(id)
				.and_modify(|e: &mut u128| *e = e.saturating_add(deposit))
				.or_insert(deposit);
		};

		for (channel_id, info) in hrmp::HrmpChannels::<T>::iter() {
			weight += T::DbWeight::get().reads(1);
			// source: https://github.com/paritytech/polkadot-sdk/blob/3dc3a11cd68762c2e5feb0beba0b61f448c4fc92/polkadot/runtime/parachains/src/hrmp.rs#L1475
			let sender: T::AccountId = channel_id.sender.into_account_truncating();
			update_reserves(sender, info.sender_deposit);

			let recipient: T::AccountId = channel_id.recipient.into_account_truncating();
			// source: https://github.com/paritytech/polkadot-sdk/blob/3dc3a11cd68762c2e5feb0beba0b61f448c4fc92/polkadot/runtime/parachains/src/hrmp.rs#L1539
			update_reserves(recipient, info.recipient_deposit);
		}

		for (channel_id, info) in hrmp::HrmpOpenChannelRequests::<T>::iter() {
			weight += T::DbWeight::get().reads(1);
			// source: https://github.com/paritytech/polkadot-sdk/blob/3dc3a11cd68762c2e5feb0beba0b61f448c4fc92/polkadot/runtime/parachains/src/hrmp.rs#L1475
			let sender: T::AccountId = channel_id.sender.into_account_truncating();
			update_reserves(sender, info.sender_deposit);
		}

		for (_, info) in Paras::<T>::iter() {
			weight += T::DbWeight::get().reads(1);
			update_reserves(info.manager, info.deposit);
		}

		for (id, rc_reserved) in reserves {
			weight += T::DbWeight::get().reads(4);
			let account_entry = SystemAccount::<T>::get(&id);
			let free = <T as Config>::Currency::balance(&id);
			let total_frozen = account_entry.data.frozen;
			let total_reserved = <T as Config>::Currency::reserved_balance(&id);
			let total_hold = pallet_balances::Holds::<T>::get(&id)
				.into_iter()
				// we do not expect more holds
				.take(5)
				.map(|h| h.amount)
				.sum::<T::Balance>();

			let rc_ed = <T as Config>::Currency::minimum_balance();
			let ah_ed = T::AhExistentialDeposit::get();

			// we prioritize the named holds over the unnamed reserve. If the account to preserve
			// has any named holds, we will send them to the AH and keep up to the unnamed reserves
			// `rc_reserved` on the RC.
			let rc_reserved = rc_reserved.min(total_reserved.saturating_sub(total_hold));
			let ah_free = free.saturating_sub(rc_ed);

			if rc_reserved == 0 {
				log::debug!(
					target: LOG_TARGET,
					"Account doesn't have enough reserved balance to keep on RC. account: {:?}.",
					id.to_ss58check(),
				);
				continue;
			}

			if ah_free < ah_ed && rc_reserved >= total_reserved && total_frozen.is_zero() {
				weight += T::DbWeight::get().writes(1);
				// when there is not enough free balance to migrate to AH and the account is used
				// only for reserves for parachains registering or hrmp channels, we will keep
				// the entire account on the RC.
				log::debug!(
					target: LOG_TARGET,
					"Preserve account on Relay Chain: '{:?}'",
					id.to_ss58check()
				);
				RcAccounts::<T>::insert(&id, AccountState::Preserve);
			} else {
				weight += T::DbWeight::get().writes(1);
				log::debug!(
					target: LOG_TARGET,
					"Keep part of account: {:?} reserve: {:?} on the RC",
					id.to_ss58check(),
					rc_reserved
				);
				RcAccounts::<T>::insert(
					&id,
					// one consumer reference of reserved balance.
					AccountState::Part { free: rc_ed, reserved: rc_reserved, consumers: 1 },
				);
			}
		}

		// Keep the on-demand pallet account on the RC.
		weight += T::DbWeight::get().writes(1);
		let on_demand_pallet_account: T::AccountId =
			T::OnDemandPalletId::get().into_account_truncating();
		log::debug!(
			target: LOG_TARGET,
			"Preserve on-demand pallet account on Relay Chain: '{:?}'",
			on_demand_pallet_account.to_ss58check()
		);
		RcAccounts::<T>::insert(&on_demand_pallet_account, AccountState::Preserve);

		weight
	}

	/// Try to translate a Parachain sovereign account to the Parachain AH sovereign account.
	///
	/// Returns:
	/// - `Ok(None)` if the account is not a Parachain sovereign account
	/// - `Ok(Some((ah_account, para_id)))` with the translated account and the para id
	/// - `Err(())` otherwise
	///
	/// The way that this normally works is through the configured `SiblingParachainConvertsVia`:
	/// https://github.com/polkadot-fellows/runtimes/blob/7b096c14c2b16cc81ca4e2188eea9103f120b7a4/system-parachains/asset-hubs/asset-hub-polkadot/src/xcm_config.rs#L93-L94
	/// it passes the `Sibling` type into it which has type-ID `sibl`:
	/// https://github.com/paritytech/polkadot-sdk/blob/c10e25aaa8b8afd8665b53f0a0b02e4ea44caa77/polkadot/parachain/src/primitives.rs#L272-L274.
	/// This type-ID gets used by the converter here:
	/// https://github.com/paritytech/polkadot-sdk/blob/7ecf3f757a5d6f622309cea7f788e8a547a5dce8/polkadot/xcm/xcm-builder/src/location_conversion.rs#L314
	/// and eventually ends up in the encoding here
	/// https://github.com/paritytech/polkadot-sdk/blob/cdf107de700388a52a17b2fb852c98420c78278e/substrate/primitives/runtime/src/traits/mod.rs#L1997-L1999
	/// The `para` conversion is likewise with `ChildParachainConvertsVia` and the `para` type-ID
	/// https://github.com/paritytech/polkadot-sdk/blob/c10e25aaa8b8afd8665b53f0a0b02e4ea44caa77/polkadot/parachain/src/primitives.rs#L162-L164
	pub fn try_translate_rc_sovereign_to_ah(
		acc: T::AccountId,
	) -> Result<Option<(T::AccountId, u16)>, ()> {
		let raw = acc.to_raw_vec();

		// Must start with "para"
		let Some(raw) = raw.strip_prefix(b"para") else {
			return Ok(None);
		};
		// Must end with 26 zero bytes
		let Some(raw) = raw.strip_suffix(&[0u8; 26]) else {
			return Ok(None);
		};
		let para_id = u16::decode_all(&mut &raw[..]).map_err(|_| ())?;

		// Translate to AH sibling account
		let mut ah_raw = [0u8; 32];
		ah_raw[0..4].copy_from_slice(b"sibl");
		ah_raw[4..6].copy_from_slice(&para_id.encode());
		let ah_acc = ah_raw.try_into().map_err(|_| ()).defensive()?;

		Ok(Some((ah_acc, para_id)))
	}
}

// Only used for testing.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum HoldReason {
	Preimage,
	Staking,
}

// Only used for testing.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FreezeReason {
	NominationPools,
}

// Only used for testing.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LockReason {
	Staking,
	Vesting,
	ConvictionVoting,
}

// Balance summary of an account of the Relay chain. Only used for testing.
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub struct BalanceSummary {
	// Balance that can be still reserved
	pub migrated_free: u128,
	// Holds + Named Reserves (should be 0) + Unnamed Reserves
	pub migrated_reserved: u128,
	// Locks + Freezes
	pub frozen: u128,
	// Each hold: (lock id enum as u8, amount)
	pub holds: Vec<(HoldReason, u128)>,
	// Each freeze: (freeze id enum as u8, amount).
	pub freezes: Vec<(FreezeReason, u128)>,
	// Each lock: (lock id enum as u8, amount, reasons as u8)
	pub locks: Vec<(LockReason, u128, u8)>,
}

pub enum ChainType {
	RC,
	AH,
}

// For each account that is fully or partially kept on the relay chain, this structure contains
// rc_kept_reserved_balance, rc_kept_free_balance, i.e., the balance that is kept on the relay
// chain for the given account, split between reserved and free in separate maps. In general,
// the free balance is equal to the existential deposit, but there may be some edge cases (e.g.,
// on-demand pallet account or accounts with inconsistent state).
#[cfg(feature = "std")]
pub struct RcKeptBalance<T: Config> {
	pub rc_reserved_kept: BTreeMap<T::AccountId, u128>,
	pub rc_free_kept: BTreeMap<T::AccountId, u128>,
}

#[cfg(feature = "std")]
impl<T: Config> RcKeptBalance<T> {
	pub fn new() -> Self {
		let mut rc_reserved_kept = BTreeMap::new();
		let mut rc_free_kept = BTreeMap::new();
		let mut rc_reserves = BTreeMap::new();

		// On-demand pallet account is not migrated to Asset Hub
		let on_demand_pallet_account = T::OnDemandPalletId::get().into_account_truncating();
		let total_reserved = <T as Config>::Currency::reserved_balance(&on_demand_pallet_account);
		let free = <T as Config>::Currency::balance(&on_demand_pallet_account);
		rc_reserved_kept.insert(on_demand_pallet_account.clone(), total_reserved);
		rc_free_kept.insert(on_demand_pallet_account.clone(), free);

		for (channel_id, info) in hrmp::HrmpChannels::<T>::iter() {
			let sender: T::AccountId = channel_id.sender.into_account_truncating();
			let sender_deposit = info.sender_deposit;
			if sender_deposit > 0 {
				rc_reserves
					.entry(sender.clone())
					.and_modify(|r: &mut u128| *r = r.saturating_add(sender_deposit))
					.or_insert(sender_deposit);
			}
			let recipient: T::AccountId = channel_id.recipient.into_account_truncating();
			let recipient_deposit = info.recipient_deposit;
			if recipient_deposit > 0 {
				rc_reserves
					.entry(recipient.clone())
					.and_modify(|r: &mut u128| *r = r.saturating_add(recipient_deposit))
					.or_insert(recipient_deposit);
			}
		}

		for (channel_id, info) in hrmp::HrmpOpenChannelRequests::<T>::iter() {
			let sender: T::AccountId = channel_id.sender.into_account_truncating();
			let sender_deposit = info.sender_deposit;
			if sender_deposit > 0 {
				rc_reserves
					.entry(sender.clone())
					.and_modify(|r: &mut u128| *r = r.saturating_add(sender_deposit))
					.or_insert(sender_deposit);
			}
		}

		for (_, info) in Paras::<T>::iter() {
			let manager = info.manager;
			let manager_deposit = info.deposit;
			if manager_deposit > 0 {
				rc_reserves
					.entry(manager.clone())
					.and_modify(|r: &mut u128| *r = r.saturating_add(manager_deposit))
					.or_insert(manager_deposit);
			}
		}

		for (who, mut reserved_kept) in rc_reserves {
			// Holds migration is prioritized over keeping unnamed reserves on the relay chain
			let total_reserved = <T as Config>::Currency::reserved_balance(&who);
			let total_hold = pallet_balances::Holds::<T>::get(&who)
				.into_iter()
				.map(|h| h.amount)
				.sum::<T::Balance>();
			reserved_kept = reserved_kept.min(total_reserved.saturating_sub(total_hold));
			if reserved_kept == 0 {
				continue;
			}

			// We need to keep some reserved balance (reserved_kept) on the relay chain,
			// together with the Relay Chain existential deposit. Notice that this cannot be
			// greater than the total balance of the account.
			let ed = <T as Config>::Currency::minimum_balance();
			let total_balance = <T as Config>::Currency::total_balance(&who);
			let free_kept = ed.min(total_balance.saturating_sub(reserved_kept));
			rc_reserved_kept.insert(who.clone(), reserved_kept);
			rc_free_kept.insert(who.clone(), free_kept);
		}
		Self { rc_reserved_kept, rc_free_kept }
	}
}

pub struct AccountsMigrationChecker<T>(sp_std::marker::PhantomData<T>);

#[cfg(feature = "std")]
impl<T> AccountsMigrationChecker<T> {
	// Translate the lock id to enum for both RC and AH using the different encodings for easier
	// comparison. Lock Ids type is  `LockIdentifier = [u8; 8]` on Polkadot.
	pub fn lock_id_encoding(lock_id: [u8; 8], _chain_type: ChainType) -> LockReason {
		match lock_id.as_slice() {
			b"staking " => LockReason::Staking,
			b"vesting " => LockReason::Vesting,
			b"pyconvot" => LockReason::ConvictionVoting,
			_ => panic!("Unknown lock id: {:?}", lock_id),
		}
	}

	// Translate the freeze id to enum for both RC and AH using the different encodings for easier
	// comparison. Freeze Ids type is enum `RuntimeFreezeReason` on Polkadot. Here we use the
	// encoded enum.
	pub fn freeze_id_encoding(freeze_id: Vec<u8>, chain_type: ChainType) -> FreezeReason {
		let nomination_pools_encoding = match chain_type {
			ChainType::RC => [39, 0], // 39 = nom pools pallet index on Polkadot
			ChainType::AH => [80, 0], // 80 = nom pools pallet index on Asset Hub
		};
		if freeze_id.as_slice() == nomination_pools_encoding {
			FreezeReason::NominationPools
		} else {
			panic!("Unknown freeze id: {:?}", freeze_id);
		}
	}

	// Translate the hold id to enum for both RC and AH using the different encodings for easier
	// comparison. Hold Ids type is enum `RuntimeHoldReason` on Polkadot. Here we use the encoded
	// enum.
	pub fn hold_id_encoding(hold_id: Vec<u8>, chain_type: ChainType) -> HoldReason {
		let preimage_encoding: [u8; 2] = match chain_type {
			ChainType::RC => [10, 0], // 10 = preimage pallet index on Polkadot
			ChainType::AH => [5, 0],  // 5 = preimage pallet index on Asset Hub
		};
		let staking_encoding: [u8; 2] = match chain_type {
			ChainType::RC => [41, 0], // 41 = staking pallet index on Polkadot
			// TODO: change to the correct encoding when Staking holds are correctly re-created on
			// Asset Hub in pallet-staking-async
			ChainType::AH => [5, 0], // ? = staking pallet index on Asset Hub
		};
		if hold_id.as_slice() == preimage_encoding {
			HoldReason::Preimage
		} else if hold_id.as_slice() == staking_encoding {
			// TODO: change to HoldReason::Staking when Staking holds are correctly re-created on
			// Asset Hub in pallet-staking-async
			HoldReason::Preimage
		} else {
			panic!("Unknown hold id: {:?}", hold_id);
		}
	}

	// Get the AH expected hold amount for a RC migrated hold.
	// This is used to check that the hold amount is correct after migration.
	pub fn ah_hold_amount_from_rc(hold_id: Vec<u8>, hold_amount: u128) -> u128 {
		match hold_id.as_slice() {
			// Preimage deposits are divided by 100 when migrated to Asset Hub.
			[10, 0] => hold_amount.saturating_div(100),
			// TODO: change to correct amounts for Staking if we decide to adjust deposits during
			// migration. Also use Self::hold_id_encoding to get the correct hold reason when the
			// function is fixed.
			_ => hold_amount,
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for AccountsMigrationChecker<T> {
	// The first item is a mapping from account to a summary of their balances, including holds,
	// reserves, locks, and freezes. The second item is the total issuance on the relay chain
	// before migration
	type RcPrePayload = (BTreeMap<T::AccountId, BalanceSummary>, u128);

	fn pre_check() -> Self::RcPrePayload {
		let mut account_summaries = BTreeMap::new();
		let total_issuance = <T as Config>::Currency::total_issuance();
		let RcKeptBalance { rc_reserved_kept, rc_free_kept } = RcKeptBalance::<T>::new();
		for (who, account_info) in SystemAccount::<T>::iter() {
			// Checking account balance migration is tested separately.
			if who == T::CheckingAccount::get() {
				continue;
			}
			let rc_kept_reserved_balance = rc_reserved_kept.get(&who).unwrap_or(&0);
			let rc_kept_free_balance = rc_free_kept.get(&who).unwrap_or(&0);
			let total_balance = <T as Config>::Currency::total_balance(&who);
			if total_balance == rc_kept_free_balance.saturating_add(*rc_kept_reserved_balance) {
				// Account is fully kept on the relay chain
				continue;
			}
			let total_reserved = <T as Config>::Currency::reserved_balance(&who);
			let migrated_reserved = total_reserved.saturating_sub(*rc_kept_reserved_balance);
			let free = <T as Config>::Currency::balance(&who);
			let migrated_free = free.saturating_sub(*rc_kept_free_balance);
			let frozen = account_info.data.frozen;

			let mut locks_enc = Vec::new();
			for lock in pallet_balances::Locks::<T>::get(&who) {
				locks_enc.push((
					Self::lock_id_encoding(lock.id, ChainType::RC),
					lock.amount,
					lock.reasons as u8,
				));
			}
			let mut freezes_enc = Vec::new();
			for freeze in pallet_balances::Freezes::<T>::get(&who) {
				freezes_enc.push((
					Self::freeze_id_encoding(freeze.id.encode(), ChainType::RC),
					freeze.amount,
				));
			}
			let mut holds_enc = Vec::new();
			for hold in pallet_balances::Holds::<T>::get(&who) {
				holds_enc.push((
					Self::hold_id_encoding(hold.id.encode(), ChainType::RC),
					Self::ah_hold_amount_from_rc(hold.id.encode(), hold.amount),
				));
			}

			let balance_summary = BalanceSummary {
				migrated_free,
				migrated_reserved,
				frozen,
				holds: holds_enc,
				locks: locks_enc,
				freezes: freezes_enc,
			};

			account_summaries.insert(who.clone(), balance_summary);
		}
		(account_summaries, total_issuance)
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload) {
		let (_, rc_total_issuance_before) = rc_pre_payload;

		let mut acc_state_maybe: Option<AccountStateFor<T>>;
		// Check that all accounts have been processed correctly
		for (who, _) in SystemAccount::<T>::iter() {
			acc_state_maybe = RcAccounts::<T>::get(who.clone());
			if acc_state_maybe.is_none() {
				let ed = <T as Config>::Currency::minimum_balance();
				let total_balance = <T as Config>::Currency::total_balance(&who);
				if total_balance < ed {
					acc_state_maybe = Some(AccountState::Preserve);
				}
			}
			match acc_state_maybe {
				Some(AccountState::Part { free, reserved, consumers }) => {
					assert_eq!(
						<T as Config>::Currency::reserved_balance(&who), reserved,
						"Incorrect reserve balance on the Relay Chain after the migration for account: {:?}, {:?}",
						who.to_ss58check(), reserved
					);
					assert_eq!(
						<T as Config>::Currency::balance(&who), free,
						"Incorrect free balance on the Relay Chain after the migration for account: {:?}, {:?}",
						who.to_ss58check(), free
					);
					assert_eq!(
						frame_system::Pallet::<T>::consumers(&who), consumers,
						"Incorrect consumer count on the Relay Chain after the migration for account: {:?}, {:?}",
						who.to_ss58check(), consumers
					);

					// Assert storage "Balances::Locks::rc_post::empty"
					let locks = pallet_balances::Locks::<T>::get(&who);
					assert!(
						locks.is_empty(),
						"Account {:?} should have no locks on the relay chain after migration",
						who.to_ss58check()
					);

					// Assert storage "Balances::Holds::rc_post::empty"
					let holds = pallet_balances::Holds::<T>::get(&who);
					assert!(
						holds.is_empty(),
						"Account {:?} should have no holds on the relay chain after migration",
						who.to_ss58check()
					);

					// Assert storage "Balances::Freezes::rc_post::empty"
					let freezes = pallet_balances::Freezes::<T>::get(&who);
					assert!(
						freezes.is_empty(),
						"Account {:?} should have no freezes on the relay chain after migration",
						who.to_ss58check()
					);
				},
				Some(AccountState::Preserve) => {
					// If the total balance is smaller than the existential deposit, we don't need
					// to check anything else because this is a sufficient reason to preserve
					// the account on the relay chain. If the total balance is greater or equal to
					// the existential deposit, we need to check that the account has no Holds,
					// Freezes, or Locks, and no free balance to be migrated to Asset Hub.
					let total_balance = <T as Config>::Currency::total_balance(&who);
					let ed = <T as Config>::Currency::minimum_balance();
					if total_balance >= ed {
						// Preserved accounts should have no Holds, Freezes, or Locks.
						let holds = pallet_balances::Holds::<T>::get(&who);
						assert!(
							holds.is_empty(),
							"Preserved account {:?} should have no holds on the relay chain after migration",
							who.to_ss58check()
						);

						let freezes = pallet_balances::Freezes::<T>::get(&who);
						assert!(
							freezes.is_empty(),
							"Preserved account {:?} should have no freezes on the relay chain after migration",
							who.to_ss58check()
						);

						let locks = pallet_balances::Locks::<T>::get(&who);
						assert!(
							locks.is_empty(),
							"Preserved account {:?} should have no locks on the relay chain after migration",
							who.to_ss58check()
						);

						// Preserved accounts should not have enough free balance to be partly
						// migrated to Asset Hub.
						let free_balance = <T as Config>::Currency::reducible_balance(
							&who,
							Preservation::Expendable,
							Fortitude::Polite,
						);
						assert!(free_balance < <T as Config>::Currency::minimum_balance().saturating_add(T::AhExistentialDeposit::get()), "Preserved account {:?} should have not enough free balance on the relay chain after migration to be migrated to Asset Hub", who.to_ss58check());
					}
				},
				// This corresponds to AccountState::Migrate: the account should be fully migrated
				// to Asset Hub.
				_ => {
					// Assert storage "Balances::Account::rc_post::empty"
					let total_balance = <T as Config>::Currency::total_balance(&who);
					assert_eq!(
						total_balance,
						0,
						"Account {:?} should have no balance on the relay chain after migration",
						who.to_ss58check()
					);

					// Assert storage "Balances::Locks::rc_post::empty"
					let locks = pallet_balances::Locks::<T>::get(&who);
					assert!(
						locks.is_empty(),
						"Account {:?} should have no locks on the relay chain after migration",
						who.to_ss58check()
					);

					// Assert storage "Balances::Holds::rc_post::empty"
					let holds = pallet_balances::Holds::<T>::get(&who);
					assert!(
						holds.is_empty(),
						"Account {:?} should have no holds on the relay chain after migration",
						who.to_ss58check()
					);

					// Assert storage "Balances::Freezes::rc_post::empty"
					let freezes = pallet_balances::Freezes::<T>::get(&who);
					assert!(
						freezes.is_empty(),
						"Account {:?} should have no freezes on the relay chain after migration",
						who.to_ss58check()
					);

					// Assert storage "Balances::Reserves::rc_post::empty"
					let reserved = <T as Config>::Currency::reserved_balance(&who);
					assert_eq!(
						reserved,
						0,
						"Account {:?} should have no reserves on the relay chain after migration",
						who.to_ss58check()
					);
				},
			}
		}

		let total_issuance = <T as Config>::Currency::total_issuance();
		let tracker = RcMigratedBalance::<T>::get();
		// verify total issuance hasn't changed for any other reason than the migrated funds
		assert_eq!(total_issuance, rc_total_issuance_before.saturating_sub(tracker.migrated));
		assert_eq!(total_issuance, tracker.kept);
	}
}
