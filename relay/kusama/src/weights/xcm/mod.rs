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

mod pallet_xcm_benchmarks_fungible;
mod pallet_xcm_benchmarks_generic;

use crate::Runtime;
use alloc::vec::Vec;
use frame_election_provider_support::BoundedVec;
use pallet_xcm_benchmarks_fungible::WeightInfo as XcmBalancesWeight;
use pallet_xcm_benchmarks_generic::WeightInfo as XcmGeneric;
use xcm::{
	latest::{prelude::*, AssetTransferFilter},
	DoubleEncoded,
};

/// Types of asset supported by the Kusama runtime.
pub enum AssetTypes {
	/// An asset backed by `pallet-balances`.
	Balances,
	/// Unknown asset.
	Unknown,
}

impl From<&Asset> for AssetTypes {
	fn from(asset: &Asset) -> Self {
		match asset {
			Asset { id: AssetId(Location { parents: 0, interior: Here }), .. } =>
				AssetTypes::Balances,
			_ => AssetTypes::Unknown,
		}
	}
}

trait WeighAssets {
	fn weigh_assets(&self, balances_weight: Weight) -> Weight;
}

// Kusama only knows about one asset, the balances pallet.
const MAX_ASSETS: u64 = 1;

impl WeighAssets for AssetFilter {
	fn weigh_assets(&self, balances_weight: Weight) -> Weight {
		match self {
			Self::Definite(assets) => assets
				.inner()
				.iter()
				.map(From::from)
				.map(|t| match t {
					AssetTypes::Balances => balances_weight,
					AssetTypes::Unknown => Weight::MAX,
				})
				.fold(Weight::zero(), |acc, x| acc.saturating_add(x)),
			// We don't support any NFTs on Kusama, so these two variants will always match
			// only 1 kind of fungible asset.
			Self::Wild(AllOf { .. } | AllOfCounted { .. }) => balances_weight,
			Self::Wild(AllCounted(count)) =>
				balances_weight.saturating_mul(MAX_ASSETS.min((*count as u64).max(1))),
			Self::Wild(All) => balances_weight.saturating_mul(MAX_ASSETS),
		}
	}
}

impl WeighAssets for Assets {
	fn weigh_assets(&self, balances_weight: Weight) -> Weight {
		self.inner()
			.iter()
			.map(<AssetTypes as From<&Asset>>::from)
			.map(|t| match t {
				AssetTypes::Balances => balances_weight,
				AssetTypes::Unknown => Weight::MAX,
			})
			.fold(Weight::zero(), |acc, x| acc.saturating_add(x))
	}
}

pub struct KusamaXcmWeight<RuntimeCall>(core::marker::PhantomData<RuntimeCall>);
impl<RuntimeCall> XcmWeightInfo<RuntimeCall> for KusamaXcmWeight<RuntimeCall> {
	fn withdraw_asset(assets: &Assets) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::withdraw_asset())
	}
	fn reserve_asset_deposited(assets: &Assets) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::reserve_asset_deposited())
	}
	fn receive_teleported_asset(assets: &Assets) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::receive_teleported_asset())
	}
	fn query_response(
		_query_id: &u64,
		_response: &Response,
		_max_weight: &Weight,
		_querier: &Option<Location>,
	) -> Weight {
		XcmGeneric::<Runtime>::query_response()
	}
	fn transfer_asset(assets: &Assets, _dest: &Location) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::transfer_asset())
	}
	fn transfer_reserve_asset(assets: &Assets, _dest: &Location, _xcm: &Xcm<()>) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::transfer_reserve_asset())
	}
	fn transact(
		_origin_kind: &OriginKind,
		_fallback_max_weight: &Option<Weight>,
		_call: &DoubleEncoded<RuntimeCall>,
	) -> Weight {
		XcmGeneric::<Runtime>::transact()
	}
	fn hrmp_new_channel_open_request(
		_sender: &u32,
		_max_message_size: &u32,
		_max_capacity: &u32,
	) -> Weight {
		// XCM Executor does not currently support HRMP channel operations
		Weight::MAX
	}
	fn hrmp_channel_accepted(_recipient: &u32) -> Weight {
		// XCM Executor does not currently support HRMP channel operations
		Weight::MAX
	}
	fn hrmp_channel_closing(_initiator: &u32, _sender: &u32, _recipient: &u32) -> Weight {
		// XCM Executor does not currently support HRMP channel operations
		Weight::MAX
	}
	fn clear_origin() -> Weight {
		XcmGeneric::<Runtime>::clear_origin()
	}
	fn descend_origin(_who: &InteriorLocation) -> Weight {
		XcmGeneric::<Runtime>::descend_origin()
	}
	fn report_error(_query_response_info: &QueryResponseInfo) -> Weight {
		XcmGeneric::<Runtime>::report_error()
	}

	fn deposit_asset(assets: &AssetFilter, _dest: &Location) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::deposit_asset())
	}
	fn deposit_reserve_asset(assets: &AssetFilter, _dest: &Location, _xcm: &Xcm<()>) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::deposit_reserve_asset())
	}
	fn exchange_asset(_give: &AssetFilter, _receive: &Assets, _maximal: &bool) -> Weight {
		// Kusama does not currently support exchange asset operations
		Weight::MAX
	}
	fn initiate_reserve_withdraw(
		assets: &AssetFilter,
		_reserve: &Location,
		_xcm: &Xcm<()>,
	) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::initiate_reserve_withdraw())
	}
	fn initiate_teleport(assets: &AssetFilter, _dest: &Location, _xcm: &Xcm<()>) -> Weight {
		assets.weigh_assets(XcmBalancesWeight::<Runtime>::initiate_teleport())
	}
	fn report_holding(_response_info: &QueryResponseInfo, _assets: &AssetFilter) -> Weight {
		XcmGeneric::<Runtime>::report_holding()
	}
	fn buy_execution(_fees: &Asset, _weight_limit: &WeightLimit) -> Weight {
		XcmGeneric::<Runtime>::buy_execution()
	}
	fn refund_surplus() -> Weight {
		XcmGeneric::<Runtime>::refund_surplus()
	}
	fn set_error_handler(_xcm: &Xcm<RuntimeCall>) -> Weight {
		XcmGeneric::<Runtime>::set_error_handler()
	}
	fn set_appendix(_xcm: &Xcm<RuntimeCall>) -> Weight {
		XcmGeneric::<Runtime>::set_appendix()
	}
	fn clear_error() -> Weight {
		XcmGeneric::<Runtime>::clear_error()
	}
	fn claim_asset(_assets: &Assets, _ticket: &Location) -> Weight {
		XcmGeneric::<Runtime>::claim_asset()
	}
	fn trap(_code: &u64) -> Weight {
		XcmGeneric::<Runtime>::trap()
	}
	fn subscribe_version(_query_id: &QueryId, _max_response_weight: &Weight) -> Weight {
		XcmGeneric::<Runtime>::subscribe_version()
	}
	fn unsubscribe_version() -> Weight {
		XcmGeneric::<Runtime>::unsubscribe_version()
	}
	fn burn_asset(assets: &Assets) -> Weight {
		assets.weigh_assets(XcmGeneric::<Runtime>::burn_asset())
	}
	fn expect_asset(assets: &Assets) -> Weight {
		assets.weigh_assets(XcmGeneric::<Runtime>::expect_asset())
	}
	fn expect_origin(_origin: &Option<Location>) -> Weight {
		XcmGeneric::<Runtime>::expect_origin()
	}
	fn expect_error(_error: &Option<(u32, XcmError)>) -> Weight {
		XcmGeneric::<Runtime>::expect_error()
	}
	fn expect_transact_status(_transact_status: &MaybeErrorCode) -> Weight {
		XcmGeneric::<Runtime>::expect_transact_status()
	}
	fn query_pallet(_module_name: &Vec<u8>, _response_info: &QueryResponseInfo) -> Weight {
		XcmGeneric::<Runtime>::query_pallet()
	}
	fn expect_pallet(
		_index: &u32,
		_name: &Vec<u8>,
		_module_name: &Vec<u8>,
		_crate_major: &u32,
		_min_crate_minor: &u32,
	) -> Weight {
		XcmGeneric::<Runtime>::expect_pallet()
	}
	fn report_transact_status(_response_info: &QueryResponseInfo) -> Weight {
		XcmGeneric::<Runtime>::report_transact_status()
	}
	fn clear_transact_status() -> Weight {
		XcmGeneric::<Runtime>::clear_transact_status()
	}
	fn universal_origin(_: &Junction) -> Weight {
		// Kusama does not currently support universal origin operations
		Weight::MAX
	}
	fn export_message(_: &NetworkId, _: &Junctions, _: &Xcm<()>) -> Weight {
		// Kusama relay should not support export message operations
		Weight::MAX
	}
	fn lock_asset(_: &Asset, _: &Location) -> Weight {
		// Kusama does not currently support asset locking operations
		Weight::MAX
	}
	fn unlock_asset(_: &Asset, _: &Location) -> Weight {
		// Kusama does not currently support asset locking operations
		Weight::MAX
	}
	fn note_unlockable(_: &Asset, _: &Location) -> Weight {
		// Kusama does not currently support asset locking operations
		Weight::MAX
	}
	fn request_unlock(_: &Asset, _: &Location) -> Weight {
		// Kusama does not currently support asset locking operations
		Weight::MAX
	}
	fn set_fees_mode(_: &bool) -> Weight {
		XcmGeneric::<Runtime>::set_fees_mode()
	}
	fn set_topic(_topic: &[u8; 32]) -> Weight {
		XcmGeneric::<Runtime>::set_topic()
	}
	fn clear_topic() -> Weight {
		XcmGeneric::<Runtime>::clear_topic()
	}
	fn alias_origin(_: &Location) -> Weight {
		XcmGeneric::<Runtime>::alias_origin()
	}
	fn unpaid_execution(_: &WeightLimit, _: &Option<Location>) -> Weight {
		XcmGeneric::<Runtime>::unpaid_execution()
	}
	fn pay_fees(_asset: &Asset) -> Weight {
		XcmGeneric::<Runtime>::pay_fees()
	}
	fn initiate_transfer(
		_dest: &Location,
		remote_fees: &Option<AssetTransferFilter>,
		_preserve_origin: &bool,
		assets: &BoundedVec<AssetTransferFilter, MaxAssetTransferFilters>,
		_xcm: &Xcm<()>,
	) -> Weight {
		let base_weight = XcmBalancesWeight::<Runtime>::initiate_transfer();
		let mut weight = if let Some(remote_fees) = remote_fees {
			let fees = remote_fees.inner();
			fees.weigh_assets(base_weight)
		} else {
			base_weight
		};

		for asset_filter in assets {
			let assets = asset_filter.inner();
			let extra = assets.weigh_assets(XcmBalancesWeight::<Runtime>::initiate_transfer());
			weight = weight.saturating_add(extra);
		}
		weight
	}
	fn execute_with_origin(
		_descendant_origin: &Option<InteriorLocation>,
		_xcm: &Xcm<RuntimeCall>,
	) -> Weight {
		XcmGeneric::<Runtime>::execute_with_origin()
	}
	fn set_hints(hints: &BoundedVec<Hint, HintNumVariants>) -> Weight {
		let mut weight = Weight::zero();
		for hint in hints {
			match hint {
				AssetClaimer { .. } => {
					weight = weight.saturating_add(XcmGeneric::<Runtime>::asset_claimer());
				},
			}
		}
		weight
	}
}

#[test]
fn all_counted_has_a_sane_weight_upper_limit() {
	let assets = AssetFilter::Wild(AllCounted(4294967295));
	let weight = Weight::from_parts(1000, 1000);

	assert_eq!(assets.weigh_assets(weight), weight * MAX_ASSETS);
}
