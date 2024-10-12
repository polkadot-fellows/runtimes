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

//! Genesis configs presets for the GluttonKusama runtime

use crate::*;
use cumulus_primitives_core::ParaId;
use sp_genesis_builder::PresetId;
use system_parachains_constants::genesis_presets::remove_phantom_fields;

fn glutton_kusama_genesis(id: ParaId) -> serde_json::Value {
	let config = RuntimeGenesisConfig {
		system: Default::default(),
		parachain_system: Default::default(),
		parachain_info: ParachainInfoConfig { parachain_id: id, ..Default::default() },
		glutton: Default::default(),
		sudo: Default::default(),
	};

	let mut config_values = serde_json::to_value(config).expect("Could not build genesis config.");
	remove_phantom_fields(&mut config_values);

	config_values
}

pub fn glutton_kusama_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	glutton_kusama_genesis(para_id)
}

fn glutton_kusama_development_genesis(para_id: ParaId) -> serde_json::Value {
	glutton_kusama_local_testnet_genesis(para_id)
}

/// Provides the names of the predefined genesis configs for this runtime.
pub fn preset_names() -> Vec<PresetId> {
	vec![PresetId::from("development"), PresetId::from("local_testnet")]
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<sp_std::vec::Vec<u8>> {
	let patch = match id.try_into() {
		Ok("development") => glutton_kusama_development_genesis(1300.into()),
		Ok("local_testnet") => glutton_kusama_local_testnet_genesis(1300.into()),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
