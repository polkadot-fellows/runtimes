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

Relay: mint_into(checking, amount) // publishes Balances::Minted event
Relay: burn_from(source, amount) // publishes Balances::Burned event
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

use crate::*;
use frame_support::{traits::tokens::IdAmount, weights::WeightMeter};
use frame_system::Account as SystemAccount;
use pallet_balances::{AccountData, BalanceLock};

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

impl<T: Config> Pallet<T> {
	// TODO: Currently, we use `debug_assert!` for basic test checks against a production snapshot.

	/// Migrate accounts from RC to AH.
	///
	/// Parameters:
	/// - `maybe_last_key` - the last migrated account from RC to AH if any
	/// - `weight_counter` - the weight meter
	///
	/// Result:
	/// - None - no accounts left to be migrated to AH.
	/// - Some(maybe_last_key) - the last migrated account from RC to AH if
	pub fn migrate_accounts(
		maybe_last_key: Option<T::AccountId>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Option<T::AccountId>>, ()> {
		// we should not send more than AH can handle within the block.
		let mut ah_weight_counter = WeightMeter::with_limit(T::MaxAhWeight::get());
		// accounts package for the current iteration.
		let mut package = Vec::new();

		// TODO transport weight
		let xcm_weight = Weight::from_all(1);
		if weight_counter.try_consume(xcm_weight).is_err() {
			return Ok(Some(maybe_last_key));
		}

		let iter = if let Some(ref last_key) = maybe_last_key {
			SystemAccount::<T>::iter_from_key(last_key)
		} else {
			SystemAccount::<T>::iter()
		};

		let mut maybe_last_key = maybe_last_key;
		let mut last_package = true;
		for (who, account_info) in iter {
			// account for `get_rc_state` read below
			if weight_counter.try_consume(T::DbWeight::get().reads(1)).is_err() {
				last_package = false;
				break;
			}

			let rc_state = Self::get_rc_state(&who);

			if matches!(rc_state, AccountState::Preserve) {
				log::debug!(
					target: LOG_TARGET,
					"Preserve account '{:?}' on Relay Chain",
					who.to_ss58check(),
				);
				maybe_last_key = Some(who);
				continue;
			}

			// TODO: we do not expect `Part` variation for now and might delete it later
			debug_assert!(!matches!(rc_state, AccountState::Part { .. }));

			log::debug!(
				target: LOG_TARGET,
				"Migrating account '{:?}'",
				who.to_ss58check(),
			);

			// account the weight for migrating a single account on Relay Chain.
			if weight_counter.try_consume(T::RcWeightInfo::migrate_account()).is_err() {
				last_package = false;
				break;
			}

			// account the weight for receiving a single account on Asset Hub.
			if ah_weight_counter.try_consume(T::AhWeightInfo::migrate_account()).is_err() {
				last_package = false;
				break;
			}

			// migrate the target account:
			// - keep `balance`, `holds`, `freezes`, .. in memory
			// - release all `holds`, `freezes`, ...
			// - teleport all balance from RC to AH:
			// -- mint into XCM `checking` account
			// -- burn from target account
			// - add `balance`, `holds`, `freezes`, .. to the accounts package to be sent via XCM

			let account_data: AccountData<T::Balance> = account_info.data.clone();

			let freezes: Vec<IdAmount<T::FreezeIdentifier, T::Balance>> =
				pallet_balances::Freezes::<T>::get(&who).into();

			for freeze in &freezes {
				let _ = <T as Config>::Currency::thaw(&freeze.id, &who)
					// TODO: handle error
					.unwrap();
			}

			let holds: Vec<IdAmount<T::RuntimeHoldReason, T::Balance>> =
				pallet_balances::Holds::<T>::get(&who).into();

			for hold in &holds {
				let _ =
					<T as Config>::Currency::release(&hold.id, &who, hold.amount, Precision::Exact)
						// TODO: handle error
						.unwrap();
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
			// TODO: total_balance > ED on AH

			let burned = <T as Config>::Currency::burn_from(
				&who,
				total_balance,
				Preservation::Expendable,
				Precision::Exact,
				Fortitude::Polite,
			)
			// TODO: handle error
			.unwrap();

			debug_assert!(total_balance == burned);

			let minted =
				<T as Config>::Currency::mint_into(&T::CheckingAccount::get(), total_balance)
					// TODO: handle error;
					.unwrap();

			debug_assert!(total_balance == minted);

			let account_to_ah = Account {
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
			};

			package.push(account_to_ah);
			maybe_last_key = Some(who);
		}

		let call = types::AssetHubPalletConfig::<T>::AhmController(
			types::AhMigratorCall::<T>::ReceiveAccounts { accounts: package },
		);

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

		if let Err(_err) =
			send_xcm::<T::SendXcm>(Location::new(0, [Junction::Parachain(1000)]), message.clone())
		{
			return Err(());
		};

		if last_package {
			Ok(None)
		} else {
			Ok(Some(maybe_last_key))
		}
	}

	/// Consumer ref count of migrating to Asset Hub pallets except a reference for `reserved` and
	/// `frozen` balance.
	///
	/// Since the `reserved` and `frozen` balances will be known on a receiving side (AH) they will
	/// be calculated there.
	pub fn get_consumer_count(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> u8 {
		// TODO: check the pallets for provider references on Relay Chain.
		0
	}

	/// Provider ref count of migrating to Asset Hub pallets except the reference for existential
	/// deposit.
	///
	/// Since the `free` balance will be known on a receiving side (AH) the ref count will be
	/// calculated there.
	pub fn get_provider_count(_who: &T::AccountId, _info: &AccountInfoFor<T>) -> u8 {
		// TODO: check the pallets for provider references on Relay Chain.
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
