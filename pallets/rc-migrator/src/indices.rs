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

// TODO FAIL-CI: Insecure unless your chain includes `PrevalidateAttests` as a
// `TransactionExtension`.

use crate::{types::AccountIdOf, *};
use frame_support::traits::Currency;

pub struct IndicesMigrator<T> {
	_marker: sp_std::marker::PhantomData<T>,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug, Clone, PartialEq, Eq)]
pub struct RcIndicesIndex<AccountIndex, AccountId, Balance> {
	pub index: AccountIndex,
	pub who: AccountId,
	pub deposit: Balance,
	pub frozen: bool,
}
pub type RcIndicesIndexOf<T> =
	RcIndicesIndex<<T as pallet_indices::Config>::AccountIndex, AccountIdOf<T>, BalanceOf<T>>;

type BalanceOf<T> = <<T as pallet_indices::Config>::Currency as Currency<
	<T as frame_system::Config>::AccountId,
>>::Balance;

impl<T: Config> PalletMigration for IndicesMigrator<T> {
	type Key = <T as pallet_indices::Config>::AccountIndex;
	type Error = Error<T>;

	fn migrate_many(
		current_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut inner_key = current_key;
		let mut messages = Vec::new();

		loop {
			if weight_counter
				.try_consume(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))
				.is_err()
			{
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > 10_000 {
				log::warn!("Weight allowed very big batch, stopping");
				break;
			}

			let mut iter = match inner_key {
				Some(inner_key) => pallet_indices::Accounts::<T>::iter_from_key(inner_key),
				None => pallet_indices::Accounts::<T>::iter(),
			};

			match iter.next() {
				Some((index, (who, deposit, frozen))) => {
					pallet_indices::Accounts::<T>::remove(&index);
					messages.push(RcIndicesIndex { index, who, deposit, frozen });
					inner_key = Some(index);
				},
				None => {
					inner_key = None;
					break;
				},
			}
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm(messages, |batch| {
				types::AhMigratorCall::<T>::ReceiveIndices { indices: batch }
			})?;
		}

		Ok(inner_key)
	}
}
