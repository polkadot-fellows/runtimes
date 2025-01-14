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

#![doc = include_str!("multisig.md")]

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
	type Key = (T::AccountId, [u8; 32]);
	type Error = Error<T>;

	/// Migrate until the weight is exhausted. Start at the given key.
	///
	/// Storage changes must be rolled back on error.
	fn migrate_many(
		mut last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Error<T>> {
		let mut batch = Vec::new();
		let last_raw_key = last_key
			.clone()
			.map(|(k1, k2)| aliases::Multisigs::<T>::hashed_key_for(k1, k2))
			.unwrap_or_default();
		let mut iter = aliases::Multisigs::<T>::iter_from(last_raw_key);

		loop {
			let kv = iter.next();
			let Some((k1, k2, multisig)) = kv else {
				last_key = None;
				log::info!(target: LOG_TARGET, "No more multisigs to migrate");
				break;
			};

			log::debug!("Migrating multisigs of acc {:?}", k1);

			match Self::migrate_single(k1.clone(), multisig, weight_counter) {
				Ok(ms) => batch.push(ms), // TODO continue here
				// Account does not need to be migrated
				// Not enough weight, lets try again in the next block since we made some progress.
				Err(Error::OutOfWeight) if batch.len() > 0 => break,
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

			// TODO delete old
			last_key = Some((k1, k2));
		}

		if !batch.is_empty() {
			Self::send_batch_xcm(batch)?;
		}

		Ok(last_key)
	}
}

impl<T: Config> MultisigMigrator<T> {
	fn migrate_single(
		k1: AccountIdOf<T>,
		ms: aliases::MultisigOf<T>,
		weight_counter: &mut WeightMeter,
	) -> Result<RcMultisigOf<T>, Error<T>> {
		// TODO weight
		if weight_counter.try_consume(Weight::from_all(1_000)).is_err() {
			return Err(Error::<T>::OutOfWeight);
		}

		Ok(RcMultisig { creator: ms.depositor, deposit: ms.deposit, details: Some(k1) })
	}

	/// Storage changes must be rolled back on error.
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
