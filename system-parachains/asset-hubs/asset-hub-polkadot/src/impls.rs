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

// TODO: the types in the module copied from the PR: https://github.com/paritytech/polkadot-sdk/pull/3250
// and should be removed when changes from the PR will get released.
pub(crate) mod pool {
	use super::*;
	use core::marker::PhantomData;
	use pallet_asset_conversion::PoolLocator;
	use sp_core::Get;
	use sp_runtime::traits::{TrailingZeroInput, TryConvert};

	/// Pool locator that mandates the inclusion of the specified `FirstAsset` in every asset pair.
	///
	/// The `PoolId` is represented as a tuple of `AssetKind`s with `FirstAsset` always positioned
	/// as the first element.
	pub struct WithFirstAsset<FirstAsset, AccountId, AssetKind, AccountIdConverter>(
		PhantomData<(FirstAsset, AccountId, AssetKind, AccountIdConverter)>,
	);
	impl<FirstAsset, AccountId, AssetKind, AccountIdConverter>
		PoolLocator<AccountId, AssetKind, (AssetKind, AssetKind)>
		for WithFirstAsset<FirstAsset, AccountId, AssetKind, AccountIdConverter>
	where
		AssetKind: Eq + Clone + Encode,
		AccountId: Decode,
		FirstAsset: Get<AssetKind>,
		AccountIdConverter: for<'a> TryConvert<&'a (AssetKind, AssetKind), AccountId>,
	{
		fn pool_id(asset1: &AssetKind, asset2: &AssetKind) -> Result<(AssetKind, AssetKind), ()> {
			let first = FirstAsset::get();
			match true {
				_ if asset1 == asset2 => Err(()),
				_ if first == *asset1 => Ok((first, asset2.clone())),
				_ if first == *asset2 => Ok((first, asset1.clone())),
				_ => Err(()),
			}
		}
		fn address(id: &(AssetKind, AssetKind)) -> Result<AccountId, ()> {
			AccountIdConverter::try_convert(id).map_err(|_| ())
		}
	}

	/// `PoolId` to `AccountId` conversion.
	pub struct AccountIdConverter<Seed, PoolId>(PhantomData<(Seed, PoolId)>);
	impl<Seed, PoolId, AccountId> TryConvert<&PoolId, AccountId> for AccountIdConverter<Seed, PoolId>
	where
		PoolId: Encode,
		AccountId: Decode,
		Seed: Get<PalletId>,
	{
		fn try_convert(id: &PoolId) -> Result<AccountId, &PoolId> {
			let encoded = sp_io::hashing::blake2_256(&Encode::encode(&(Seed::get(), id))[..]);
			Decode::decode(&mut TrailingZeroInput::new(encoded.as_ref())).map_err(|_| id)
		}
	}
}
