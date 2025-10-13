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
use system_parachains_constants::genesis_presets::*;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

const ENCOINTER_KUSAMA_ED: Balance = ExistentialDeposit::get();

fn encointer_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENCOINTER_KUSAMA_ED * 4096))
				.collect(),
			dev_accounts: None,
		},
		"parachainInfo": ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ENCOINTER_KUSAMA_ED * 16,
			..Default::default()
		},
		"session": SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                         // account id
						acc,                                 // validator id
						SessionKeys { aura }, 			// session keys
					)
				})
				.collect(),
			non_authority_keys: vec![],
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"encointerScheduler": {
			"currentPhase": CeremonyPhaseType::Registering,
			"currentCeremonyIndex": 1,
			"phaseDurations": vec![
				(CeremonyPhaseType::Registering, 604800000u64), // 7d
				(CeremonyPhaseType::Assigning, 86400000u64),    // 1d
				(CeremonyPhaseType::Attesting, 172800000u64),   // 2d
			],
		},
		"encointerCeremonies": {
			"ceremonyReward": BalanceType::from_num(1),
			"timeTolerance": 600_000u64,   // +-10min
			"locationTolerance": 1_000, // [m]
			"endorsementTicketsPerBootstrapper": 10,
			"endorsementTicketsPerReputable": 5,
			"reputationLifetime": 5,
			"inactivityTimeout": 5, // idle ceremonies before purging community
			"meetupTimeOffset": 0,
		},
		"encointerCommunities": {
			"minSolarTripTimeS": 1, // [s]
			"maxSpeedMps": 1,         // [m/s] suggested would be 83m/s for security,
		},
		"encointerBalances": {
			// for relative adjustment.
			"feeConversionFactor": 7_143u32,
		},
		"encointerFaucet": {
			"reserveAmount": 10_000_000_000_000u128,
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

pub fn encointer_kusama_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	encointer_kusama_genesis(invulnerables(), testnet_accounts(), para_id)
}

fn encointer_kusama_development_genesis(para_id: ParaId) -> serde_json::Value {
	encointer_kusama_genesis(
		invulnerables(),
		testnet_accounts_with([
			// Make sure `StakingPot` is funded for benchmarking purposes.
			StakingPot::get(),
		]),
		para_id,
	)
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => encointer_kusama_development_genesis(1001.into()),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => {
			encointer_kusama_local_testnet_genesis(1001.into())
		},
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
