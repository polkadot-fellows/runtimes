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

// Substrate
use sp_core::storage::Storage;

// Cumulus
use asset_hub_polkadot_runtime::xcm_config::bridging::to_ethereum::EthereumNetwork;
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, get_account_id_from_seed, get_from_seed,
	xcm_emulator::ConvertLocation, RESERVABLE_ASSET_ID, SAFE_XCM_VERSION,
};
use frame_support::sp_runtime::traits::AccountIdConversion;
use integration_tests_helpers::common::{MIN_ETHER_BALANCE, WETH};
use parachains_common::{AccountId, AssetHubPolkadotAuraId, Balance};
use polkadot_parachain_primitives::primitives::Sibling;
use snowbridge_router_primitives::inbound::GlobalConsensusEthereumConvertsFor;
use sp_core::sr25519;
use xcm::prelude::*;

pub const PARA_ID: u32 = 1000;
pub const ED: Balance = asset_hub_polkadot_runtime::ExistentialDeposit::get();
pub const USDT_ID: u32 = 1984;

frame_support::parameter_types! {
	pub AssetHubPolkadotAssetOwner: AccountId = get_account_id_from_seed::<sr25519::Public>("Alice");
	pub PenpalATeleportableAssetLocation: Location
		= Location::new(1, [
				Junction::Parachain(penpal_emulated_chain::PARA_ID_A),
				Junction::PalletInstance(penpal_emulated_chain::ASSETS_PALLET_ID),
				Junction::GeneralIndex(penpal_emulated_chain::TELEPORTABLE_ASSET_ID.into()),
			]
		);
	pub PenpalBTeleportableAssetLocation: Location
		= Location::new(1, [
				Junction::Parachain(penpal_emulated_chain::PARA_ID_B),
				Junction::PalletInstance(penpal_emulated_chain::ASSETS_PALLET_ID),
				Junction::GeneralIndex(penpal_emulated_chain::TELEPORTABLE_ASSET_ID.into()),
			]
		);
	pub PenpalASiblingSovereignAccount: AccountId = Sibling::from(penpal_emulated_chain::PARA_ID_A).into_account_truncating();
	pub PenpalBSiblingSovereignAccount: AccountId = Sibling::from(penpal_emulated_chain::PARA_ID_B).into_account_truncating();
	pub EthereumSovereignAccount: AccountId = GlobalConsensusEthereumConvertsFor::<AccountId>::convert_location(
		&Location::new(
			2,
			[GlobalConsensus(EthereumNetwork::get())],
		),
	).unwrap();
}

fn invulnerables_asset_hub_polkadot() -> Vec<(AccountId, AssetHubPolkadotAuraId)> {
	vec![
		(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AssetHubPolkadotAuraId>("Alice"),
		),
		(
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_from_seed::<AssetHubPolkadotAuraId>("Bob"),
		),
	]
}

pub fn genesis() -> Storage {
	let genesis_config = asset_hub_polkadot_runtime::RuntimeGenesisConfig {
		system: asset_hub_polkadot_runtime::SystemConfig::default(),
		balances: asset_hub_polkadot_runtime::BalancesConfig {
			balances: accounts::init_balances()
				.iter()
				.cloned()
				.map(|k| (k, ED * 4096 * 4096))
				.collect(),
		},
		parachain_info: asset_hub_polkadot_runtime::ParachainInfoConfig {
			parachain_id: PARA_ID.into(),
			..Default::default()
		},
		collator_selection: asset_hub_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables_asset_hub_polkadot()
				.iter()
				.cloned()
				.map(|(acc, _)| acc)
				.collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: asset_hub_polkadot_runtime::SessionConfig {
			keys: invulnerables_asset_hub_polkadot()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                      // account id
						acc,                                              // validator id
						asset_hub_polkadot_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			..Default::default()
		},
		polkadot_xcm: asset_hub_polkadot_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		assets: asset_hub_polkadot_runtime::AssetsConfig {
			assets: vec![
				(RESERVABLE_ASSET_ID, AssetHubPolkadotAssetOwner::get(), false, ED),
				(USDT_ID, AssetHubPolkadotAssetOwner::get(), true, ED),
			],
			..Default::default()
		},
		foreign_assets: asset_hub_polkadot_runtime::ForeignAssetsConfig {
			assets: vec![
				// Penpal's teleportable asset representation
				(
					PenpalATeleportableAssetLocation::get(),
					PenpalASiblingSovereignAccount::get(),
					false,
					ED,
				),
				(
					PenpalBTeleportableAssetLocation::get(),
					PenpalBSiblingSovereignAccount::get(),
					false,
					ED,
				),
				// Ether
				(
					Location::new(2, [GlobalConsensus(EthereumNetwork::get())]),
					EthereumSovereignAccount::get(),
					true,
					MIN_ETHER_BALANCE,
				),
				// Weth
				(
					Location::new(
						2,
						[
							GlobalConsensus(EthereumNetwork::get()),
							AccountKey20 { network: None, key: WETH.into() },
						],
					),
					EthereumSovereignAccount::get(),
					true,
					MIN_ETHER_BALANCE,
				),
			],
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		asset_hub_polkadot_runtime::WASM_BINARY
			.expect("WASM binary was not built, please build it!"),
	)
}
