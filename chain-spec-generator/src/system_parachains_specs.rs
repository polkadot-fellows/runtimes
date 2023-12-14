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

pub type AssetHubPolkadotChainSpec =
	sc_chain_spec::GenericChainSpec<asset_hub_polkadot_runtime::RuntimeGenesisConfig, Extensions>;

pub type AssetHubKusamaChainSpec =
	sc_chain_spec::GenericChainSpec<asset_hub_kusama_runtime::RuntimeGenesisConfig, Extensions>;

pub type CollectivesPolkadotChainSpec =
	sc_chain_spec::GenericChainSpec<collectives_polkadot_runtime::RuntimeGenesisConfig, Extensions>;

pub type BridgeHubPolkadotChainSpec =
	sc_chain_spec::GenericChainSpec<bridge_hub_polkadot_runtime::RuntimeGenesisConfig, Extensions>;

pub type BridgeHubKusamaChainSpec =
	sc_chain_spec::GenericChainSpec<bridge_hub_kusama_runtime::RuntimeGenesisConfig, Extensions>;

const ASSET_HUB_POLKADOT_ED: Balance = parachains_common::polkadot::currency::EXISTENTIAL_DEPOSIT;

const ASSET_HUB_KUSAMA_ED: Balance = parachains_common::kusama::currency::EXISTENTIAL_DEPOSIT;

const COLLECTIVES_POLKADOT_ED: Balance = parachains_common::polkadot::currency::EXISTENTIAL_DEPOSIT;

const BRIDGE_HUB_POLKADOT_ED: Balance = parachains_common::polkadot::currency::EXISTENTIAL_DEPOSIT;

const BRIDGE_HUB_KUSAMA_ED: Balance = parachains_common::kusama::currency::EXISTENTIAL_DEPOSIT;

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

// AssetHubPolkadot
fn asset_hub_polkadot_genesis(
	wasm_binary: &[u8],
	invulnerables: Vec<(AccountId, AssetHubPolkadotAuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> asset_hub_polkadot_runtime::RuntimeGenesisConfig {
	asset_hub_polkadot_runtime::RuntimeGenesisConfig {
		system: asset_hub_polkadot_runtime::SystemConfig {
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		balances: asset_hub_polkadot_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ASSET_HUB_POLKADOT_ED * 4096))
				.collect(),
		},
		parachain_info: asset_hub_polkadot_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		collator_selection: asset_hub_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ASSET_HUB_POLKADOT_ED * 16,
			..Default::default()
		},
		session: asset_hub_polkadot_runtime::SessionConfig {
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
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		polkadot_xcm: asset_hub_polkadot_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
	}
}

fn asset_hub_polkadot_local_genesis(
	wasm_binary: &[u8],
) -> asset_hub_polkadot_runtime::RuntimeGenesisConfig {
	asset_hub_polkadot_genesis(
		// initial collators.
		wasm_binary,
		invulnerables_asset_hub_polkadot(),
		testnet_accounts(),
		1000.into(),
	)
}

pub fn asset_hub_polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	let wasm_binary =
		asset_hub_polkadot_runtime::WASM_BINARY.ok_or("AssetHubPolkadot wasm not available")?;

	Ok(Box::new(AssetHubPolkadotChainSpec::from_genesis(
		// Name
		"Polkadot Asset Hub Local",
		// ID
		"asset-hub-polkadot-local",
		ChainType::Local,
		move || asset_hub_polkadot_local_genesis(wasm_binary),
		Vec::new(),
		None,
		None,
		None,
		Some(properties),
		Extensions { relay_chain: "polkadot-local".into(), para_id: 1000 },
	)))
}

// AssetHubKusama
fn asset_hub_kusama_genesis(
	wasm_binary: &[u8],
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> asset_hub_kusama_runtime::RuntimeGenesisConfig {
	asset_hub_kusama_runtime::RuntimeGenesisConfig {
		system: asset_hub_kusama_runtime::SystemConfig {
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		balances: asset_hub_kusama_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ASSET_HUB_KUSAMA_ED * 4096))
				.collect(),
		},
		parachain_info: asset_hub_kusama_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		collator_selection: asset_hub_kusama_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ASSET_HUB_KUSAMA_ED * 16,
			..Default::default()
		},
		session: asset_hub_kusama_runtime::SessionConfig {
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
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		polkadot_xcm: asset_hub_kusama_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
	}
}

fn asset_hub_kusama_local_genesis(
	wasm_binary: &[u8],
) -> asset_hub_kusama_runtime::RuntimeGenesisConfig {
	asset_hub_kusama_genesis(
		// initial collators.
		wasm_binary,
		invulnerables(),
		testnet_accounts(),
		1000.into(),
	)
}

pub fn asset_hub_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	let wasm_binary =
		asset_hub_kusama_runtime::WASM_BINARY.ok_or("AssetHubKusama wasm not available")?;

	Ok(Box::new(AssetHubKusamaChainSpec::from_genesis(
		// Name
		"Kusama Asset Hub Local",
		// ID
		"asset-hub-kusama-local",
		ChainType::Local,
		move || asset_hub_kusama_local_genesis(wasm_binary),
		Vec::new(),
		None,
		None,
		None,
		Some(properties),
		Extensions { relay_chain: "kusama-local".into(), para_id: 1000 },
	)))
}

// CollectivesPolkadot
fn collectives_polkadot_genesis(
	wasm_binary: &[u8],
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> collectives_polkadot_runtime::RuntimeGenesisConfig {
	collectives_polkadot_runtime::RuntimeGenesisConfig {
		system: collectives_polkadot_runtime::SystemConfig {
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		balances: collectives_polkadot_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, COLLECTIVES_POLKADOT_ED * 4096))
				.collect(),
		},
		parachain_info: collectives_polkadot_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		collator_selection: collectives_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: COLLECTIVES_POLKADOT_ED * 16,
			..Default::default()
		},
		session: collectives_polkadot_runtime::SessionConfig {
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
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		polkadot_xcm: collectives_polkadot_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
		alliance: Default::default(),
		alliance_motion: Default::default(),
	}
}

fn collectives_polkadot_local_genesis(
	wasm_binary: &[u8],
) -> collectives_polkadot_runtime::RuntimeGenesisConfig {
	collectives_polkadot_genesis(
		// initial collators.
		wasm_binary,
		invulnerables(),
		testnet_accounts(),
		1001.into(),
	)
}

pub fn collectives_polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	let wasm_binary = collectives_polkadot_runtime::WASM_BINARY
		.ok_or("CollectivesPolkadot wasm not available")?;

	Ok(Box::new(CollectivesPolkadotChainSpec::from_genesis(
		// Name
		"Polkadot Collectives Local",
		// ID
		"collectives-polkadot-local",
		ChainType::Local,
		move || collectives_polkadot_local_genesis(wasm_binary),
		Vec::new(),
		None,
		None,
		None,
		Some(properties),
		Extensions { relay_chain: "polkadot-local".into(), para_id: 1001 },
	)))
}

// BridgeHubPolkadot
fn bridge_hub_polkadot_genesis(
	wasm_binary: &[u8],
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> bridge_hub_polkadot_runtime::RuntimeGenesisConfig {
	bridge_hub_polkadot_runtime::RuntimeGenesisConfig {
		system: bridge_hub_polkadot_runtime::SystemConfig {
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		balances: bridge_hub_polkadot_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, BRIDGE_HUB_POLKADOT_ED * 4096))
				.collect(),
		},
		parachain_info: bridge_hub_polkadot_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		collator_selection: bridge_hub_polkadot_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: BRIDGE_HUB_POLKADOT_ED * 16,
			..Default::default()
		},
		session: bridge_hub_polkadot_runtime::SessionConfig {
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
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		polkadot_xcm: bridge_hub_polkadot_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
	}
}

fn bridge_hub_polkadot_local_genesis(
	wasm_binary: &[u8],
) -> bridge_hub_polkadot_runtime::RuntimeGenesisConfig {
	bridge_hub_polkadot_genesis(
		// initial collators.
		wasm_binary,
		invulnerables(),
		testnet_accounts(),
		1002.into(),
	)
}

pub fn bridge_hub_polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 0.into());
	properties.insert("tokenSymbol".into(), "DOT".into());
	properties.insert("tokenDecimals".into(), 10.into());

	let wasm_binary =
		bridge_hub_polkadot_runtime::WASM_BINARY.ok_or("BridgeHubPolkadot wasm not available")?;

	Ok(Box::new(BridgeHubPolkadotChainSpec::from_genesis(
		// Name
		"Polkadot Bridge Hub Local",
		// ID
		"bridge-hub-polkadot-local",
		ChainType::Local,
		move || bridge_hub_polkadot_local_genesis(wasm_binary),
		Vec::new(),
		None,
		None,
		None,
		Some(properties),
		Extensions { relay_chain: "polkadot-local".into(), para_id: 1002 },
	)))
}

// BridgeHubKusama
fn bridge_hub_kusama_genesis(
	wasm_binary: &[u8],
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> bridge_hub_kusama_runtime::RuntimeGenesisConfig {
	bridge_hub_kusama_runtime::RuntimeGenesisConfig {
		system: bridge_hub_kusama_runtime::SystemConfig {
			code: wasm_binary.to_vec(),
			..Default::default()
		},
		balances: bridge_hub_kusama_runtime::BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, BRIDGE_HUB_KUSAMA_ED * 4096))
				.collect(),
		},
		parachain_info: bridge_hub_kusama_runtime::ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		collator_selection: bridge_hub_kusama_runtime::CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: BRIDGE_HUB_KUSAMA_ED * 16,
			..Default::default()
		},
		session: bridge_hub_kusama_runtime::SessionConfig {
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
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this.
		aura: Default::default(),
		aura_ext: Default::default(),
		parachain_system: Default::default(),
		polkadot_xcm: bridge_hub_kusama_runtime::PolkadotXcmConfig {
			safe_xcm_version: Some(SAFE_XCM_VERSION),
			..Default::default()
		},
	}
}

fn bridge_hub_kusama_local_genesis(
	wasm_binary: &[u8],
) -> bridge_hub_kusama_runtime::RuntimeGenesisConfig {
	bridge_hub_kusama_genesis(
		// initial collators.
		wasm_binary,
		invulnerables(),
		testnet_accounts(),
		1002.into(),
	)
}

pub fn bridge_hub_kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	let mut properties = sc_chain_spec::Properties::new();
	properties.insert("ss58Format".into(), 2.into());
	properties.insert("tokenSymbol".into(), "KSM".into());
	properties.insert("tokenDecimals".into(), 12.into());

	let wasm_binary =
		bridge_hub_kusama_runtime::WASM_BINARY.ok_or("BridgeHubKusama wasm not available")?;

	Ok(Box::new(BridgeHubKusamaChainSpec::from_genesis(
		// Name
		"Kusama Bridge Hub Local",
		// ID
		"bridge-hub-kusama-local",
		ChainType::Local,
		move || bridge_hub_kusama_local_genesis(wasm_binary),
		Vec::new(),
		None,
		None,
		None,
		Some(properties),
		Extensions { relay_chain: "kusama-local".into(), para_id: 1002 },
	)))
}
