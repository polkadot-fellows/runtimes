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

/*
TODO: remove this dec comment when not needed

Sources of account references

provider refs:
- crowdloans: fundraising system account / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/polkadot/runtime/common/src/crowdloan/mod.rs#L416
- parachains_assigner_on_demand / on_demand: pallet's account https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/polkadot/runtime/parachains/src/on_demand/mod.rs#L407
- balances: user account / existential deposit
- session: initial validator set on Genesis / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/session/src/lib.rs#L466
- delegated-staking: delegators and agents (users)

consumer refs:
- balances:
-- might hold on account mutation / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/balances/src/lib.rs#L1007
-- on migration to new logic for every migrating account / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/balances/src/lib.rs#L877
- session:
-- for user setting the keys / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/session/src/lib.rs#L812
-- initial validator set on Genesis / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/session/src/lib.rs#L461
- recovery: user on recovery claim / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/recovery/src/lib.rs#L610
- staking:
-- for user bonding / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/staking/src/pallet/mod.rs#L1036
-- virtual bond / agent key / https://github.com/paritytech/polkadot-sdk/blob/ace62f120fbc9ec617d6bab0a5180f0be4441537/substrate/frame/staking/src/pallet/impls.rs#L1948

sufficient refs:
- must be zero since only assets pallet might hold such reference
*/

/*
TODO: remove when not needed

Regular native asset teleport from Relay (mint authority) to Asset Hub looks like:

Relay: burn_from(source, amount) // publishes Balances::Burned event
Relay: mint_into(checking, amount) // publishes Balances::Minted event
Relay: no effect on total issuance
Relay: XCM with teleport sent
AH: mint_into(dest, amount) // publishes Balances::Minted event
AH: total issuance increased by `amount`
Relay: XCM teleport processed

^ The minimum what we should replay while moving accounts from Relay to AH

When the Asset Hub turned to the mint authority

Relay: let checking_total = // total checking account balance
Relay: burn_from(checking, checking_total) // publishes Balances::Burned event
AH: let total_issuance = // total issuance on AH
AH: mint_into(checking, checking_total - total_issuance) // publishes Balances::Minted event

^ Ensure that this is the desired method of communicating the mint authority change via events.

*/

use crate::{types::*, *};
use frame_support::{traits::tokens::IdAmount, weights::WeightMeter};
use frame_system::Account as SystemAccount;
use pallet_balances::{AccountData, BalanceLock};
use sp_runtime::traits::Zero;

/// Account type meant to transfer data between RC and AH.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
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
	pub holds: Vec<IdAmount<HoldReason, Balance>>,
	/// Account freezes from Relay Chain.
	pub freezes: Vec<IdAmount<FreezeReason, Balance>>,
	/// Account locks from Relay Chain.
	pub locks: Vec<BalanceLock<Balance>>,
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

/// The state for the Relay Chain accounts.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub enum AccountState<Balance> {
	/// The account should be migrated to AH and removed on RC.
	Migrate,
	/// The account must stay on RC.
	///
	/// E.g. RC system account.
	Preserve,

	// We might not need the `Part` variation since there are no many cases for `Part` we can just
	// keep the whole account balance on RC
	/// The part of the account must be preserved on RC.
	///
	/// Cases:
	/// - accounts placed deposit for parachain registration (paras_registrar pallet);
	/// - accounts placed deposit for hrmp channel registration (parachains_hrmp pallet);
	Part {
		/// Free part of the account the must be preserved on RC.
		///
		/// In practice the new ED.
		free: Balance,
		/// The reserved balance that must be preserved on RC.
		///
		/// In practice reserved by old `Currency` api and has no associated reason.
		reserved: Balance,
	},
}

pub type AccountStateFor<T> = AccountState<<T as pallet_balances::Config>::Balance>;
pub type AccountFor<T> = Account<
	<T as frame_system::Config>::AccountId,
	<T as pallet_balances::Config>::Balance,
	<T as pallet_balances::Config>::RuntimeHoldReason,
	<T as pallet_balances::Config>::FreezeIdentifier,
>;

pub struct AccountsMigrator<T: Config> {
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
	/// - Some(maybe_last_key) - the last migrated account from RC to AH if
	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Error<T>> {
		// we should not send more than AH can handle within the block.
		let mut ah_weight_counter = WeightMeter::with_limit(T::MaxAhWeight::get());
		// accounts batch for the current iteration.
		let mut batch = Vec::new();

		// TODO transport weight. probably we need to leave some buffer since we do not know how
		// many send batches the one migrate_many will require.
		let xcm_weight = Weight::from_all(1);
		if weight_counter.try_consume(xcm_weight).is_err() {
			return Err(Error::OutOfWeight);
		}

		let mut iter = if let Some(ref last_key) = last_key {
			SystemAccount::<T>::iter_from_key(last_key)
		} else {
			SystemAccount::<T>::iter()
		};

		let mut maybe_last_key = last_key;
		loop {
			let Some((who, account_info)) = iter.next() else {
				maybe_last_key = None;
				break;
			};

			let withdraw_res =
				with_transaction_opaque_err::<Option<AccountFor<T>>, Error<T>, _>(|| {
					match Self::withdraw_account(
						who.clone(),
						account_info.clone(),
						weight_counter,
						&mut ah_weight_counter,
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

		if batch.is_empty() {
			Pallet::<T>::send_chunked_xcm(batch, |batch| {
				types::AhMigratorCall::<T>::ReceiveAccounts { accounts: batch }
			})?;
		}

		Ok(maybe_last_key)
	}
}

impl<T: Config> AccountsMigrator<T> {
	// TODO: Currently, we use `debug_assert!` for basic test checks against a production snapshot.

	/// Migrate a single account out of the Relay chain and return it.
	///
	/// The account on the relay chain is modified as part of this operation.
	fn withdraw_account(
		who: T::AccountId,
		account_info: AccountInfoFor<T>,
		rc_weight: &mut WeightMeter,
		ah_weight: &mut WeightMeter,
	) -> Result<Option<AccountFor<T>>, Error<T>> {
		// account for `get_rc_state` read below
		if rc_weight.try_consume(T::DbWeight::get().reads(1)).is_err() {
			return Err(Error::OutOfWeight);
		}

		let rc_state = Self::get_rc_state(&who);

		if matches!(rc_state, AccountState::Preserve) {
			log::debug!(
				target: LOG_TARGET,
				"Preserve account '{:?}' on Relay Chain",
				who.to_ss58check(),
			);
			return Ok(None);
		}

		// TODO: we do not expect `Part` variation for now and might delete it later
		debug_assert!(!matches!(rc_state, AccountState::Part { .. }));

		log::debug!(
			target: LOG_TARGET,
			"Migrating account '{}'",
			who.to_ss58check(),
		);

		// account the weight for migrating a single account on Relay Chain.
		if rc_weight.try_consume(T::RcWeightInfo::migrate_account()).is_err() {
			return Err(Error::OutOfWeight);
		}

		// account the weight for receiving a single account on Asset Hub.
		if ah_weight.try_consume(T::AhWeightInfo::migrate_account()).is_err() {
			return Err(Error::OutOfWeight);
		}

		// migrate the target account:
		// - keep `balance`, `holds`, `freezes`, .. in memory
		// - release all `holds`, `freezes`, ...
		// - teleport all balance from RC to AH:
		// -- mint into XCM `checking` account
		// -- burn from target account
		// - add `balance`, `holds`, `freezes`, .. to the accounts package to be sent via XCM

		let account_data: AccountData<T::Balance> = account_info.data.clone();

		if account_data.free.is_zero() &&
			account_data.reserved.is_zero() &&
			account_data.frozen.is_zero()
		{
			if account_info.nonce.is_zero() {
				log::warn!(
					target: LOG_TARGET,
					"Possible system account detected. \
					Consumer ref: {}, Provider ref: {}, Account: '{}'",
					account_info.consumers,
					account_info.providers,
					who.to_ss58check()
				);
			} else {
				log::warn!(target: LOG_TARGET, "Weird account detected '{}'", who.to_ss58check());
			}
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

		let holds: Vec<IdAmount<T::RuntimeHoldReason, T::Balance>> =
			pallet_balances::Holds::<T>::get(&who).into();

		for hold in &holds {
			if let Err(e) =
				<T as Config>::Currency::release(&hold.id, &who, hold.amount, Precision::Exact)
			{
				log::error!(target: LOG_TARGET,
					"Failed to release hold: {:?} \
					for account: {:?} \
					with error: {:?}",
					hold.id,
					who.to_ss58check(),
					e
				);
				return Err(Error::FailedToWithdrawAccount);
			}
		}

		let locks: Vec<BalanceLock<T::Balance>> =
			pallet_balances::Locks::<T>::get(&who).into_inner();

		for lock in &locks {
			// Expected lock ids:
			// - "staking " // should be transformed to hold with https://github.com/paritytech/polkadot-sdk/pull/5501
			// - "vesting "
			// - "pyconvot"
			<T as Config>::Currency::remove_lock(lock.id, &who);
		}

		let unnamed_reserve = <T as Config>::Currency::reserved_balance(&who);
		let _ = <T as Config>::Currency::unreserve(&who, unnamed_reserve);

		// TODO: To ensure the account can be fully withdrawn from RC to AH, we force-update the
		// references here. After inspecting the state, it's clear that fully correcting the
		// reference counts would be nearly impossible. Instead, for accounts meant to be fully
		// migrated to the AH, we will calculate the actual reference counts based on the
		// migrating pallets and transfer them to AH. This is done using the
		// `Self::get_consumer_count` and `Self::get_provider_count` functions.
		SystemAccount::<T>::mutate(&who, |a| {
			a.consumers = 0;
			a.providers = 1;
		});

		let balance = <T as Config>::Currency::reducible_balance(
			&who,
			Preservation::Expendable,
			Fortitude::Polite,
		);
		let total_balance = <T as Config>::Currency::total_balance(&who);

		debug_assert!(total_balance == balance);
		debug_assert!(total_balance == account_data.free + account_data.reserved);
		debug_assert!(total_balance >= <T as Config>::AhExistentialDeposit::get());

		let burned = match <T as Config>::Currency::burn_from(
			&who,
			total_balance,
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

		debug_assert!(total_balance == burned);

		let minted =
			match <T as Config>::Currency::mint_into(&T::CheckingAccount::get(), total_balance) {
				Ok(minted) => minted,
				Err(e) => {
					log::error!(
						target: LOG_TARGET,
						"Failed to mint balance into checking account: {}, error: {:?}",
						who.to_ss58check(),
						e
					);
					return Err(Error::FailedToWithdrawAccount);
				},
			};

		debug_assert!(total_balance == minted);

		Ok(Some(Account {
			who: who.clone(),
			free: account_data.free,
			reserved: account_data.reserved,
			frozen: account_data.frozen,
			holds,
			freezes,
			locks,
			unnamed_reserve,
			consumers: Self::get_consumer_count(&who, &account_info),
			providers: Self::get_provider_count(&who, &account_info),
		}))
	}

	/// Consumer ref count of migrating to Asset Hub pallets except a reference for `reserved` and
	/// `frozen` balance.
	///
	/// Since the `reserved` and `frozen` balances will be known on a receiving side (AH) they will
	/// be calculated there.
	pub fn get_consumer_count(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> u8 {
		// TODO: check the pallets for consumer references on Relay Chain.

		// The following pallets increase consumers and are deployed on (Polkadot, Kusama, Westend):
		// - `balances`: (P/K/W)
		// - `recovery`: (/K/W)
		// - `assets`: (//)
		// - `contracts`: (//)
		// - `nfts`: (//)
		// - `uniques`: (//)
		// - `revive`: (//)
		// Staking stuff:
		// - `session`: (P/K/W)
		// - `staking`: (P/K/W)

		0
	}

	/// Provider ref count of migrating to Asset Hub pallets except the reference for existential
	/// deposit.
	///
	/// Since the `free` balance will be known on a receiving side (AH) the ref count will be
	/// calculated there.
	pub fn get_provider_count(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> u8 {
		// TODO: check the pallets for provider references on Relay Chain.

		// The following pallets increase provider and are deployed on (Polkadot, Kusama, Westend):
		// - `crowdloan`: (P/K/W) https://github.com/paritytech/polkadot-sdk/blob/master/polkadot/runtime/common/src/crowdloan/mod.rs#L416
		// - `parachains_on_demand`: (P/K/W) https://github.com/paritytech/polkadot-sdk/blob/master/polkadot/runtime/parachains/src/on_demand/mod.rs#L407
		// - `balances`: (P/K/W) https://github.com/paritytech/polkadot-sdk/blob/master/substrate/frame/balances/src/lib.rs#L1026
		// - `broker`: (_/_/_)
		// - `delegate_staking`: (P/K/W)
		// - `session`: (P/K/W) <- Don't count this one (see https://github.com/paritytech/polkadot-sdk/blob/8d4138f77106a6af49920ad84f3283f696f3f905/substrate/frame/session/src/lib.rs#L462-L465)

		0
	}

	/// The part of the balance of the `who` that must stay on the Relay Chain.
	pub fn get_rc_state(who: &T::AccountId) -> AccountStateFor<T> {
		// TODO: static list of System Accounts that must stay on RC
		// e.g. XCM teleport checking account

		if let Some(state) = RcAccounts::<T>::get(who) {
			return state;
		}
		AccountStateFor::<T>::Migrate
	}

	/// Obtain all known accounts that must stay on RC and persist it to the [`RcAccounts`] storage
	/// item.
	///
	/// Should be executed once before the migration starts.
	pub fn obtain_rc_accounts() -> Weight {
		for (channel_id, _info) in hrmp::HrmpChannels::<T>::iter() {
			let sender: T::AccountId = channel_id.sender.into_account_truncating();
			RcAccounts::<T>::insert(sender, AccountStateFor::<T>::Preserve);

			let recipient: T::AccountId = channel_id.recipient.into_account_truncating();
			RcAccounts::<T>::insert(recipient, AccountStateFor::<T>::Preserve);
		}

		for (_, info) in Paras::<T>::iter() {
			RcAccounts::<T>::insert(
				info.manager,
				// TODO: we can use `Part` variation to keep only the reserved part on RC
				// for now for simplicity we preserve the whole account on RC
				AccountStateFor::<T>::Preserve,
			);
		}

		// TODO: should we consider `hrmp::HrmpOpenChannelRequests` or we can just clean up it
		// before the migration.

		// TODO: define actual weight
		Weight::from_all(1)
	}
}
