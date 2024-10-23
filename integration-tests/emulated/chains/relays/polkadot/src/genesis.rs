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
use authority_discovery_primitives::AuthorityId as AuthorityDiscoveryId;
use babe_primitives::AuthorityId as BabeId;
use beefy_primitives::ecdsa_crypto::AuthorityId as BeefyId;
use grandpa::AuthorityId as GrandpaId;
use sp_core::{sr25519, storage::Storage};
use sp_runtime::Perbill;

// Polkadot
use polkadot_primitives::{AssignmentId, ValidatorId};

// Cumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, get_account_id_from_seed, get_from_seed, get_host_config,
};
use parachains_common::Balance;
use polkadot_runtime_constants::currency::UNITS as DOT;

pub const ED: Balance = polkadot_runtime::ExistentialDeposit::get();
const ENDOWMENT: u128 = 1_000_000 * DOT;
const STASH: u128 = 100 * DOT;

mod validators {
	use super::*;
	use parachains_common::AccountId;

	#[allow(clippy::type_complexity)]
	pub fn initial_authorities() -> Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)> {
		let seed = "Alice";
		vec![(
			get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
			get_account_id_from_seed::<sr25519::Public>(seed),
			get_from_seed::<BabeId>(seed),
			get_from_seed::<GrandpaId>(seed),
			get_from_seed::<ValidatorId>(seed),
			get_from_seed::<AssignmentId>(seed),
			get_from_seed::<AuthorityDiscoveryId>(seed),
			get_from_seed::<BeefyId>(seed),
		)]
	}
}

fn session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	para_validator: ValidatorId,
	para_assignment: AssignmentId,
	authority_discovery: AuthorityDiscoveryId,
	beefy: BeefyId,
) -> polkadot_runtime::SessionKeys {
	polkadot_runtime::SessionKeys {
		babe,
		grandpa,
		para_validator,
		para_assignment,
		authority_discovery,
		beefy,
	}
}

pub fn genesis() -> Storage {
	let genesis_config = polkadot_runtime::RuntimeGenesisConfig {
		system: polkadot_runtime::SystemConfig::default(),
		balances: polkadot_runtime::BalancesConfig {
			balances: accounts::init_balances().iter().cloned().map(|k| (k, ENDOWMENT)).collect(),
		},
		session: polkadot_runtime::SessionConfig {
			keys: validators::initial_authorities()
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
			..Default::default()
		},
		staking: polkadot_runtime::StakingConfig {
			validator_count: validators::initial_authorities().len() as u32,
			minimum_validator_count: 1,
			stakers: validators::initial_authorities()
				.iter()
				.map(|x| (x.0.clone(), x.1.clone(), STASH, pallet_staking::StakerStatus::Validator))
				.collect(),
			invulnerables: validators::initial_authorities().iter().map(|x| x.0.clone()).collect(),
			force_era: pallet_staking::Forcing::ForceNone,
			slash_reward_fraction: Perbill::from_percent(10),
			..Default::default()
		},
		babe: polkadot_runtime::BabeConfig {
			authorities: Default::default(),
			epoch_config: polkadot_runtime::BABE_GENESIS_EPOCH_CONFIG,
			..Default::default()
		},
		configuration: polkadot_runtime::ConfigurationConfig { config: get_host_config() },
		..Default::default()
	};

	build_genesis_storage(&genesis_config, polkadot_runtime::WASM_BINARY.unwrap())
}
