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

use super::*;

use frame_support::parameter_types;
use frame_system::{EnsureNever, EnsureRoot};
use xcm::latest::prelude::*;

parameter_types! {
	pub const AssetDeposit: Balance = UNITS;
	pub const AssetAccountDeposit: Balance = system_para_deposit(1, 16);
	pub const ApprovalDeposit: Balance = SYSTEM_PARA_EXISTENTIAL_DEPOSIT;
	pub const AssetsStringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = system_para_deposit(1, 68);
	pub const MetadataDepositPerByte: Balance = system_para_deposit(0, 1);
}

impl pallet_assets::Config for Runtime {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type AssetId = Location;
	type AssetIdParameter = Location;
	type Currency = Balances;
	// Assets can only be force created by root.
	type CreateOrigin = EnsureNever<AccountId>;
	type ForceOrigin = EnsureRoot<AccountId>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = AssetsStringLimit;
	type Holder = ();
	type Freezer = ();
	type Extra = ();
	type WeightInfo = pallet_assets::weights::SubstrateWeight<Runtime>;
	type CallbackHandle = ();
	type AssetAccountDeposit = AssetAccountDeposit;
	type RemoveItemsLimit = frame_support::traits::ConstU32<1000>;
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = xcm_config::XcmBenchmarkHelper;
}

/// Module that holds everything related to the HOLLAR asset.
pub mod hollar {
	use super::*;
	use frame_support::traits::ContainsPair;

	/// The parachain id of the Hydration DEX.
	pub const HYDRATION_PARA_ID: u32 = 2034;

	/// The asset id of HOLLAR.
	pub const HOLLAR_ASSET_ID: u128 = 222;

	/// A unit of HOLLAR consists of 10^18 plancks.
	pub const HOLLAR_UNITS: u128 = 1_000_000_000_000_000_000u128;

	parameter_types! {
		pub HydrationLocation: Location = Location::new(1, [Parachain(HYDRATION_PARA_ID)]);
		pub HollarId: AssetId = AssetId(Location::new(1, [Parachain(HYDRATION_PARA_ID), GeneralIndex(HOLLAR_ASSET_ID)]));
		pub Hollar: Asset = (HollarId::get(), 10 * HOLLAR_UNITS).into();
	}

	/// A type that matches the pair `(Hollar, Hydration)`,
	/// used in the XCM	configuration's `IsReserve`.
	pub struct HollarFromHydration;
	impl ContainsPair<Asset, Location> for HollarFromHydration {
		fn contains(asset: &Asset, origin: &Location) -> bool {
			let is_hydration = matches!(origin.unpack(), (1, [Parachain(para_id)]) if *para_id == HYDRATION_PARA_ID);
			let is_hollar = matches!(
				asset.id.0.unpack(),
				(1, [Parachain(para_id), GeneralIndex(asset_id)])
				if *para_id == HYDRATION_PARA_ID && *asset_id == HOLLAR_ASSET_ID
			);

			is_hydration && is_hollar
		}
	}
}
