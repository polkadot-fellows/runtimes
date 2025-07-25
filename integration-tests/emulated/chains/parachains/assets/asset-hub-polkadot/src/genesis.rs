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
use sp_keyring::{Ed25519Keyring, Sr25519Keyring};

// Cumulus
use asset_hub_polkadot_runtime::xcm_config::bridging::to_ethereum::EthereumNetwork;
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, xcm_emulator::ConvertLocation, RESERVABLE_ASSET_ID,
	SAFE_XCM_VERSION,
};
use frame_support::sp_runtime::traits::AccountIdConversion;
use integration_tests_helpers::common::snowbridge::{EthLocation, WethLocation, MIN_ETHER_BALANCE};
use parachains_common::{AccountId, Balance};
use polkadot_parachain_primitives::primitives::Sibling;
use snowbridge_inbound_queue_primitives::EthereumLocationsConverterFor;
use xcm::prelude::*;

pub const PARA_ID: u32 = 1000;
pub const ED: Balance = asset_hub_polkadot_runtime::ExistentialDeposit::get();
pub const USDT_ID: u32 = 1984;

frame_support::parameter_types! {
	pub AssetHubPolkadotAssetOwner: AccountId = Sr25519Keyring::Alice.to_account_id();
	pub PenpalATeleportableAssetLocation: Location
		= Location::new(1, [
				Parachain(penpal_emulated_chain::PARA_ID_A),
				PalletInstance(penpal_emulated_chain::ASSETS_PALLET_ID),
				GeneralIndex(penpal_emulated_chain::TELEPORTABLE_ASSET_ID.into()),
			]
		);
	pub PenpalBTeleportableAssetLocation: Location
		= Location::new(1, [
				Parachain(penpal_emulated_chain::PARA_ID_B),
				PalletInstance(penpal_emulated_chain::ASSETS_PALLET_ID),
				GeneralIndex(penpal_emulated_chain::TELEPORTABLE_ASSET_ID.into()),
			]
		);
	pub PenpalASiblingSovereignAccount: AccountId = Sibling::from(penpal_emulated_chain::PARA_ID_A).into_account_truncating();
	pub PenpalBSiblingSovereignAccount: AccountId = Sibling::from(penpal_emulated_chain::PARA_ID_B).into_account_truncating();
	pub EthereumSovereignAccount: AccountId = EthereumLocationsConverterFor::<AccountId>::convert_location(
		&Location::new(
			2,
			[GlobalConsensus(EthereumNetwork::get())],
		),
	).unwrap();
}

pub mod collators {
	use super::*;

	pub use emulated_integration_tests_common::collators::invulnerables;
	use parachains_common::AssetHubPolkadotAuraId;

	pub fn session_keys() -> Vec<(AccountId, AssetHubPolkadotAuraId)> {
		vec![
			(Sr25519Keyring::Alice.to_account_id(), Ed25519Keyring::Alice.public().into()),
			(Sr25519Keyring::Bob.to_account_id(), Ed25519Keyring::Bob.public().into()),
		]
	}
}

pub fn genesis() -> sp_core::storage::Storage {
	let genesis_config = asset_hub_polkadot_runtime::RuntimeGenesisConfig {
		system: asset_hub_polkadot_runtime::SystemConfig::default(),
		balances: asset_hub_polkadot_runtime::BalancesConfig {
			balances: accounts::init_balances()
				.iter()
				.cloned()
				.map(|k| (k, ED * 4096 * 4096))
				.collect(),
			dev_accounts: None,
		},
		parachain_info: asset_hub_polkadot_runtime::ParachainInfoConfig {
			parachain_id: PARA_ID.into(),
			..Default::default()
		},
		collator_selection: asset_hub_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: asset_hub_polkadot_runtime::SessionConfig {
			keys: collators::session_keys()
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
				(EthLocation::get(), EthereumSovereignAccount::get(), true, MIN_ETHER_BALANCE),
				// Weth
				(WethLocation::get(), EthereumSovereignAccount::get(), true, MIN_ETHER_BALANCE),
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
