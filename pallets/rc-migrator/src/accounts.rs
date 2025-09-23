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
use frame_support::{
	traits::tokens::{Balance as BalanceT, IdAmount},
	weights::WeightMeter,
};
use frame_system::Account as SystemAccount;
use pallet_balances::{AccountData, BalanceLock};
use sp_runtime::{traits::Zero, BoundedVec};

/// Account type meant to transfer data between RC and AH.
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
	///   but the conversion was lazy, so there may be some staking locks left
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

impl<AccountId, Balance: Zero, HoldReason, FreezeReason>
	Account<AccountId, Balance, HoldReason, FreezeReason>
{
	pub fn translate_account(
		self,
		f: impl Fn(AccountId) -> AccountId,
	) -> Account<AccountId, Balance, HoldReason, FreezeReason> {
		Account { who: f(self.who), ..self }
	}
}

/// The state for the Relay Chain accounts.
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
		/// The reserved balance that should be must be preserved on RC.
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
	PortableHoldReason,
	PortableFreezeReason,
>;

/// Helper struct tracking total balance kept on RC and total migrated.
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
pub struct MigratedBalances<Balance> {
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
		let mut total_items_iterated = 0;
		loop {
			// account the weight for migrating a single account on Relay Chain.
			if weight_counter.try_consume(T::RcWeightInfo::withdraw_account()).is_err() ||
				weight_counter.try_consume(batch.consume_weight()).is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() && total_items_iterated == 0 {
					defensive!("Not enough weight to migrate a single account");
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if batch.len() > MAX_ITEMS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					batch.len()
				);
				break;
			}

			if batch.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					batch.batch_count()
				);
				break;
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
						batch.len(),
					) {
						Ok(ok) => TransactionOutcome::Commit(Ok(ok)),
						Err(e) => TransactionOutcome::Rollback(Err(e)),
					}
				})
				.expect("Always returning Ok; qed");

			total_items_iterated += 1;

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
				Err(Error::OutOfWeight) if total_items_iterated > 1 => {
					break;
				},
				// Not enough weight and was unable to make progress, bad.
				Err(Error::OutOfWeight) if total_items_iterated <= 1 => {
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
			Pallet::<T>::send_chunked_xcm_and_track(batch, |batch| {
				types::AhMigratorCall::<T>::ReceiveAccounts { accounts: batch }
			})?;
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

		let freezes: Vec<IdAmount<<T as pallet::Config>::RuntimeFreezeReason, T::Balance>> =
			pallet_balances::Freezes::<T>::get(&who).into_inner();

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

		let rc_ed = <T as Config>::Currency::minimum_balance();
		let ah_ed = T::AhExistentialDeposit::get();
		let holds: Vec<IdAmount<<T as pallet_balances::Config>::RuntimeHoldReason, T::Balance>> =
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
			let amount = if free < rc_ed && amount.saturating_sub(rc_ed - free) > 0 {
				log::debug!(
					target: LOG_TARGET,
					"Partially releasing hold to prevent the free balance from being burned"
				);
				let partial_amount = rc_ed - free;
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

			if <T as Config>::Currency::release(&id, &who, amount, Precision::Exact).is_err() {
				defensive!(
					"There is not enough reserved balance to release the hold for (account, hold id, amount) {:?}",
					(who.to_ss58check(), id.clone(), amount)
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
			if a.data.free < rc_ed && a.data.free >= ah_ed {
				// this account has a broken ED invariant. withdrawing the entire free balance will
				// not decrease the provider count and remove the account from storage. by setting
				// providers to `0`, we ensure the account is properly removed from storage.
				a.providers = 0;
			} else {
				// the provider count is set to `1` to allow reaping accounts that provided the ED
				// at the `burn_from` below.
				a.providers = 1;
			}
		});

		let total_balance = <T as Config>::Currency::total_balance(&who);
		let teleport_total = <T as Config>::Currency::reducible_balance(
			&who,
			Preservation::Expendable,
			Fortitude::Polite,
		);
		let teleport_reserved = account_data
			.reserved
			.checked_sub(account_state.get_rc_reserved())
			.defensive_unwrap_or_default();
		let teleport_free = account_data
			.free
			.checked_sub(account_state.get_rc_free())
			.defensive_unwrap_or_default();

		// This is common for many accounts.
		// The RC migration of nomination pools to delegated-staking holds in the past caused
		// many accounts to have zero free balance or just less the RC existential deposit free
		// balance.
		if teleport_free < ah_ed {
			log::warn!(
				target: LOG_TARGET,
				"Migrated account {:?} has teleported free balance < AH existential deposit: {:?} < {:?}",
				who.to_ss58check(),
				teleport_free,
				ah_ed
			);
		}
		defensive_assert!(
			teleport_total ==
				total_balance - account_state.get_rc_free() - account_state.get_rc_reserved()
		);
		defensive_assert!(
			teleport_total == teleport_free + teleport_reserved,
			"teleport_total == teleport_free + teleport_reserved"
		);

		if teleport_total.is_zero() {
			log::info!(
				target: LOG_TARGET,
				"Nothing to migrate for account: {:?}; state: {:?}",
				who.to_ss58check(),
				account_state,
			);
			return Ok(None);
		}

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
		let portable_holds = holds.into_iter().map(IntoPortable::into_portable).collect();
		let portable_freezes = freezes.into_iter().map(IntoPortable::into_portable).collect();

		let withdrawn_account = AccountFor::<T> {
			who: who.clone(),
			free: teleport_free,
			reserved: teleport_reserved,
			frozen: account_data.frozen,
			holds: BoundedVec::defensive_truncate_from(portable_holds),
			freezes: BoundedVec::defensive_truncate_from(portable_freezes),
			locks: BoundedVec::defensive_truncate_from(locks),
			unnamed_reserve,
			consumers,
			providers,
		};

		// account the weight for receiving a single account on Asset Hub.
		let ah_receive_weight = Self::weight_ah_receive_account(batch_len, &withdrawn_account);
		if ah_weight.try_consume(ah_receive_weight).is_err() {
			log::info!("AH weight limit reached at batch length {batch_len}, stopping");
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
		let ah_ed = T::AhExistentialDeposit::get();
		let total_balance = <T as Config>::Currency::total_balance(who);
		if total_balance < ed {
			if account.data.free >= ah_ed &&
				account.data.reserved.is_zero() &&
				account.data.frozen.is_zero()
			{
				log::info!(
					target: LOG_TARGET,
					"Account has no RC ED, but has enough free balance for AH RC. \
					Account: '{}', info: {:?}",
					who.to_ss58check(),
					account
				);
				return true;
			}
			if !total_balance.is_zero() {
				log::warn!(
					target: LOG_TARGET,
					"Non-migratable account has non-zero balance. \
					Account: '{}', info: {:?}",
					who.to_ss58check(),
					account
				);
			} else {
				log::info!(
					target: LOG_TARGET,
					"Possible system non-migratable account detected. \
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
			T::AhWeightInfo::receive_accounts
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

	/// Populate the `PureProxyCandidatesMigrated` storage item. Return the number of accounts and
	/// weight.
	pub fn obtain_free_proxy_candidates() -> (Option<u32>, Weight) {
		if PureProxyCandidatesMigrated::<T>::iter_keys().next().is_some() {
			// Not using defensive here since that would fail on idempotency check.
			log::info!(target: LOG_TARGET, "Init pure proxy candidates already ran, skipping");
			return (None, T::DbWeight::get().reads(1));
		}

		let mut num_accounts = 0;
		let mut weight = Weight::zero();

		for acc in pallet_proxy::Proxies::<T>::iter_keys() {
			weight += T::DbWeight::get().reads(1);

			if frame_system::Pallet::<T>::account_nonce(&acc).is_zero() {
				PureProxyCandidatesMigrated::<T>::insert(&acc, false);
				num_accounts += 1;
			}
		}

		weight += T::DbWeight::get().reads(1); // +1 for checking whether the iterator is empty
		(Some(num_accounts), weight)
	}

	/// Obtain all known accounts that must stay on RC and persist it to the [`RcAccounts`] storage
	/// item.
	///
	/// Should be executed once before the migration starts.
	pub fn obtain_rc_accounts() -> Weight {
		if RcAccounts::<T>::iter_keys().next().is_some() {
			defensive!("Init accounts migration already ran, skipping");
			return T::DbWeight::get().reads(1);
		}

		let mut weight = Weight::zero();
		let mut reserves = sp_std::collections::btree_map::BTreeMap::new();
		let mut update_reserves = |id, deposit| {
			if deposit == 0 {
				return;
			}
			reserves.entry(id).and_modify(|e| *e += deposit).or_insert(deposit);
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

		for (id, expected_rc_reserved) in reserves {
			weight += T::DbWeight::get().reads_writes(6, 1);
			let free = <T as Config>::Currency::balance(&id);
			let total_reserved = <T as Config>::Currency::reserved_balance(&id);
			let total_hold = pallet_balances::Holds::<T>::get(&id)
				.into_iter()
				// we do not expect more holds
				.take(5)
				.map(|h| h.amount)
				.sum::<T::Balance>();

			let rc_ed = <T as Config>::Currency::minimum_balance();
			let ah_ed = T::AhExistentialDeposit::get();

			defensive_assert!(total_reserved >= total_hold, "total_reserved >= total_hold");

			// We need to keep rc_ed free balance on the relay chain and migrate at least ah_ed free
			// balance to the asset hub.
			let missing_free = (rc_ed + ah_ed).saturating_sub(free);
			// we prioritize the named holds over the unnamed reserve. If the account to preserve
			// has any named holds, we will send them to the AH and keep up to the unnamed reserves
			// `rc_reserved` on the RC.
			let actual_rc_reserved = (expected_rc_reserved
				.min(total_reserved.saturating_sub(total_hold)))
			.saturating_sub(missing_free);

			if actual_rc_reserved == 0 {
				log::debug!(
					target: LOG_TARGET,
					"Account doesn't have enough reserved balance to keep on RC. account: {:?}.",
					id.to_ss58check(),
				);
				continue;
			}

			if missing_free == 0 {
				RcAccounts::<T>::insert(
					&id,
					// one consumer reference of reserved balance.
					AccountState::Part { free: rc_ed, reserved: actual_rc_reserved, consumers: 1 },
				);
			} else {
				log::warn!(
					target: LOG_TARGET,
					"Account {:?} has less free balance {} than the existential deposits {} + {} (RC ed + AH ed)",
					id.to_ss58check(),
					free,
					rc_ed,
					ah_ed
				);

				let failed = <T as Config>::Currency::unreserve(&id, missing_free);
				defensive_assert!(failed == 0, "failed to unreserve");

				let new_free = <T as Config>::Currency::balance(&id);
				if new_free < rc_ed + ah_ed {
					log::error!(
						target: LOG_TARGET,
						"We could not unreserve enough balance on the RC for RC and AH existential deposits for partially migrated account {:?}",
						id.to_ss58check()
					)
				}

				RcAccounts::<T>::insert(
					&id,
					// one consumer reference of reserved balance.
					AccountState::Part { free: rc_ed, reserved: actual_rc_reserved, consumers: 1 },
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
}

// Only used for testing.
#[cfg(feature = "std")]
pub mod tests {
	use super::*;
	use std::collections::BTreeMap;

	// Balance summary of an account of the Relay chain.
	#[derive(Default, Clone, PartialEq, Eq, Debug)]
	pub struct BalanceSummary {
		// Balance that can be still reserved
		pub migrated_free: u128,
		// Holds + Named Reserves (should be 0) + Unnamed Reserves
		pub migrated_reserved: u128,
		// Locks + Freezes
		pub frozen: u128,
		// Each hold: (hold id encoded, amount)
		pub holds: Vec<(Vec<u8>, u128)>,
		// Each freeze: (freeze id encoded, amount).
		pub freezes: Vec<(Vec<u8>, u128)>,
		// Each lock: (lock id, amount, reasons as u8)
		pub locks: Vec<([u8; 8], u128, u8)>,
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
		pub fn build() -> Self {
			let mut rc_reserved_kept = BTreeMap::new();
			let mut rc_free_kept = BTreeMap::new();
			let mut rc_reserves = BTreeMap::new();

			// On-demand pallet account is not migrated to Asset Hub
			let on_demand_pallet_account = T::OnDemandPalletId::get().into_account_truncating();
			let total_reserved =
				<T as Config>::Currency::reserved_balance(&on_demand_pallet_account);
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

				let rc_ed = <T as Config>::Currency::minimum_balance();
				let ah_ed = T::AhExistentialDeposit::get();
				let free = <T as Config>::Currency::balance(&who);
				// We always need rc_ed free balance on the relay chain and migrate at least ah_ed
				// free balance to the asset hub.
				if free < rc_ed + ah_ed {
					reserved_kept = reserved_kept.saturating_sub(rc_ed + ah_ed - free);
				}
				rc_reserved_kept.insert(who.clone(), reserved_kept);
				rc_free_kept.insert(who.clone(), rc_ed);
			}
			Self { rc_reserved_kept, rc_free_kept }
		}
	}

	pub struct AccountsMigrationChecker<T>(sp_std::marker::PhantomData<T>);

	#[cfg(not(feature = "kusama-ahm"))]
	impl<T> AccountsMigrationChecker<T> {
		// Translate the RC freeze id encoding to the corresponding AH freeze id encoding.
		pub fn rc_freeze_id_encoding_to_ah(freeze_id: Vec<u8>) -> Vec<u8> {
			match freeze_id.as_slice() {
				// Nomination pools pallet indexes on Polkadot RC => AH
				[39, 0] => [80, 0].to_vec(),
				_ => panic!("Unknown freeze id: {freeze_id:?}"),
			}
		}

		// Translate the RC hold id encoding to the corresponding AH hold id encoding.
		pub fn rc_hold_id_encoding_to_ah(hold_id: Vec<u8>) -> Vec<u8> {
			match hold_id.as_slice() {
				// Preimage pallet indexes on Polkadot RC => AH
				[10, 0] => [5, 0].to_vec(),
				// Pallet staking indexes on Polkadot RC => AH
				[7, 0] => [89, 0].to_vec(),
				// Pallet delegated-staking indexes on Polkadot RC => AH
				[41, 0] => [83, 0].to_vec(),
				_ => panic!("Unknown hold id: {hold_id:?}"),
			}
		}

		// Get the AH expected hold amount for a RC migrated hold.
		// This is used to check that the hold amount is correct after migration.
		pub fn ah_hold_amount_from_rc(hold_id: Vec<u8>, hold_amount: u128) -> u128 {
			match hold_id.as_slice() {
				// Preimage deposits are divided by 100 when migrated to Asset Hub.
				[10, 0] => hold_amount.saturating_div(100),
				// TODO: change to correct amounts for Staking if we decide to adjust deposits
				// during migration.
				_ => hold_amount,
			}
		}
	}

	#[cfg(feature = "kusama-ahm")]
	impl<T> AccountsMigrationChecker<T> {
		// Translate the RC freeze id encoding to the corresponding AH freeze id encoding.
		pub fn rc_freeze_id_encoding_to_ah(freeze_id: Vec<u8>) -> Vec<u8> {
			match freeze_id.as_slice() {
				// Nomination pools pallet indexes on Kusama RC => AH
				[41, 0] => [80, 0].to_vec(),
				_ => panic!("Unknown freeze id: {freeze_id:?}"),
			}
		}
		// Translate the RC hold id encoding to the corresponding AH hold id encoding.
		pub fn rc_hold_id_encoding_to_ah(hold_id: Vec<u8>) -> Vec<u8> {
			match hold_id.as_slice() {
				// Preimage pallet indexes on Kusama RC => AH
				[32, 0] => [6, 0].to_vec(),
				// Pallet staking indexes on Kusama RC => AH
				[6, 0] => [89, 0].to_vec(),
				// Pallet delegated-staking indexes on Kusama RC => AH
				[47, 0] => [83, 0].to_vec(),
				_ => panic!("Unknown hold id: {hold_id:?}"),
			}
		}

		// Get the AH expected hold amount for a RC migrated hold.
		// This is used to check that the hold amount is correct after migration.
		pub fn ah_hold_amount_from_rc(hold_id: Vec<u8>, hold_amount: u128) -> u128 {
			match hold_id.as_slice() {
				// Preimage deposits are divided by 100 when migrated to Asset Hub.
				[32, 0] => hold_amount.saturating_div(100),
				// TODO: change to correct amounts for Staking if we decide to adjust deposits
				// during migration.
				_ => hold_amount,
			}
		}
	}

	impl<T: Config> crate::types::RcMigrationCheck for AccountsMigrationChecker<T> {
		// The first item is a mapping from account to a summary of their balances, including holds,
		// reserves, locks, and freezes. The second item is the total issuance on the relay chain
		// before migration
		type RcPrePayload = (BTreeMap<T::AccountId, tests::BalanceSummary>, u128);

		fn pre_check() -> Self::RcPrePayload {
			let mut account_summaries = BTreeMap::new();
			let total_issuance = <T as Config>::Currency::total_issuance();
			let tests::RcKeptBalance { rc_reserved_kept, rc_free_kept } =
				tests::RcKeptBalance::<T>::build();
			for (who, _) in SystemAccount::<T>::iter() {
				// Checking account balance migration is tested separately.
				if who == T::CheckingAccount::get() {
					continue;
				}
				let total_balance = <T as Config>::Currency::total_balance(&who);
				let rc_ed = <T as Config>::Currency::minimum_balance();
				// Such accounts are not migrated to Asset Hub.
				if total_balance < rc_ed {
					continue;
				}
				let rc_kept_reserved_balance = rc_reserved_kept.get(&who).unwrap_or(&0);
				let rc_kept_free_balance = rc_free_kept.get(&who).unwrap_or(&0);
				if total_balance == rc_kept_free_balance.saturating_add(*rc_kept_reserved_balance) {
					// Account is fully kept on the relay chain
					continue;
				}
				let total_reserved = <T as Config>::Currency::reserved_balance(&who);
				let free = <T as Config>::Currency::balance(&who);
				// Extra balance that needs to be freed for migration for existential deposits.
				let mut freed_for_migration = 0;
				let ah_ed = T::AhExistentialDeposit::get();
				let tot_kept_balance =
					rc_kept_reserved_balance.saturating_add(*rc_kept_free_balance);
				if tot_kept_balance > 0 && tot_kept_balance < total_balance && free < rc_ed + ah_ed
				{
					freed_for_migration = rc_ed + ah_ed - free;
				}

				let migrated_free =
					free.saturating_add(freed_for_migration).saturating_sub(*rc_kept_free_balance);
				let migrated_reserved = total_reserved
					.saturating_sub(freed_for_migration)
					.saturating_sub(*rc_kept_reserved_balance);

				let mut frozen = 0;

				let mut locks_enc = Vec::new();
				for lock in pallet_balances::Locks::<T>::get(&who) {
					locks_enc.push((lock.id, lock.amount, lock.reasons as u8));
					frozen += lock.amount;
				}
				let mut freezes_enc = Vec::new();
				for freeze in pallet_balances::Freezes::<T>::get(&who) {
					freezes_enc.push((freeze.id.encode(), freeze.amount));
					frozen += freeze.amount;
				}
				let mut holds_enc = Vec::new();
				for hold in pallet_balances::Holds::<T>::get(&who) {
					holds_enc.push((
						hold.id.encode(),
						Self::ah_hold_amount_from_rc(hold.id.encode(), hold.amount),
					));
				}

				let balance_summary = tests::BalanceSummary {
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

			let mut account_state_maybe: Option<AccountStateFor<T>>;
			// Check that all accounts have been processed correctly
			for (who, _) in SystemAccount::<T>::iter() {
				account_state_maybe = RcAccounts::<T>::get(who.clone());
				if account_state_maybe.is_none() {
					let ed = <T as Config>::Currency::minimum_balance();
					let total_balance = <T as Config>::Currency::total_balance(&who);
					if total_balance < ed {
						account_state_maybe = Some(AccountState::Preserve);
					}
				}
				match account_state_maybe {
					Some(AccountState::Part { free, reserved, consumers, .. }) => {
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
						// If the total balance is smaller than the existential deposit, we don't
						// need to check anything else because this is a sufficient reason to
						// preserve the account on the relay chain. If the total balance is
						// greater or equal to the existential deposit, we need to check that
						// the account has no Holds, Freezes, or Locks, and no free balance to
						// be migrated to Asset Hub.
						let total_balance = <T as Config>::Currency::total_balance(&who);
						let ed = <T as Config>::Currency::minimum_balance();
						if total_balance >= ed {
							let manager = Manager::<T>::get();
							let on_demand_pallet_account: T::AccountId =
								T::OnDemandPalletId::get().into_account_truncating();
							let is_manager = manager.as_ref().is_some_and(|m| *m == who);
							let is_on_demand = who == on_demand_pallet_account;
							assert!(
								is_manager || is_on_demand,
								"Only the on-demand pallet account or the manager (if set) may have \
								`AccountState::Preserve` state on the Relay Chain"
							);
						}
					},
					// This corresponds to AccountState::Migrate: the account should be fully
					// migrated to Asset Hub.
					Some(AccountState::Migrate) | None => {
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
			let tracker = RcMigratedBalanceArchive::<T>::get();
			assert_eq!(
				total_issuance,
				rc_total_issuance_before.saturating_sub(tracker.migrated),
				"Change on total issuance on the relay chain after migration is not as expected"
			);
			assert_eq!(
				total_issuance, tracker.kept,
				"Kept balance on the relay chain after migration is not as expected"
			);
		}
	}
}
