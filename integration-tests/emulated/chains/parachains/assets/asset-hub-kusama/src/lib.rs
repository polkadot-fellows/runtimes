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

pub mod genesis;

// Substrate
use frame_support::traits::OnInitialize;

// Cumulus
use emulated_integration_tests_common::{
	impl_accounts_helpers_for_parachain, impl_assert_events_helpers_for_parachain,
	impl_assets_helpers_for_parachain, impl_assets_helpers_for_system_parachain,
	impl_foreign_assets_helpers_for_parachain, impl_xcm_helpers_for_parachain, impls::Parachain,
	xcm_emulator::decl_test_parachains,
};
use kusama_emulated_chain::Kusama;

pub use assets_common::local_and_foreign_assets::ForeignAssetReserveData;

// AssetHubKusama Parachain declaration
decl_test_parachains! {
	pub struct AssetHubKusama {
		genesis = genesis::genesis(),
		on_init = {
			asset_hub_kusama_runtime::AuraExt::on_initialize(1);
		},
		runtime = asset_hub_kusama_runtime,
		core = {
			XcmpMessageHandler: asset_hub_kusama_runtime::XcmpQueue,
			LocationToAccountId: asset_hub_kusama_runtime::xcm_config::LocationToAccountId,
			ParachainInfo: asset_hub_kusama_runtime::ParachainInfo,
			MessageOrigin: cumulus_primitives_core::AggregateMessageOrigin,
		},
		pallets = {
			PolkadotXcm: asset_hub_kusama_runtime::PolkadotXcm,
			Assets: asset_hub_kusama_runtime::Assets,
			ForeignAssets: asset_hub_kusama_runtime::ForeignAssets,
			PoolAssets: asset_hub_kusama_runtime::PoolAssets,
			AssetConversion: asset_hub_kusama_runtime::AssetConversion,
			Balances: asset_hub_kusama_runtime::Balances,
		}
	},
}

// AssetHubKusama implementation
impl_accounts_helpers_for_parachain!(AssetHubKusama);
impl_assert_events_helpers_for_parachain!(AssetHubKusama);
impl_assets_helpers_for_system_parachain!(AssetHubKusama, Kusama);
impl_assets_helpers_for_parachain!(AssetHubKusama);
impl_foreign_assets_helpers_for_parachain!(
	AssetHubKusama,
	xcm::latest::Location,
	ForeignAssetReserveData
);
impl_xcm_helpers_for_parachain!(AssetHubKusama);
