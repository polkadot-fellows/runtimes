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

use crate::common::{get_account_id_from_seed, get_from_seed, testnet_accounts};
use cumulus_primitives_core::ParaId;
use parachains_common::{AccountId, AuraId, Balance};
use sc_chain_spec::{ChainSpec, ChainSpecExtension, ChainSpecGroup, ChainType};
use serde::{Deserialize, Serialize};
use sp_core::sr25519;

/// Generic extensions for Parachain ChainSpecs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ChainSpecGroup, ChainSpecExtension)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
	/// The relay chain of the Parachain.
	pub relay_chain: String,
	/// The id of the Parachain.
	pub para_id: u32,
}

pub type AssetHubPolkadotChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type AssetHubKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type CollectivesPolkadotChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type BridgeHubPolkadotChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type BridgeHubKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type GluttonKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type EncointerKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type CoretimeKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type PeopleKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type PeoplePolkadotChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

pub fn asset_hub_polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	Ok(Box::new(
		AssetHubPolkadotChainSpec::builder(
			asset_hub_polkadot_runtime::WASM_BINARY.expect("AssetHubPolkadot wasm not available!"),
			Extensions { relay_chain: "polkadot-local".into(), para_id: 1000 },
		)
		.with_name("Polkadot Asset Hub Local")
		.with_id("asset-hub-polkadot-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(asset_hub_polkadot_runtime::genesis_config_presets::asset_hub_polkadot_local_testnet_genesis(1000.into()))
		.with_properties(properties)
		.build(),
	))
}

pub fn asset_hub_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	Ok(Box::new(
		AssetHubKusamaChainSpec::builder(
			asset_hub_kusama_runtime::WASM_BINARY.expect("AssetHubKusama wasm not available!"),
			Extensions { relay_chain: "kusama-local".into(), para_id: 1000 },
		)
		.with_name("Kusama Asset Hub Local")
		.with_id("asset-hub-kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(asset_hub_kusama_runtime::genesis_config_presets::asset_hub_kusama_local_testnet_genesis(1000.into()))
		.with_properties(properties)
		.build(),
	))
}

pub fn collectives_polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	Ok(Box::new(
		CollectivesPolkadotChainSpec::builder(
			collectives_polkadot_runtime::WASM_BINARY
				.expect("CollectivesPolkadot wasm not available!"),
			Extensions { relay_chain: "polkadot-local".into(), para_id: 1001 },
		)
		.with_name("Polkadot Collectives Local")
		.with_id("collectives-polkadot-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(collectives_polkadot_runtime::genesis_config_presets::collectives_polkadot_local_testnet_genesis(1001.into()))
		.with_properties(properties)
		.build(),
	))
}

pub fn bridge_hub_polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	Ok(Box::new(
		BridgeHubPolkadotChainSpec::builder(
			bridge_hub_polkadot_runtime::WASM_BINARY
				.expect("BridgeHubPolkadot wasm not available!"),
			Extensions { relay_chain: "polkadot-local".into(), para_id: 1002 },
		)
		.with_name("Polkadot Bridge Hub Local")
		.with_id("bridge-hub-polkadot-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(bridge_hub_polkadot_runtime::genesis_config_presets::bridge_hub_polkadot_local_testnet_genesis(1002.into()))
		.with_properties(properties)
		.build(),
	))
}

pub fn bridge_hub_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	Ok(Box::new(
		BridgeHubKusamaChainSpec::builder(
			bridge_hub_kusama_runtime::WASM_BINARY.expect("BridgeHubKusama wasm not available!"),
			Extensions { relay_chain: "kusama-local".into(), para_id: 1002 },
		)
		.with_name("Kusama Bridge Hub Local")
		.with_id("bridge-hub-kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(bridge_hub_kusama_runtime::genesis_config_presets::bridge_hub_kusama_local_testnet_genesis(1002.into()))
		.with_properties(properties)
		.build(),
	))
}

pub fn glutton_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());

	Ok(Box::new(
		GluttonKusamaChainSpec::builder(
			glutton_kusama_runtime::WASM_BINARY.expect("GluttonKusama wasm not available!"),
			Extensions { relay_chain: "kusama-local".into(), para_id: 1300 },
		)
		.with_name("Kusama Glutton Local")
		.with_id("glutton-kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(
			glutton_kusama_runtime::genesis_config_presets::glutton_kusama_local_testnet_genesis(
				1300.into(),
			),
		)
		.with_properties(properties)
		.build(),
	))
}

pub fn encointer_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	Ok(Box::new(
		EncointerKusamaChainSpec::builder(
			encointer_kusama_runtime::WASM_BINARY.expect("EncointerKusama wasm not available!"),
			Extensions { relay_chain: "kusama-local".into(), para_id: 1001 },
		)
		.with_name("Kusama Encointer Local")
		.with_id("encointer-kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(encointer_kusama_runtime::genesis_config_presets::encointer_kusama_local_testnet_genesis(1001))
		.with_properties(properties)
		.build(),
	))
}

pub fn coretime_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	Ok(Box::new(
		CoretimeKusamaChainSpec::builder(
			coretime_kusama_runtime::WASM_BINARY.expect("CoretimeKusama wasm not available!"),
			Extensions { relay_chain: "kusama-local".into(), para_id: 1005 },
		)
		.with_name("Kusama Coretime Local")
		.with_id("coretime-kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(
			coretime_kusama_runtime::genesis_config_presets::coretime_kusama_local_testnet_genesis(
				1005.into(),
			),
		)
		.with_properties(properties)
		.build(),
	))
}

pub fn people_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	Ok(Box::new(
		PeopleKusamaChainSpec::builder(
			people_kusama_runtime::WASM_BINARY.expect("PeopleKusama wasm not available!"),
			Extensions { relay_chain: "kusama-local".into(), para_id: 1004 },
		)
		.with_name("Kusama People Local")
		.with_id("people-kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(
			people_kusama_runtime::genesis_config_presets::people_kusama_local_testnet_genesis(
				1004.into(),
			),
		)
		.with_properties(properties)
		.build(),
	))
}

pub fn people_polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	Ok(Box::new(
		PeoplePolkadotChainSpec::builder(
			people_polkadot_runtime::WASM_BINARY.expect("PeoplePolkadot wasm not available!"),
			Extensions { relay_chain: "polkadot-local".into(), para_id: 1004 },
		)
		.with_name("Polkadot People Local")
		.with_id("people-polkadot-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(
			people_polkadot_runtime::genesis_config_presets::people_polkadot_local_testnet_genesis(
				1004.into(),
			),
		)
		.with_properties(properties)
		.build(),
	))
}
