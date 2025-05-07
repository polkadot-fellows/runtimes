// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
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

use sc_chain_spec::NoExtension;
#[cfg(any(feature = "polkadot", feature = "kusama"))]
use sc_chain_spec::{ChainSpec, ChainType};

pub type PolkadotChainSpec = sc_chain_spec::GenericChainSpec<NoExtension>;

pub type KusamaChainSpec = sc_chain_spec::GenericChainSpec<NoExtension>;

#[cfg(any(feature = "polkadot", feature = "kusama"))]
const DEFAULT_PROTOCOL_ID: &str = "dot";

/// Returns the properties for the [`PolkadotChainSpec`].
#[cfg(feature = "polkadot")]
pub fn polkadot_chain_spec_properties() -> serde_json::map::Map<String, serde_json::Value> {
	serde_json::json!({
		"tokenDecimals": 10,
	})
	.as_object()
	.expect("Map given; qed")
	.clone()
}

/// Polkadot development config (single validator Alice)
#[cfg(feature = "polkadot")]
pub fn polkadot_development_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		PolkadotChainSpec::builder(
			polkadot_runtime::WASM_BINARY.ok_or("Polkadot development wasm not available")?,
			Default::default(),
		)
		.with_name("Polakdot Development")
		.with_id("polkadot-dev")
		.with_chain_type(ChainType::Development)
		.with_genesis_config_patch(
			polkadot_runtime::genesis_config_presets::polkadot_development_config_genesis(),
		)
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.with_properties(polkadot_chain_spec_properties())
		.build(),
	))
}

/// Kusama development config (single validator Alice)
#[cfg(feature = "kusama")]
pub fn kusama_development_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		KusamaChainSpec::builder(
			kusama_runtime::WASM_BINARY.ok_or("Kusama development wasm not available")?,
			Default::default(),
		)
		.with_name("Kusama Development")
		.with_id("kusama-dev")
		.with_chain_type(ChainType::Development)
		.with_genesis_config_patch(
			kusama_runtime::genesis_config_presets::kusama_development_config_genesis(),
		)
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.build(),
	))
}

/// Polkadot local testnet config (multivalidator Alice + Bob)
#[cfg(feature = "polkadot")]
pub fn polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		PolkadotChainSpec::builder(
			polkadot_runtime::WASM_BINARY.ok_or("Polkadot development wasm not available")?,
			Default::default(),
		)
		.with_name("Polkadot Local Testnet")
		.with_id("polkadot-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(
			polkadot_runtime::genesis_config_presets::polkadot_local_testnet_genesis(),
		)
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.with_properties(polkadot_chain_spec_properties())
		.build(),
	))
}

/// Kusama local testnet config (multivalidator Alice + Bob)
#[cfg(feature = "kusama")]
pub fn kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		KusamaChainSpec::builder(
			kusama_runtime::WASM_BINARY.ok_or("Kusama development wasm not available")?,
			Default::default(),
		)
		.with_name("Kusama Local Testnet")
		.with_id("kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(
			kusama_runtime::genesis_config_presets::kusama_local_testnet_genesis(),
		)
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.build(),
	))
}
