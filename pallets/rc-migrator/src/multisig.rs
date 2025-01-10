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

use frame_support::traits::Currency;

extern crate alloc;
use crate::{types::*, *};
use alloc::vec::Vec;

mod aliases {
	use super::*;
	use frame_system::pallet_prelude::BlockNumberFor;
	use pallet_multisig::Timepoint;

	/// Copied from https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/multisig/src/lib.rs#L96-L111
	#[derive(
		Clone, Eq, PartialEq, Encode, Decode, Default, RuntimeDebug, TypeInfo, MaxEncodedLen,
	)]
	#[scale_info(skip_type_params(MaxApprovals))]
	pub struct Multisig<BlockNumber, Balance, AccountId, MaxApprovals>
	where
		MaxApprovals: Get<u32>,
	{
		/// The extrinsic when the multisig operation was opened.
		pub when: Timepoint<BlockNumber>,
		/// The amount held in reserve of the `depositor`, to be returned once the operation ends.
		pub deposit: Balance,
		/// The account who opened it (i.e. the first to approve it).
		pub depositor: AccountId,
		/// The approvals achieved so far, including the depositor. Always sorted.
		pub approvals: BoundedVec<AccountId, MaxApprovals>,
	}

	/// Copied from https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/multisig/src/lib.rs#L77-L78
	pub type BalanceOf<T> = <<T as pallet_multisig::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	/// Copied from https://github.com/paritytech/polkadot-sdk/blob/7c5224cb01710d0c14c87bf3463cc79e49b3e7b5/substrate/frame/multisig/src/lib.rs#L171-L180
	#[frame_support::storage_alias(pallet_name)]
	pub type Multisigs<T: pallet_multisig::Config> = StorageDoubleMap<
		pallet_multisig::Pallet<T>,
		Twox64Concat,
		<T as frame_system::Config>::AccountId,
		Blake2_128Concat,
		[u8; 32],
		Multisig<
			BlockNumberFor<T>,
			BalanceOf<T>,
			<T as frame_system::Config>::AccountId,
			<T as pallet_multisig::Config>::MaxSignatories,
		>,
	>;

	pub type MultisigOf<T> = Multisig<
		BlockNumberFor<T>,
		BalanceOf<T>,
		AccountIdOf<T>,
		<T as pallet_multisig::Config>::MaxSignatories,
	>;
}

/// A multi sig that was migrated out and is ready to be received by AH.
// NOTE I am not sure if generics here are so smart, since RC and AH *have* to put the same
// generics, otherwise it would be a bug and fail to decode. However, we can just prevent that but
// by not exposing generics... On the other hand: for Westend and Kusama it could possibly help if
// we don't hard-code all types.
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
pub struct RcMultisig<AccountId, Balance> {
	/// The creator of the multisig who placed the deposit.
	pub creator: AccountId,
	/// Amount of the deposit.
	pub deposit: Balance,
	/// Optional details field to debug. Can be `None` in prod. Contains the derived account.
	pub details: Option<AccountId>,
}

pub type RcMultisigOf<T> = RcMultisig<AccountIdOf<T>, BalanceOf<T>>;

type BalanceOf<T> =
	<<T as pallet_multisig::Config>::Currency as Currency<sp_runtime::AccountId32>>::Balance;

pub struct MultisigMigrator<T: Config> {
	_marker: sp_std::marker::PhantomData<T>,
}

impl<T: Config> PalletMigration for MultisigMigrator<T> {
	type Key = BoundedVec<u8, ConstU32<1024>>;
	type Error = Error<T>;

	/// The first storage key to migrate.
	fn first_key(_weight: &mut WeightMeter) -> Result<Option<Self::Key>, Error<T>> {
		();
		let Some((k1, k2)) = aliases::Multisigs::<T>::iter_keys().next() else {
			return Ok(None);
		};
		let encoded_key = aliases::Multisigs::<T>::hashed_key_for(k1, k2);
		let bounded_key = BoundedVec::try_from(encoded_key).defensive().map_err(|_| Error::TODO)?;
		Ok(Some(bounded_key))
	}

	/// Migrate until the weight is exhausted. Start at the given key.
	fn migrate_many(
		last_key: Self::Key,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Error<T>> {
		let mut batch = Vec::new();
		let mut iter = aliases::Multisigs::<T>::iter_from(last_key.to_vec().clone());

		let maybe_last_key = loop {
			log::debug!("Migrating multisigs from {:?}", last_key);
			let kv = iter.next();
			let Some((k1, k2, multisig)) = kv else {
				break None;
			};

			match Self::migrate_single(k1.clone(), multisig, weight_counter) {
				Ok(ms) => batch.push(ms), // TODO continue here
				// Account does not need to be migrated
				// Not enough weight, lets try again in the next block since we made some progress.
				Err(Error::OutOfWeight) if batch.len() > 0 =>
					break Some(aliases::Multisigs::<T>::hashed_key_for(k1, k2)),
				// Not enough weight and was unable to make progress, bad.
				Err(Error::OutOfWeight) if batch.len() == 0 => {
					defensive!("Not enough weight to migrate a single account");
					return Err(Error::OutOfWeight);
				},
				Err(e) => {
					defensive!("Error while migrating account");
					log::error!(target: LOG_TARGET, "Error while migrating account: {:?}", e);
					return Err(e);
				},
			}

			// TODO construct XCM
		};

		if batch.len() > 0 {
			Self::send_batch_xcm(batch)?;
		}

		let bounded_key = maybe_last_key
			.map(|k| BoundedVec::try_from(k).defensive().map_err(|_| Error::TODO))
			.transpose()?;
		Ok(bounded_key)
	}
}

impl<T: Config> MultisigMigrator<T> {
	/// WILL NOT MODIFY STORAGE IN THE ERROR CASE
	fn migrate_single(
		k1: AccountIdOf<T>,
		ms: aliases::MultisigOf<T>,
		weight_counter: &mut WeightMeter,
	) -> Result<RcMultisigOf<T>, Error<T>> {
		// TODO weight
		if weight_counter.try_consume(Weight::from_all(1_000)).is_err() {
			return Err(Error::<T>::OutOfWeight);
		}

		// TODO construct XCM

		Ok(RcMultisig { creator: ms.depositor, deposit: ms.deposit, details: Some(k1) })
	}

	fn send_batch_xcm(multisigs: Vec<RcMultisigOf<T>>) -> Result<(), Error<T>> {
		let call = types::AssetHubPalletConfig::<T>::AhmController(
			types::AhMigratorCall::<T>::ReceiveMultisigs { multisigs },
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
			return Err(Error::TODO);
		};

		Ok(())
	}
}
