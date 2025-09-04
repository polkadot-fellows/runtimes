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

// Cumulus
use emulated_integration_tests_common::{
	accounts, build_genesis_storage, collators, SAFE_XCM_VERSION,
};
use encointer_kusama_runtime::{BalanceType, CeremonyPhaseType};
use parachains_common::Balance;
use xcm::prelude::*;

pub const PARA_ID: u32 = 1001;
pub const ED: Balance = encointer_kusama_runtime::ExistentialDeposit::get();

frame_support::parameter_types! {
	pub UniversalLocation: InteriorLocation = [GlobalConsensus(Kusama), Parachain(PARA_ID)].into();
}

pub fn genesis() -> sp_core::storage::Storage {
	let genesis_config = encointer_kusama_runtime::RuntimeGenesisConfig {
		system: encointer_kusama_runtime::SystemConfig::default(),
		balances: encointer_kusama_runtime::BalancesConfig {
			balances: accounts::init_balances()
				.iter()
				.cloned()
				.map(|k| (k, ED * 4096 * 4096))
				.collect(),
			dev_accounts: None,
		},
		parachain_info: encointer_kusama_runtime::ParachainInfoConfig {
			parachain_id: PARA_ID.into(),
			..Default::default()
		},
		collator_selection: encointer_kusama_runtime::CollatorSelectionConfig {
			invulnerables: collators::invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ED * 16,
			..Default::default()
		},
		session: encointer_kusama_runtime::SessionConfig {
			keys: collators::invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                                    // account id
						acc,                                            // validator id
						encointer_kusama_runtime::SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			..Default::default()
		},
		polkadot_xcm: encointer_kusama_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		encointer_scheduler: encointer_kusama_runtime::EncointerSchedulerConfig {
			current_phase: CeremonyPhaseType::Registering,
			current_ceremony_index: 1,
			phase_durations: vec![
				(CeremonyPhaseType::Registering, 604800000u64), // 7d
				(CeremonyPhaseType::Assigning, 86400000u64),    // 1d
				(CeremonyPhaseType::Attesting, 172800000u64),   // 2d
			],
			..Default::default()
		},
		encointer_ceremonies: encointer_kusama_runtime::EncointerCeremoniesConfig {
			ceremony_reward: BalanceType::from_num(1),
			time_tolerance: 600_000u64, // +-10min
			location_tolerance: 1_000,  // [m]
			endorsement_tickets_per_bootstrapper: 10,
			endorsement_tickets_per_reputable: 5,
			reputation_lifetime: 5,
			inactivity_timeout: 5, // idle ceremonies before purging community
			meetup_time_offset: 0,
			..Default::default()
		},
		encointer_communities: encointer_kusama_runtime::EncointerCommunitiesConfig {
			min_solar_trip_time_s: 1, // [s]
			max_speed_mps: 1,         // [m/s] suggested would be 83m/s for security,
			..Default::default()
		},
		encointer_balances: encointer_kusama_runtime::EncointerBalancesConfig {
			// for relative adjustment.
			fee_conversion_factor: 7_143u128,
			..Default::default()
		},
		encointer_faucet: encointer_kusama_runtime::EncointerFaucetConfig {
			reserve_amount: 10_000_000_000_000u128,
			..Default::default()
		},
		..Default::default()
	};

	build_genesis_storage(
		&genesis_config,
		encointer_kusama_runtime::WASM_BINARY.expect("WASM binary was not built, please build it!"),
	)
}
