// Copyright (C) Polkadot Fellows.
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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Multi-asset bounty and child-bounty source types that derive account IDs using distinct
//! sub-account prefixes (`"mbt"` and `"mcb"`) so they do not collide with the legacy
//! single-asset bounties pallet (which uses `"bt"` and `"cb"`).

use frame_support::{traits::Get, PalletId};
use pallet_multi_asset_bounties::BountyIndex;
use sp_runtime::traits::{AccountIdConversion, Convert, TryConvert};

// TODO (issue #1071) @dhirajs0:   remove this module and use the try_convert methods from
// multi-asset-bounties pallet directly in config.

/// Derives a **multi-asset** bounty account ID from the `PalletId` and the `BountyIndex`,
/// then converts it into the corresponding bounty `Beneficiary`.
///
/// Uses the prefix `"mbt"` (multi-asset bounty) so account IDs do not collide with the
/// legacy bounties pallet, which uses `"bt"`.
///
/// # Type Parameters
/// - `Id`: The pallet ID getter
/// - `T`: The pallet configuration
/// - `C`: Converter from `T::AccountId` to `T::Beneficiary`. Use `Identity` when types are the
///   same.
/// - `I`: Instance parameter (default: `()`)
pub struct MultiAssetBountySourceFromPalletId<Id, T, C, I = ()>(
	core::marker::PhantomData<(Id, T, C, I)>,
);

impl<Id, T, C, I> TryConvert<(BountyIndex, T::AssetKind), T::Beneficiary>
	for MultiAssetBountySourceFromPalletId<Id, T, C, I>
where
	Id: Get<PalletId>,
	T: pallet_multi_asset_bounties::Config<I>,
	C: Convert<T::AccountId, T::Beneficiary>,
{
	fn try_convert(
		(parent_bounty_id, _asset_kind): (BountyIndex, T::AssetKind),
	) -> Result<T::Beneficiary, (BountyIndex, T::AssetKind)> {
		let account: T::AccountId =
			Id::get().into_sub_account_truncating(("mbt", parent_bounty_id));
		Ok(C::convert(account))
	}
}

/// Derives a **multi-asset** child-bounty account ID from the `PalletId`, the parent index,
/// and the child index, then converts it into the child-bounty `Beneficiary`.
///
/// Uses the prefix `"mcb"` (multi-asset child bounty) so account IDs do not collide with the
/// legacy child-bounties pallet, which uses `"cb"`.
///
/// # Type Parameters
/// - `Id`: The pallet ID getter
/// - `T`: The pallet configuration
/// - `C`: Converter from `T::AccountId` to `T::Beneficiary`. Use `Identity` when types are the
///   same.
/// - `I`: Instance parameter (default: `()`)
pub struct MultiAssetChildBountySourceFromPalletId<Id, T, C, I = ()>(
	core::marker::PhantomData<(Id, T, C, I)>,
);

impl<Id, T, C, I> TryConvert<(BountyIndex, BountyIndex, T::AssetKind), T::Beneficiary>
	for MultiAssetChildBountySourceFromPalletId<Id, T, C, I>
where
	Id: Get<PalletId>,
	T: pallet_multi_asset_bounties::Config<I>,
	C: Convert<T::AccountId, T::Beneficiary>,
{
	fn try_convert(
		(parent_bounty_id, child_bounty_id, _asset_kind): (BountyIndex, BountyIndex, T::AssetKind),
	) -> Result<T::Beneficiary, (BountyIndex, BountyIndex, T::AssetKind)> {
		let account: T::AccountId =
			Id::get().into_sub_account_truncating(("mcb", parent_bounty_id, child_bounty_id));
		Ok(C::convert(account))
	}
}
