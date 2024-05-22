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
use parachains_common::{AccountId, AssetHubPolkadotAuraId, AuraId, Balance};
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

pub type AssetHubPolkadotChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type AssetHubKusamaChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type CollectivesPolkadotChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type BridgeHubPolkadotChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type BridgeHubKusamaChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type GluttonKusamaChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type EncointerKusamaChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type CoretimeKusamaChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

pub type PeopleKusamaChainSpec = sc_chain_spec::GenericChainSpec<(), Extensions>;

const ASSET_HUB_POLKADOT_ED: Balance = asset_hub_polkadot_runtime::ExistentialDeposit::get();

const ASSET_HUB_KUSAMA_ED: Balance = asset_hub_kusama_runtime::ExistentialDeposit::get();

const COLLECTIVES_POLKADOT_ED: Balance = collectives_polkadot_runtime::ExistentialDeposit::get();

const BRIDGE_HUB_POLKADOT_ED: Balance = bridge_hub_polkadot_runtime::ExistentialDeposit::get();

const BRIDGE_HUB_KUSAMA_ED: Balance = bridge_hub_kusama_runtime::ExistentialDeposit::get();

const ENCOINTER_KUSAMA_ED: Balance = encointer_kusama_runtime::ExistentialDeposit::get();

const CORETIME_KUSAMA_ED: Balance = coretime_kusama_runtime::ExistentialDeposit::get();

const PEOPLE_KUSAMA_ED: Balance = people_kusama_runtime::ExistentialDeposit::get();

/// The default XCM version to set in genesis config.
const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;

/// Invulnerable Collators
pub fn invulnerables() -> Vec<(AccountId, AuraId)> {
	vec![
		(get_account_id_from_seed::<sr25519::Public>("Alice"), get_from_seed::<AuraId>("Alice")),
		(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
	]
}

/// Invulnerable Collators for the particular case of AssetHubPolkadot
pub fn invulnerables_asset_hub_polkadot() -> Vec<(AccountId, AssetHubPolkadotAuraId)> {
	vec![
		(
			get_account_id_from_seed::<sr25519::Public>("Alice"),
			get_from_seed::<AssetHubPolkadotAuraId>("Alice"),
		),
		(
			get_account_id_from_seed::<sr25519::Public>("Bob"),
			get_from_seed::<AssetHubPolkadotAuraId>("Bob"),
		),
	]
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn coretime_kusama_session_keys(keys: AuraId) -> coretime_kusama_runtime::SessionKeys {
	coretime_kusama_runtime::SessionKeys { aura: keys }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn asset_hub_polkadot_session_keys(
	keys: AssetHubPolkadotAuraId,
) -> asset_hub_polkadot_runtime::SessionKeys {
	asset_hub_polkadot_runtime::SessionKeys { aura: keys }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn asset_hub_kusama_session_keys(keys: AuraId) -> asset_hub_kusama_runtime::SessionKeys {
	asset_hub_kusama_runtime::SessionKeys { aura: keys }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn collectives_polkadot_session_keys(
	keys: AuraId,
) -> collectives_polkadot_runtime::SessionKeys {
	collectives_polkadot_runtime::SessionKeys { aura: keys }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn bridge_hub_polkadot_session_keys(keys: AuraId) -> bridge_hub_polkadot_runtime::SessionKeys {
	bridge_hub_polkadot_runtime::SessionKeys { aura: keys }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn bridge_hub_kusama_session_keys(keys: AuraId) -> bridge_hub_kusama_runtime::SessionKeys {
	bridge_hub_kusama_runtime::SessionKeys { aura: keys }
}

/// Generate the session keys from individual elements.
///
/// The input must be a tuple of individual keys (a single arg for now since we have just one key).
pub fn people_kusama_session_keys(keys: AuraId) -> people_kusama_runtime::SessionKeys {
	people_kusama_runtime::SessionKeys { aura: keys }
}

// AssetHubPolkadot
fn asset_hub_polkadot_genesis(
	invulnerables: Vec<(AccountId, AssetHubPolkadotAuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": asset_hub_polkadot_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ASSET_HUB_POLKADOT_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": asset_hub_polkadot_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": asset_hub_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ASSET_HUB_POLKADOT_ED * 16,
			..Default::default()
		},
		"session": asset_hub_polkadot_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                           // account id
						acc,                                   // validator id
						asset_hub_polkadot_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn asset_hub_polkadot_local_genesis(para_id: ParaId) -> serde_json::Value {
	asset_hub_polkadot_genesis(
		// initial collators.
		invulnerables_asset_hub_polkadot(),
		testnet_accounts(),
		para_id,
	)
}

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
		.with_genesis_config_patch(asset_hub_polkadot_local_genesis(1000.into()))
		.with_properties(properties)
		.build(),
	))
}

// AssetHubKusama
fn asset_hub_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": asset_hub_kusama_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ASSET_HUB_KUSAMA_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": asset_hub_kusama_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": asset_hub_kusama_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ASSET_HUB_KUSAMA_ED * 16,
			..Default::default()
		},
		"session": asset_hub_kusama_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                         // account id
						acc,                                 // validator id
						asset_hub_kusama_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn asset_hub_kusama_local_genesis(para_id: ParaId) -> serde_json::Value {
	asset_hub_kusama_genesis(
		// initial collators.
		invulnerables(),
		testnet_accounts(),
		para_id,
	)
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
		.with_genesis_config_patch(asset_hub_kusama_local_genesis(1000.into()))
		.with_properties(properties)
		.build(),
	))
}

// CollectivesPolkadot
fn collectives_polkadot_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": collectives_polkadot_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, COLLECTIVES_POLKADOT_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": collectives_polkadot_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": collectives_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: COLLECTIVES_POLKADOT_ED * 16,
			..Default::default()
		},
		"session": collectives_polkadot_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                             // account id
						acc,                                     // validator id
						collectives_polkadot_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn collectives_polkadot_local_genesis(para_id: ParaId) -> serde_json::Value {
	collectives_polkadot_genesis(
		// initial collators.
		invulnerables(),
		testnet_accounts(),
		para_id,
	)
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
		.with_genesis_config_patch(collectives_polkadot_local_genesis(1001.into()))
		.with_properties(properties)
		.build(),
	))
}

// BridgeHubPolkadot
fn bridge_hub_polkadot_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": bridge_hub_polkadot_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, BRIDGE_HUB_POLKADOT_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": bridge_hub_polkadot_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": bridge_hub_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: BRIDGE_HUB_POLKADOT_ED * 16,
			..Default::default()
		},
		"session": bridge_hub_polkadot_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                            // account id
						acc,                                    // validator id
						bridge_hub_polkadot_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"ethereumSystem": bridge_hub_polkadot_runtime::EthereumSystemConfig {
			para_id: id,
			asset_hub_para_id: polkadot_runtime_constants::system_parachain::ASSET_HUB_ID.into(),
			..Default::default()
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn bridge_hub_polkadot_local_genesis(para_id: ParaId) -> serde_json::Value {
	bridge_hub_polkadot_genesis(
		// initial collators.
		invulnerables(),
		testnet_accounts(),
		para_id,
	)
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
		.with_genesis_config_patch(bridge_hub_polkadot_local_genesis(1002.into()))
		.with_properties(properties)
		.build(),
	))
}

// BridgeHubKusama
fn bridge_hub_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": bridge_hub_kusama_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, BRIDGE_HUB_KUSAMA_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": bridge_hub_kusama_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": bridge_hub_kusama_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: BRIDGE_HUB_KUSAMA_ED * 16,
			..Default::default()
		},
		"session": bridge_hub_kusama_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                          // account id
						acc,                                  // validator id
						bridge_hub_kusama_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"ethereumSystem": bridge_hub_kusama_runtime::EthereumSystemConfig {
			para_id: id,
			asset_hub_para_id: kusama_runtime_constants::system_parachain::ASSET_HUB_ID.into(),
			..Default::default()
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn bridge_hub_kusama_local_genesis(para_id: ParaId) -> serde_json::Value {
	bridge_hub_kusama_genesis(
		// initial collators.
		invulnerables(),
		testnet_accounts(),
		para_id,
	)
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
		.with_genesis_config_patch(bridge_hub_kusama_local_genesis(1002.into()))
		.with_properties(properties)
		.build(),
	))
}

// GluttonKusama
fn glutton_kusama_genesis(id: ParaId) -> serde_json::Value {
	serde_json::json!({
		"parachainInfo": glutton_kusama_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
	})
}

fn glutton_kusama_local_genesis(id: ParaId) -> serde_json::Value {
	glutton_kusama_genesis(id)
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
		.with_genesis_config_patch(glutton_kusama_local_genesis(1300.into()))
		.with_properties(properties)
		.build(),
	))
}

// EncointerKusama
fn encointer_kusama_genesis(endowed_accounts: Vec<AccountId>, id: u32) -> serde_json::Value {
	serde_json::json!({
		"balances": asset_hub_kusama_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ENCOINTER_KUSAMA_ED * 4096))
				.collect(),
		},
		"parachainInfo": encointer_kusama_runtime::ParachainInfoConfig {
			parachain_id: id.into(),
			..Default::default()
		},
		"collatorSelection": encointer_kusama_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables().iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ENCOINTER_KUSAMA_ED * 16,
			..Default::default()
		},
		"session": asset_hub_kusama_runtime::SessionConfig {
			keys: invulnerables()
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                         // account id
						acc,                                 // validator id
						asset_hub_kusama_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"aura": encointer_kusama_runtime::aura_config_for_chain_spec(&["Alice"]),
	})
}

fn encointer_kusama_local_genesis(para_id: u32) -> serde_json::Value {
	encointer_kusama_genesis(
		// initial collators.
		testnet_accounts(),
		para_id,
	)
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
		.with_genesis_config_patch(encointer_kusama_local_genesis(1001))
		.with_properties(properties)
		.build(),
	))
}

// CoretimeKusama
fn coretime_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": coretime_kusama_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, CORETIME_KUSAMA_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": coretime_kusama_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": coretime_kusama_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: CORETIME_KUSAMA_ED * 16,
			..Default::default()
		},
		"session": coretime_kusama_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                         // account id
						acc,                                 // validator id
						coretime_kusama_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn coretime_kusama_local_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_kusama_genesis(
		// initial collators.
		invulnerables(),
		testnet_accounts(),
		para_id,
	)
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
		.with_genesis_config_patch(coretime_kusama_local_genesis(1005.into()))
		.with_properties(properties)
		.build(),
	))
}

// PeopleKusama
fn people_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": people_kusama_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, PEOPLE_KUSAMA_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": people_kusama_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": people_kusama_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: PEOPLE_KUSAMA_ED * 16,
			..Default::default()
		},
		"session": people_kusama_runtime::SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                         // account id
						acc,                                 // validator id
						people_kusama_session_keys(aura), // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn people_kusama_local_genesis(para_id: ParaId) -> serde_json::Value {
	people_kusama_genesis(
		// initial collators.
		invulnerables(),
		testnet_accounts(),
		para_id,
	)
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
		.with_genesis_config_patch(people_kusama_local_genesis(1004.into()))
		.with_properties(properties)
		.build(),
	))
}
