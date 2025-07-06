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
use sp_keyring::Sr25519Keyring as Keyring;

// Cumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators, xcm_emulator::ConvertLocation, RESERVABLE_ASSET_ID,
	SAFE_XCM_VERSION,
};
use frame_support::sp_runtime::traits::AccountIdConversion;
use integration_tests_helpers::common::snowbridge::{
	EthLocationXcmV4, WethLocationXcmV4, MIN_ETHER_BALANCE,
};
use parachains_common::{AccountId, Balance};
use polkadot_parachain_primitives::primitives::Sibling;
use xcm::prelude::*;
use xcm_builder::GlobalConsensusParachainConvertsFor;

pub const PARA_ID: u32 = 1000;
pub const ED: Balance = asset_hub_kusama_runtime::ExistentialDeposit::get();
pub const USDT_ID: u32 = 1984;

frame_support::parameter_types! {
	pub AssetHubKusamaAssetOwner: AccountId = Keyring::Alice.to_account_id();
	pub PenpalATeleportableAssetLocation: xcm::v4::Location
		= xcm::v4::Location::new(1, [
				xcm::v4::Junction::Parachain(penpal_emulated_chain::PARA_ID_A),
				xcm::v4::Junction::PalletInstance(penpal_emulated_chain::ASSETS_PALLET_ID),
				xcm::v4::Junction::GeneralIndex(penpal_emulated_chain::TELEPORTABLE_ASSET_ID.into()),
			]
		);
	pub UniversalLocation: InteriorLocation = [GlobalConsensus(Kusama), Parachain(PARA_ID)].into();
	pub AssetHubPolkadotLocation: Location = Location::new(2, [GlobalConsensus(Polkadot), Parachain(1000)]);
	pub PenpalASiblingSovereignAccount: AccountId = Sibling::from(penpal_emulated_chain::PARA_ID_A).into_account_truncating();
	pub AssetHubPolkadotSovereignAccount: AccountId = GlobalConsensusParachainConvertsFor::<UniversalLocation, AccountId>::convert_location(
		&AssetHubPolkadotLocation::get(),
	).unwrap();
}

pub fn genesis() -> sp_core::storage::Storage {
	let genesis_config = asset_hub_kusama_runtime::RuntimeGenesisConfig {
		system: asset_hub_kusama_runtime::SystemConfig::default(),
		balances: asset_hub_kusama_runtime::BalancesConfig {
			balances: accounts::init_balances()
				.iter()
				.cloned()
				.map(|k| (k, ED * 4096 * 4096))
				.collect(),
			dev_accounts: None,
		},
		parachain_info: asset_hub_kusama_runtime::ParachainInfoConfig {
			parachain_id: PARA_ID.into(),
			..Default::default()
		},
		collator_selection: asset_hub_kusama_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: asset_hub_kusama_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                    // account id
						acc,                                            // validator id
						asset_hub_kusama_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			..Default::default()
		},
		polkadot_xcm: asset_hub_kusama_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		assets: asset_hub_kusama_runtime::AssetsConfig {
			assets: vec![
				(RESERVABLE_ASSET_ID, AssetHubKusamaAssetOwner::get(), false, ED),
				(USDT_ID, AssetHubKusamaAssetOwner::get(), true, ED),
			],
			..Default::default()
		},
		foreign_assets: asset_hub_kusama_runtime::ForeignAssetsConfig {
			assets: vec![
				// Penpal's teleportable asset representation
				(
					PenpalATeleportableAssetLocation::get(),
					PenpalASiblingSovereignAccount::get(),
					false,
					ED,
				),
				// Ether
				(
					EthLocationXcmV4::get(),
					AssetHubPolkadotSovereignAccount::get(),
					true,
					MIN_ETHER_BALANCE,
				),
				// Weth
				(
					WethLocationXcmV4::get(),
					AssetHubPolkadotSovereignAccount::get(),
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
		asset_hub_kusama_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}
