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
use cumulus_primitives_core::ParaId;
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators, SAFE_XCM_VERSION,
};
use parachains_common::Balance;
use polkadot_runtime_constants::currency::UNITS as DOT;

const ENDOWMENT: u128 = 1_000 * DOT;
pub const PARA_ID: u32 = 1004;
pub const ED: Balance = people_polkadot_runtime::ExistentialDeposit::get();

pub fn genesis() -> Storage {
	let genesis_config = people_polkadot_runtime::RuntimeGenesisConfig {
		balances: people_polkadot_runtime::BalancesConfig {
			balances: accounts::init_balances().iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
		},
		system: people_polkadot_runtime::SystemConfig::default(),
		parachain_info: people_polkadot_runtime::ParachainInfoConfig {
			parachain_id: ParaId::from(PARA_ID),
			..Default::default()
		},
		collator_selection: people_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: people_polkadot_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                   // account id
						acc,                                           // validator id
						people_polkadot_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			..Default::default()
		},
		polkadot_xcm: people_polkadot_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		people_polkadot_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}
