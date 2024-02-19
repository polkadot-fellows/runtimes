// Copyright Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

use parachains_common::AccountId;
use xcm::{prelude::*, DoubleEncoded};

/// Helper method to build a XCM with a `Transact` instruction and paying for its execution
pub fn xcm_transact_paid_execution(
	call: DoubleEncoded<()>,
	origin_kind: OriginKind,
	native_asset: Asset,
	beneficiary: AccountId,
) -> VersionedXcm<()> {
	let weight_limit = WeightLimit::Unlimited;
	let require_weight_at_most = Weight::from_parts(1000000000, 200000);
	let native_assets: Assets = native_asset.clone().into();

	VersionedXcm::from(Xcm(vec![
		WithdrawAsset(native_assets),
		BuyExecution { fees: native_asset, weight_limit },
		Transact { require_weight_at_most, origin_kind, call },
		RefundSurplus,
		DepositAsset {
			assets: All.into(),
			beneficiary: Location {
				parents: 0,
				interior: X1(AccountId32 { network: None, id: beneficiary.into() }),
			},
		},
	]))
}

/// Helper method to build a XCM with a `Transact` instruction without paying for its execution
pub fn xcm_transact_unpaid_execution(
	call: DoubleEncoded<()>,
	origin_kind: OriginKind,
) -> VersionedXcm<()> {
	let weight_limit = WeightLimit::Unlimited;
	let require_weight_at_most = Weight::from_parts(1000000000, 200000);
	let check_origin = None;

	VersionedXcm::from(Xcm(vec![
		UnpaidExecution { weight_limit, check_origin },
		Transact { require_weight_at_most, origin_kind, call },
	]))
}
/// Helper method to get the non-fee asset used in multiple assets transfer
pub fn non_fee_asset(assets: &Assets, fee_idx: usize) -> Option<(Location, u128)> {
	let asset = assets.inner().into_iter().enumerate().find(|a| a.0 != fee_idx)?.1.clone();
	let asset_id = match asset.id {
		AssetId(id) => id,
		_ => return None,
	};
	let asset_amount = match asset.fun {
		Fungible(amount) => amount,
		_ => return None,
	};
	Some((asset_id, asset_amount))
}
