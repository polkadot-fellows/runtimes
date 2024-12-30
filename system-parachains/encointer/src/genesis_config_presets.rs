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

//! Genesis configs presets for the EncointerKusama runtime
use crate::*;
use sp_genesis_builder::PresetId;
use sp_std::vec::Vec;
use system_parachains_constants::genesis_presets::*;

const ENCOINTER_KUSAMA_ED: Balance = ExistentialDeposit::get();

fn encointer_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	let config = RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENCOINTER_KUSAMA_ED * 4096))
				.collect(),
		},
		parachain_info: ParachainInfoConfig { parachain_id: id, ..Default::default() },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ENCOINTER_KUSAMA_ED * 16,
			..Default::default()
		},
		session: SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),          // account id
						acc,                  // validator id
						SessionKeys { aura }, // session keys
					)
				})
				.collect(),
			non_authority_keys: vec![],
		},
		polkadot_xcm: PolkadotXcmConfig {
			_config: Default::default(),
			safe_xcm_version: Some(SAFE_XCM_VERSION),
		},
		encointer_scheduler: EncointerSchedulerConfig {
			current_phase: CeremonyPhaseType::Registering,
			current_ceremony_index: 1,
			phase_durations: vec![
				(CeremonyPhaseType::Registering, 604800000u64), //7d
				(CeremonyPhaseType::Assigning, 86400000u64),    // 1d
				(CeremonyPhaseType::Attesting, 172800000u64),   // 2d
			],
			_config: Default::default(),
		},
		encointer_ceremonies: EncointerCeremoniesConfig {
			ceremony_reward: BalanceType::from_num(1),
			time_tolerance: 600_000u64, // +-10min
			location_tolerance: 1_000,  // [m]
			endorsement_tickets_per_bootstrapper: 10,
			endorsement_tickets_per_reputable: 5,
			reputation_lifetime: 5,
			inactivity_timeout: 5, // idle ceremonies before purging community
			meetup_time_offset: 0,
			_config: Default::default(),
		},
		encointer_communities: EncointerCommunitiesConfig {
			min_solar_trip_time_s: 1, // [s]
			max_speed_mps: 1,         // [m/s] suggested would be 83m/s for security,
			_config: Default::default(),
		},
		encointer_balances: EncointerBalancesConfig {
			fee_conversion_factor: 7_143u128,
			_config: Default::default(),
		},
		encointer_faucet: EncointerFaucetConfig {
			reserve_amount: 10_000_000_000_000u128,
			_config: Default::default(),
		},
		..Default::default()
	};

	serde_json::to_value(config).expect("Could not build genesis config.")
}

pub fn encointer_kusama_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	encointer_kusama_genesis(invulnerables(), testnet_accounts(), para_id)
}

fn encointer_kusama_development_genesis(para_id: ParaId) -> serde_json::Value {
	encointer_kusama_local_testnet_genesis(para_id)
}

/// Provides the names of the predefined genesis configs for this runtime.
pub fn preset_names() -> Vec<PresetId> {
	vec![PresetId::from("development"), PresetId::from("local_testnet")]
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<sp_std::vec::Vec<u8>> {
	let patch = match id.try_into() {
		Ok("development") => encointer_kusama_development_genesis(1001.into()),
		Ok("local_testnet") => encointer_kusama_local_testnet_genesis(1001.into()),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
