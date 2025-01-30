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

// TODO FAIL-CI: Insecure unless your chain includes `PrevalidateAttests` as a `TransactionExtension`.

use crate::*;
use pallet_claims::EthereumAddress;

#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ClaimsStage<AccountId> {
	StorageValues,
	Claims(Option<EthereumAddress>),
	Vesting(Option<EthereumAddress>),
	Signing(Option<EthereumAddress>),
	Preclaims(Option<AccountId>),
	Finished
}

#[derive(
	Encode,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	RuntimeDebug,
	Clone,
	PartialEq,
	Eq,
)]
pub enum RcClaimsMessage<AccountId, Balance, BlockNumber> {
	StorageValues { total: Balance },
	Claims((EthereumAddress, Balance)),
	Vesting { who: EthereumAddress, schedule: (Balance, Balance, BlockNumber) },
	Signing((EthereumAddress, StatementKind)),
	Preclaims((AccountId, EthereumAddress)),
	Finished
}
pub type RcClaimsMessageOf<T> = RcClaimsMessage<T::AccountId, T::Balance, BlockNumberFor<T>>;

pub struct ClaimsMigrator<T> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for ClaimsMigrator<T> {
	type Key = ClaimsStage<T::AccountId>;
	type Error = Error<T>;

	fn migrate_many(
		current_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut inner_key = current_key.unwrap_or(ClaimsStage::StorageValues);
	}
}
