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
use frame_support::parameter_types;
use sp_keyring::Sr25519Keyring as Keyring;

// Cumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators, SAFE_XCM_VERSION,
};
use parachains_common::{AccountId, Balance};
use penpal_runtime::xcm_config::{LocalReservableFromAssetHub, RelayLocation, UsdtFromAssetHub};

// Penpal
pub const PARA_ID_A: u32 = 2000;
pub const PARA_ID_B: u32 = 2001;
pub const ED: Balance = penpal_runtime::ExistentialDeposit::get();
pub const USDT_ED: Balance = 70_000;

parameter_types! {
	pub PenpalSudoAccount: AccountId = Keyring::Alice.to_account_id();
	pub PenpalAssetOwner: AccountId = PenpalSudoAccount::get();
}

pub fn genesis(para_id: u32) -> sp_core::storage::Storage {
	let genesis_config = penpal_runtime::RuntimeGenesisConfig {
		system: penpal_runtime::SystemConfig::default(),
		balances: penpal_runtime::BalancesConfig {
			balances: accounts::init_balances().iter().cloned().map(|k| (k, ED * 4096)).collect(),
			dev_accounts: None,
		},
		parachain_info: penpal_runtime::ParachainInfoConfig {
			parachain_id: para_id.into(),
			..Default::default()
		},
		collator_selection: penpal_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: penpal_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                          // account id
						acc,                                  // validator id
						penpal_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			..Default::default()
		},
		polkadot_xcm: penpal_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		sudo: penpal_runtime::SudoConfig { key: Some(PenpalSudoAccount::get()) },
		assets: penpal_runtime::AssetsConfig {
			assets: vec![(
				penpal_runtime::xcm_config::TELEPORTABLE_ASSET_ID,
				PenpalAssetOwner::get(),
				false,
				ED,
			)],
			..Default::default()
		},
		foreign_assets: penpal_runtime::ForeignAssetsConfig {
			assets: vec![
				// Relay Native asset representation
				(RelayLocation::get(), PenpalAssetOwner::get(), true, ED),
				// Sufficient AssetHub asset representation
				(LocalReservableFromAssetHub::get(), PenpalAssetOwner::get(), true, ED),
				// USDT from AssetHub
				(UsdtFromAssetHub::get(), PenpalAssetOwner::get(), true, USDT_ED),
			],
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		penpal_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}
