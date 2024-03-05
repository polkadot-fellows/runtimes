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

//! Module with configuration which reflects AssetHubKusama runtime setup.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use scale_info::TypeInfo;
use xcm::prelude::*;

pub use bp_xcm_bridge_hub_router::XcmBridgeHubRouterCall;

/// `AssetHubKusama` Runtime `Call` enum.
///
/// The enum represents a subset of possible `Call`s we can send to `AssetHubKusama` chain.
/// Ideally this code would be auto-generated from metadata, because we want to
/// avoid depending directly on the ENTIRE runtime just to get the encoding of `Dispatchable`s.
///
/// All entries here (like pretty much in the entire file) must be kept in sync with
/// `AssetHubKusama` `construct_runtime`, so that we maintain SCALE-compatibility.
#[allow(clippy::large_enum_variant)]
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone, TypeInfo)]
pub enum Call {
	/// `ToPolkadotXcmRouter` bridge pallet.
	#[codec(index = 34)]
	ToPolkadotXcmRouter(XcmBridgeHubRouterCall),
}

frame_support::parameter_types! {
	/// Some sane weight to execute `xcm::Transact(pallet-xcm-bridge-hub-router::Call::report_bridge_status)`.
	pub const XcmBridgeHubRouterTransactCallMaxWeight: Weight = Weight::from_parts(200_000_000, 6144);

	/// Message that is sent to the sibling Kusama Asset Hub when the with-Polkadot bridge becomes congested.
	pub CongestedMessage: Xcm<()> = build_congestion_message(true).into();
	/// Message that is sent to the sibling Kusama Asset Hub when the with-Polkadot bridge becomes uncongested.
	pub UncongestedMessage: Xcm<()> = build_congestion_message(false).into();
}

fn build_congestion_message(is_congested: bool) -> sp_std::vec::Vec<Instruction<()>> {
	sp_std::vec![
		UnpaidExecution { weight_limit: Unlimited, check_origin: None },
		Transact {
			origin_kind: OriginKind::Xcm,
			require_weight_at_most: XcmBridgeHubRouterTransactCallMaxWeight::get(),
			call: Call::ToPolkadotXcmRouter(XcmBridgeHubRouterCall::report_bridge_status {
				bridge_id: Default::default(),
				is_congested,
			})
			.encode()
			.into(),
		}
	]
}
