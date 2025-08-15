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

use sc_chain_spec::{ChainSpecExtension, ChainSpecGroup};
use serde::{Deserialize, Serialize};

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

// pub type EncointerKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type CoretimeKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type CoretimePolkadotChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type PeopleKusamaChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

pub type PeoplePolkadotChainSpec = sc_chain_spec::GenericChainSpec<Extensions>;

#[cfg(feature = "asset-hub-polkadot")]
pub fn asset_hub_polkadot_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String>
{
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "asset-hub-kusama")]
pub fn asset_hub_kusama_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String>
{
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "collectives-polkadot")]
pub fn collectives_polkadot_local_testnet_config(
) -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "bridge-hub-polkadot")]
pub fn bridge_hub_polkadot_local_testnet_config(
) -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "bridge-hub-kusama")]
pub fn bridge_hub_kusama_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String>
{
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "glutton-kusama")]
pub fn glutton_kusama_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());

	Ok(Box::new(
		GluttonKusamaChainSpec::builder(
			glutton_kusama_runtime::WASM_BINARY.expect("GluttonKusama wasm not available!"),
			Extensions { relay_chain: "kusama-local".into(), para_id: 1300 },
		)
		.with_name("Kusama Glutton Local")
		.with_id("glutton-kusama-local")
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}
/*
#[cfg(feature = "encointer-kusama")]
pub fn encointer_kusama_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String>
{
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}
*/
#[cfg(feature = "coretime-kusama")]
pub fn coretime_kusama_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "coretime-kusama")]
pub fn coretime_kusama_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	let boot_nodes = [
		"/dns/kusama-coretime-connect-a-0.polkadot.io/tcp/30334/p2p/12D3KooWR7Biy6nPgQFhk2eYP62pAkcFA6he9RUFURTDh7ewTjpo",
		"/dns/kusama-coretime-connect-a-1.polkadot.io/tcp/30334/p2p/12D3KooWAGFiMZDF9RxdacrkenzGdo8nhfSe9EXofHc5mHeJ9vGX",
		"/dns/kusama-coretime-connect-a-0.polkadot.io/tcp/443/wss/p2p/12D3KooWR7Biy6nPgQFhk2eYP62pAkcFA6he9RUFURTDh7ewTjpo",
		"/dns/kusama-coretime-connect-a-1.polkadot.io/tcp/443/wss/p2p/12D3KooWAGFiMZDF9RxdacrkenzGdo8nhfSe9EXofHc5mHeJ9vGX",
	];

	Ok(Box::new(
		CoretimeKusamaChainSpec::builder(
			coretime_kusama_runtime::WASM_BINARY.expect("Kusama Coretime wasm not available!"),
			Extensions { relay_chain: "kusama".into(), para_id: 1005 },
		)
		.with_name("Kusama Coretime")
		.with_id("coretime-kusama")
		.with_chain_type(sc_chain_spec::ChainType::Live)
		.with_genesis_config_preset_name("live")
		.with_properties(properties)
		.with_boot_nodes(
			boot_nodes
				.iter()
				.map(|addr| {
					use std::str::FromStr;
					sc_network::config::MultiaddrWithPeerId::from_str(addr)
						.expect("Boot node address is incorrect.")
				})
				.collect(),
		)
		.build(),
	))
}

#[cfg(feature = "coretime-polkadot")]
pub fn coretime_polkadot_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String>
{
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	Ok(Box::new(
		CoretimePolkadotChainSpec::builder(
			coretime_polkadot_runtime::WASM_BINARY.expect("CoretimePolkadot wasm not available!"),
			Extensions { relay_chain: "polkadot-local".into(), para_id: 1005 },
		)
		.with_name("Polkadot Coretime Local")
		.with_id("coretime-polkadot-local")
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "coretime-polkadot")]
pub fn coretime_polkadot_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	let boot_nodes = [
		"/dns/polkadot-coretime-connect-a-0.polkadot.io/tcp/30334/p2p/12D3KooWKjnixAHbKMsPTJwGx8SrBeGEJLHA8KmKcEDYMp3YmWgR",
		"/dns/polkadot-coretime-connect-a-1.polkadot.io/tcp/30334/p2p/12D3KooWQ7B7p4DFv1jWqaKfhrZBcMmi5g8bWFnmskguLaGEmT6n",
		"/dns/polkadot-coretime-connect-a-0.polkadot.io/tcp/443/wss/p2p/12D3KooWKjnixAHbKMsPTJwGx8SrBeGEJLHA8KmKcEDYMp3YmWgR",
		"/dns/polkadot-coretime-connect-a-1.polkadot.io/tcp/443/wss/p2p/12D3KooWQ7B7p4DFv1jWqaKfhrZBcMmi5g8bWFnmskguLaGEmT6n",
		"/dns4/coretime-polkadot.boot.stake.plus/tcp/30332/wss/p2p/12D3KooWFJ2yBTKFKYwgKUjfY3F7XfaxHV8hY6fbJu5oMkpP7wZ9",
		"/dns4/coretime-polkadot.boot.stake.plus/tcp/31332/wss/p2p/12D3KooWCy5pToLafcQzPHn5kadxAftmF6Eh8ZJGPXhSeXSUDfjv",

	];

	Ok(Box::new(
		CoretimePolkadotChainSpec::builder(
			coretime_polkadot_runtime::WASM_BINARY.expect("Polkadot Coretime wasm not available!"),
			Extensions { relay_chain: "polkadot".into(), para_id: 1005 },
		)
		.with_name("Polkadot Coretime")
		.with_id("coretime-polkadot")
		.with_chain_type(sc_chain_spec::ChainType::Live)
		.with_genesis_config_preset_name("live")
		.with_properties(properties)
		.with_boot_nodes(
			boot_nodes
				.iter()
				.map(|addr| {
					use std::str::FromStr;
					sc_network::config::MultiaddrWithPeerId::from_str(addr)
						.expect("Boot node address is incorrect.")
				})
				.collect(),
		)
		.build(),
	))
}

#[cfg(feature = "people-kusama")]
pub fn people_kusama_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}

#[cfg(feature = "people-polkadot")]
pub fn people_polkadot_local_testnet_config() -> Result<Box<dyn sc_chain_spec::ChainSpec>, String> {
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
		.with_chain_type(sc_chain_spec::ChainType::Local)
		.with_genesis_config_preset_name("local_testnet")
		.with_properties(properties)
		.build(),
	))
}
