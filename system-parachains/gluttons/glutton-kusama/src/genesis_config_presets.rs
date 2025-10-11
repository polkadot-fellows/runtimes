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

fn glutton_kusama_genesis(id: ParaId) -> serde_json::Value {
	serde_json::json!({
		"parachainInfo": ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
	})
}

pub fn glutton_kusama_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	glutton_kusama_genesis(para_id)
}

fn glutton_kusama_development_genesis(para_id: ParaId) -> serde_json::Value {
	glutton_kusama_local_testnet_genesis(para_id)
}

/// Provides the names of the predefined genesis configs for this runtime.
pub fn preset_names() -> Vec<PresetId> {
	vec![
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		sp_genesis_builder::DEV_RUNTIME_PRESET => glutton_kusama_development_genesis(1300.into()),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => {
			glutton_kusama_local_testnet_genesis(1300.into())
		},
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
