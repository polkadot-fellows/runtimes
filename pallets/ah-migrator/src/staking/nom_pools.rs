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

use crate::*;
use frame_support::{traits::ConstU32, BoundedVec};
use pallet_nomination_pools::BondedPoolInner;
#[cfg(feature = "std")]
use pallet_rc_migrator::staking::nom_pools::tests;
use pallet_rc_migrator::{
	staking::nom_pools::{BalanceOf, NomPoolsMigrator, NomPoolsStorageValues},
	types::ToPolkadotSs58,
};

/// Trait to provide account translation logic for bonded pool structures.
///
/// This trait works with two different pool types that share the same field structure:
/// 1. `BondedPoolInner<T>` - The actual runtime pallet type used during migration
/// 2. `tests::GenericBondedPoolInner<Balance, AccountId, BlockNumber>` - Generic test type used for
///    verification
trait TranslateBondedPoolAccounts<AccountId> {
	fn translate_bonded_pool_accounts<F>(&mut self, translate_fn: F)
	where
		F: Fn(AccountId) -> AccountId;
}

impl<T: Config> TranslateBondedPoolAccounts<T::AccountId> for BondedPoolInner<T> {
	fn translate_bonded_pool_accounts<F>(&mut self, translate_fn: F)
	where
		F: Fn(T::AccountId) -> T::AccountId,
	{
		// Translate accounts in pool roles
		self.roles.depositor = translate_fn(self.roles.depositor.clone());
		self.roles.root = self.roles.root.clone().map(&translate_fn);
		self.roles.nominator = self.roles.nominator.clone().map(&translate_fn);
		self.roles.bouncer = self.roles.bouncer.clone().map(&translate_fn);

		// Translate commission accounts
		if let Some(pallet_nomination_pools::CommissionClaimPermission::Account(ref mut account)) =
			self.commission.claim_permission
		{
			*account = translate_fn(account.clone());
		}
		if let Some((rate, ref mut account)) = self.commission.current {
			self.commission.current = Some((rate, translate_fn(account.clone())));
		}
	}
}

#[cfg(feature = "std")]
impl<Balance, AccountId, BlockNumber> TranslateBondedPoolAccounts<AccountId>
	for tests::GenericBondedPoolInner<Balance, AccountId, BlockNumber>
where
	AccountId: Clone + core::fmt::Debug,
	Balance: core::fmt::Debug,
	BlockNumber: core::fmt::Debug,
{
	fn translate_bonded_pool_accounts<F>(&mut self, translate_fn: F)
	where
		F: Fn(AccountId) -> AccountId,
	{
		// Translate accounts in pool roles
		self.roles.depositor = translate_fn(self.roles.depositor.clone());
		self.roles.root = self.roles.root.clone().map(&translate_fn);
		self.roles.nominator = self.roles.nominator.clone().map(&translate_fn);
		self.roles.bouncer = self.roles.bouncer.clone().map(&translate_fn);

		// Translate commission accounts
		if let Some(pallet_nomination_pools::CommissionClaimPermission::Account(ref mut account)) =
			self.commission.claim_permission
		{
			*account = translate_fn(account.clone());
		}
		if let Some((rate, ref mut account)) = self.commission.current {
			self.commission.current = Some((rate, translate_fn(account.clone())));
		}
	}
}

/// Trait for nom pools message types that can have their accounts translated.
trait TranslateNomPoolsMessage<AccountId, Balance, RewardCounter, BlockNumber> {
	fn translate_accounts<F>(self, translate_fn: F) -> Self
	where
		F: Fn(AccountId) -> AccountId;
}

impl<T: Config>
	TranslateNomPoolsMessage<T::AccountId, BalanceOf<T>, T::RewardCounter, BlockNumberFor<T>>
	for RcNomPoolsMessage<T>
{
	fn translate_accounts<F>(self, translate_fn: F) -> Self
	where
		F: Fn(T::AccountId) -> T::AccountId,
	{
		use RcNomPoolsMessage::*;
		match self {
			StorageValues { values } => StorageValues { values },
			PoolMembers { member: (account_id, member_data) } => {
				let translated_account = translate_fn(account_id);
				PoolMembers { member: (translated_account, member_data) }
			},
			BondedPools { pool: (pool_id, mut pool_data) } => {
				pool_data.translate_bonded_pool_accounts(&translate_fn);
				BondedPools { pool: (pool_id, pool_data) }
			},
			RewardPools { rewards } => RewardPools { rewards },
			SubPoolsStorage { sub_pools } => SubPoolsStorage { sub_pools },
			Metadata { meta } => Metadata { meta },
			ReversePoolIdLookup { lookups: (account_id, pool_id) } => {
				let translated_account = translate_fn(account_id);
				ReversePoolIdLookup { lookups: (translated_account, pool_id) }
			},
			ClaimPermissions { perms: (account_id, permissions) } => {
				let translated_account = translate_fn(account_id);
				ClaimPermissions { perms: (translated_account, permissions) }
			},
		}
	}
}

#[cfg(feature = "std")]
impl<Balance, RewardCounter, AccountId, BlockNumber>
	TranslateNomPoolsMessage<AccountId, Balance, RewardCounter, BlockNumber>
	for tests::GenericNomPoolsMessage<Balance, RewardCounter, AccountId, BlockNumber>
where
	AccountId: Clone + core::fmt::Debug + PartialEq,
	Balance: Clone + core::fmt::Debug + PartialEq,
	RewardCounter: Clone + core::fmt::Debug + PartialEq,
	BlockNumber: Clone + core::fmt::Debug + PartialEq,
{
	fn translate_accounts<F>(self, translate_fn: F) -> Self
	where
		F: Fn(AccountId) -> AccountId,
	{
		use tests::GenericNomPoolsMessage::*;
		match self {
			StorageValues { values } => StorageValues { values },
			PoolMembers { member: (account_id, member_data) } => {
				let translated_account = translate_fn(account_id);
				PoolMembers { member: (translated_account, member_data) }
			},
			BondedPools { pool: (pool_id, mut pool_data) } => {
				pool_data.translate_bonded_pool_accounts(&translate_fn);
				BondedPools { pool: (pool_id, pool_data) }
			},
			RewardPools { rewards } => RewardPools { rewards },
			SubPoolsStorage { sub_pools } => SubPoolsStorage { sub_pools },
			Metadata { meta } => Metadata { meta },
			ReversePoolIdLookup { lookups: (account_id, pool_id) } => {
				let translated_account = translate_fn(account_id);
				ReversePoolIdLookup { lookups: (translated_account, pool_id) }
			},
			ClaimPermissions { perms: (account_id, permissions) } => {
				let translated_account = translate_fn(account_id);
				ClaimPermissions { perms: (translated_account, permissions) }
			},
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn do_receive_nom_pools_messages(
		messages: Vec<RcNomPoolsMessage<T>>,
	) -> Result<(), Error<T>> {
		let mut good = 0;
		log::info!(target: LOG_TARGET, "Integrating {} NomPoolsMessages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::NomPools,
			count: messages.len() as u32,
		});

		for message in messages {
			Self::do_receive_nom_pools_message(message);
			good += 1;
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::NomPools,
			count_good: good as u32,
			count_bad: 0,
		});
		Ok(())
	}

	pub fn do_receive_nom_pools_message(message: RcNomPoolsMessage<T>) {
		// First translate the message
		let translated_message = message.translate_accounts(Self::translate_account_rc_to_ah);

		// Then process the translated message
		use RcNomPoolsMessage::*;
		match translated_message {
			StorageValues { values } => {
				pallet_rc_migrator::staking::nom_pools::NomPoolsMigrator::<T>::put_values(values);
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsStorageValues");
			},
			PoolMembers { member: (account_id, member_data) } => {
				debug_assert!(!pallet_nomination_pools::PoolMembers::<T>::contains_key(
					&account_id
				));
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsPoolMember: {}",
					account_id.to_polkadot_ss58());
				pallet_nomination_pools::PoolMembers::<T>::insert(account_id, member_data);
			},
			BondedPools { pool: (pool_id, pool_data) } => {
				debug_assert!(!pallet_nomination_pools::BondedPools::<T>::contains_key(pool_id));
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsBondedPool: {}", &pool_id);
				pallet_nomination_pools::BondedPools::<T>::insert(pool_id, pool_data);
			},
			RewardPools { rewards } => {
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsRewardPool: {:?}", &rewards.0);
				// Not sure if it is the best to use the alias here, but it is the easiest...
				pallet_rc_migrator::staking::nom_pools_alias::RewardPools::<T>::insert(
					rewards.0, rewards.1,
				);
			},
			SubPoolsStorage { sub_pools } => {
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsSubPoolsStorage: {:?}", &sub_pools.0);
				pallet_rc_migrator::staking::nom_pools_alias::SubPoolsStorage::<T>::insert(
					sub_pools.0,
					sub_pools.1,
				);
			},
			Metadata { meta } => {
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsMetadata: {:?}", &meta.0);
				pallet_nomination_pools::Metadata::<T>::insert(meta.0, meta.1);
			},
			ReversePoolIdLookup { lookups: (account_id, pool_id) } => {
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsReversePoolIdLookup: {}",
					account_id.to_polkadot_ss58());
				pallet_nomination_pools::ReversePoolIdLookup::<T>::insert(account_id, pool_id);
			},
			ClaimPermissions { perms: (account_id, permissions) } => {
				log::debug!(target: LOG_TARGET, "Integrating NomPoolsClaimPermissions: {}",
					account_id.to_polkadot_ss58());
				pallet_nomination_pools::ClaimPermissions::<T>::insert(account_id, permissions);
			},
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for NomPoolsMigrator<T> {
	type RcPrePayload = Vec<
		tests::GenericNomPoolsMessage<
			BalanceOf<T>,
			T::RewardCounter,
			<T as frame_system::Config>::AccountId,
			BlockNumberFor<T>,
		>,
	>;
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		assert!(
			pallet_nomination_pools::TotalValueLocked::<T>::get().is_zero(),
			"Assert storage 'NominationPools::TotalValueLocked::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::MinJoinBond::<T>::get().is_zero(),
			"Assert storage 'NominationPools::MinJoinBond::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::MinCreateBond::<T>::get().is_zero(),
			"Assert storage 'NominationPools::MinCreateBond::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::MaxPools::<T>::get().is_none(),
			"Assert storage 'NominationPools::MaxPools::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::MaxPoolMembers::<T>::get().is_none(),
			"Assert storage 'NominationPools::MaxPoolMembers::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::MaxPoolMembersPerPool::<T>::get().is_none(),
			"Assert storage 'NominationPools::MaxPoolMembersPerPool::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::GlobalMaxCommission::<T>::get().is_none(),
			"Assert storage 'NominationPools::GlobalMaxCommission::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::LastPoolId::<T>::get().is_zero(),
			"Assert storage 'NominationPools::LastPoolId::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::PoolMembers::<T>::iter().next().is_none(),
			"Assert storage 'NominationPools::PoolMembers::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::BondedPools::<T>::iter().next().is_none(),
			"Assert storage 'NominationPools::BondedPools::ah_pre::empty'"
		);
		assert!(
			pallet_rc_migrator::staking::nom_pools_alias::RewardPools::<T>::iter()
				.next()
				.is_none(),
			"Assert storage 'NominationPools::RewardPools::ah_pre::empty'"
		);
		assert!(
			pallet_rc_migrator::staking::nom_pools_alias::SubPoolsStorage::<T>::iter()
				.next()
				.is_none(),
			"Assert storage 'NominationPools::SubPoolsStorage::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::Metadata::<T>::iter().next().is_none(),
			"Assert storage 'NominationPools::Metadata::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::ReversePoolIdLookup::<T>::iter().next().is_none(),
			"Assert storage 'NominationPools::ReversePoolIdLookup::ah_pre::empty'"
		);
		assert!(
			pallet_nomination_pools::ClaimPermissions::<T>::iter().next().is_none(),
			"Assert storage 'NominationPools::ClaimPermissions::ah_pre::empty'"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		// Build expected data by applying account translation to RC pre-payload data
		let expected_ah_messages: Vec<_> = rc_pre_payload
			.into_iter()
			.map(|message| message.translate_accounts(Pallet::<T>::translate_account_rc_to_ah))
			.collect();

		let mut ah_messages = Vec::new();

		// Collect storage values from AH
		let values = NomPoolsStorageValues {
			total_value_locked: pallet_nomination_pools::TotalValueLocked::<T>::try_get().ok(),
			min_join_bond: pallet_nomination_pools::MinJoinBond::<T>::try_get().ok(),
			min_create_bond: pallet_nomination_pools::MinCreateBond::<T>::try_get().ok(),
			max_pools: pallet_nomination_pools::MaxPools::<T>::get(),
			max_pool_members: pallet_nomination_pools::MaxPoolMembers::<T>::get(),
			max_pool_members_per_pool: pallet_nomination_pools::MaxPoolMembersPerPool::<T>::get(),
			global_max_commission: pallet_nomination_pools::GlobalMaxCommission::<T>::get(),
			last_pool_id: pallet_nomination_pools::LastPoolId::<T>::try_get().ok(),
		};
		ah_messages.push(tests::GenericNomPoolsMessage::StorageValues { values });

		// Collect all other storage items from AH
		for (who, member) in pallet_nomination_pools::PoolMembers::<T>::iter() {
			let generic_member = tests::GenericPoolMember {
				pool_id: member.pool_id,
				points: member.points,
				last_recorded_reward_counter: member.last_recorded_reward_counter,
				unbonding_eras: member.unbonding_eras.into_inner(),
			};
			ah_messages
				.push(tests::GenericNomPoolsMessage::PoolMembers { member: (who, generic_member) });
		}

		for (pool_id, pool) in pallet_nomination_pools::BondedPools::<T>::iter() {
			let generic_pool = tests::GenericBondedPoolInner {
				commission: tests::GenericCommission {
					current: pool.commission.current,
					max: pool.commission.max,
					change_rate: pool.commission.change_rate,
					throttle_from: pool.commission.throttle_from,
					claim_permission: pool.commission.claim_permission,
				},
				member_counter: pool.member_counter,
				points: pool.points,
				roles: pool.roles,
				state: pool.state,
			};
			ah_messages
				.push(tests::GenericNomPoolsMessage::BondedPools { pool: (pool_id, generic_pool) });
		}

		for (pool_id, rewards) in
			pallet_rc_migrator::staking::nom_pools_alias::RewardPools::<T>::iter()
		{
			let generic_rewards = tests::GenericRewardPool {
				last_recorded_reward_counter: rewards.last_recorded_reward_counter,
				last_recorded_total_payouts: rewards.last_recorded_total_payouts,
				total_rewards_claimed: rewards.total_rewards_claimed,
				total_commission_pending: rewards.total_commission_pending,
				total_commission_claimed: rewards.total_commission_claimed,
			};
			ah_messages.push(tests::GenericNomPoolsMessage::RewardPools {
				rewards: (pool_id, generic_rewards),
			});
		}

		for (pool_id, sub_pools) in
			pallet_rc_migrator::staking::nom_pools_alias::SubPoolsStorage::<T>::iter()
		{
			let generic_sub_pools = tests::GenericSubPools {
				no_era: tests::GenericUnbondPool {
					points: sub_pools.no_era.points,
					balance: sub_pools.no_era.balance,
				},
				with_era: sub_pools
					.with_era
					.into_inner()
					.into_iter()
					.map(|(era, pool)| {
						(
							era,
							tests::GenericUnbondPool { points: pool.points, balance: pool.balance },
						)
					})
					.collect(),
			};
			ah_messages.push(tests::GenericNomPoolsMessage::SubPoolsStorage {
				sub_pools: (pool_id, generic_sub_pools),
			});
		}

		for (pool_id, meta) in pallet_nomination_pools::Metadata::<T>::iter() {
			let meta_converted = BoundedVec::<u8, ConstU32<256>>::try_from(meta.into_inner())
				.expect("Metadata length is known to be within bounds; qed");
			ah_messages
				.push(tests::GenericNomPoolsMessage::Metadata { meta: (pool_id, meta_converted) });
		}

		for (who, pool_id) in pallet_nomination_pools::ReversePoolIdLookup::<T>::iter() {
			ah_messages.push(tests::GenericNomPoolsMessage::ReversePoolIdLookup {
				lookups: (who, pool_id),
			});
		}

		for (who, perms) in pallet_nomination_pools::ClaimPermissions::<T>::iter() {
			ah_messages
				.push(tests::GenericNomPoolsMessage::ClaimPermissions { perms: (who, perms) });
		}

		// Assert storage "NominationPools::TotalValueLocked::ah_post::correct"
		// Assert storage "NominationPools::TotalValueLocked::ah_post::consistent"
		// Assert storage "NominationPools::MinJoinBond::ah_post::correct"
		// Assert storage "NominationPools::MinJoinBond::ah_post::consistent"
		// Assert storage "NominationPools::MinCreateBond::ah_post::correct"
		// Assert storage "NominationPools::MinCreateBond::ah_post::consistent"
		// Assert storage "NominationPools::MaxPools::ah_post::correct"
		// Assert storage "NominationPools::MaxPools::ah_post::consistent"
		// Assert storage "NominationPools::MaxPoolMembers::ah_post::correct"
		// Assert storage "NominationPools::MaxPoolMembers::ah_post::consistent"
		// Assert storage "NominationPools::MaxPoolMembersPerPool::ah_post::correct"
		// Assert storage "NominationPools::MaxPoolMembersPerPool::ah_post::consistent"
		// Assert storage "NominationPools::GlobalMaxCommission::ah_post::correct"
		// Assert storage "NominationPools::GlobalMaxCommission::ah_post::consistent"
		// Assert storage "NominationPools::LastPoolId::ah_post::correct"
		// Assert storage "NominationPools::LastPoolId::ah_post::consistent"
		// Assert storage "NominationPools::PoolMembers::ah_post::correct"
		// Assert storage "NominationPools::PoolMembers::ah_post::consistent"
		// Assert storage "NominationPools::BondedPools::ah_post::correct"
		// Assert storage "NominationPools::BondedPools::ah_post::consistent"
		// Assert storage "NominationPools::RewardPools::ah_post::correct"
		// Assert storage "NominationPools::RewardPools::ah_post::consistent"
		// Assert storage "NominationPools::SubPoolsStorage::ah_post::correct"
		// Assert storage "NominationPools::SubPoolsStorage::ah_post::consistent"
		// Assert storage "NominationPools::Metadata::ah_post::correct"
		// Assert storage "NominationPools::Metadata::ah_post::consistent"
		// Assert storage "NominationPools::ReversePoolIdLookup::ah_post::correct"
		// Assert storage "NominationPools::ReversePoolIdLookup::ah_post::consistent"
		// Assert storage "NominationPools::ClaimPermissions::ah_post::correct"
		// Assert storage "NominationPools::ClaimPermissions::ah_post::consistent"
		assert_eq!(
			expected_ah_messages, ah_messages,
			"Nomination pools data mismatch: Asset Hub data differs from translated Relay Chain data"
		);
	}
}
