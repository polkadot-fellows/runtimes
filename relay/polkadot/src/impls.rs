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

use super::*;
use core::marker::PhantomData;
use frame_support::traits::{tokens::ConversionFromAssetBalance, Contains};
use polkadot_primitives::Id as ParaId;
use xcm_builder::IsChildSystemParachain;

// TODO: replace by types from polkadot-sdk https://github.com/paritytech/polkadot-sdk/pull/3659
/// Determines if the given `asset_kind` is a native asset. If it is, returns the balance without
/// conversion; otherwise, delegates to the implementation specified by `I`.
///
/// Example where the `asset_kind` represents the native asset:
/// - location: (1, Parachain(1000)), // location of a Sibling Parachain;
/// - asset_id: (1, Here), // the asset id in the context of `asset_kind.location`;
pub struct NativeOnSystemParachain<I>(PhantomData<I>);
impl<I> ConversionFromAssetBalance<Balance, VersionedLocatableAsset, Balance>
	for NativeOnSystemParachain<I>
where
	I: ConversionFromAssetBalance<Balance, VersionedLocatableAsset, Balance>,
{
	type Error = ();
	fn from_asset_balance(
		balance: Balance,
		asset_kind: VersionedLocatableAsset,
	) -> Result<Balance, Self::Error> {
		use VersionedLocatableAsset::*;
		let (location, asset_id) = match asset_kind.clone() {
			V3 { location, asset_id } => (location.try_into()?, asset_id.try_into()?),
			V4 { location, asset_id } => (location, asset_id),
		};
		if asset_id.0.contains_parents_only(1) &&
			IsChildSystemParachain::<ParaId>::contains(&location)
		{
			Ok(balance)
		} else {
			I::from_asset_balance(balance, asset_kind).map_err(|_| ())
		}
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(asset_kind: VersionedLocatableAsset) {
		I::ensure_successful(asset_kind)
	}
}

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarks {
	use super::{xcm_config::CheckAccount, ExistentialDeposit};
	use crate::Balances;
	use frame_support::{
		dispatch::RawOrigin,
		traits::{Currency, EnsureOrigin},
	};

	pub struct InitializeReaperForBenchmarking<A, E>(core::marker::PhantomData<(A, E)>);
	impl<A, O: Into<Result<RawOrigin<A>, O>> + From<RawOrigin<A>>, E: EnsureOrigin<O>>
		EnsureOrigin<O> for InitializeReaperForBenchmarking<A, E>
	{
		type Success = E::Success;

		fn try_origin(o: O) -> Result<E::Success, O> {
			E::try_origin(o)
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			// initialize the XCM Check Account with the existential deposit
			Balances::make_free_balance_be(&CheckAccount::get(), ExistentialDeposit::get());

			// call the real implementation
			E::try_successful_origin()
		}
	}
}
