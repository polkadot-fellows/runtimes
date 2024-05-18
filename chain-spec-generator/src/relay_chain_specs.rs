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

use authority_discovery_primitives::AuthorityId as AuthorityDiscoveryId;
use babe_primitives::AuthorityId as BabeId;
use beefy_primitives::ecdsa_crypto::AuthorityId as BeefyId;
use grandpa::AuthorityId as GrandpaId;
use kusama_runtime_constants::currency::UNITS as KSM;
use pallet_staking::Forcing;
use polkadot_primitives::{AccountId, AccountPublic, AssignmentId, ValidatorId};
use polkadot_runtime_constants::currency::UNITS as DOT;
use runtime_parachains::configuration::HostConfiguration;
use sc_chain_spec::{ChainSpec, ChainType, NoExtension};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{traits::IdentifyAccount, Perbill};

pub type PolkadotChainSpec = sc_chain_spec::GenericChainSpec<(), NoExtension>;

pub type KusamaChainSpec = sc_chain_spec::GenericChainSpec<(), NoExtension>;

const DEFAULT_PROTOCOL_ID: &str = "dot";

/// Returns the properties for the [`PolkadotChainSpec`].
pub fn polkadot_chain_spec_properties() -> serde_json::map::Map<String, serde_json::Value> {
	serde_json::json!({
		"tokenDecimals": 10,
	})
	.as_object()
	.expect("Map given; qed")
	.clone()
}

fn default_parachains_host_configuration() -> HostConfiguration<polkadot_primitives::BlockNumber> {
	use polkadot_primitives::{MAX_CODE_SIZE, MAX_POV_SIZE};

	runtime_parachains::configuration::HostConfiguration {
		validation_upgrade_cooldown: 2u32,
		validation_upgrade_delay: 2,
		code_retention_period: 1200,
		max_code_size: MAX_CODE_SIZE,
		max_pov_size: MAX_POV_SIZE,
		max_head_data_size: 32 * 1024,
		group_rotation_frequency: 20,
		paras_availability_period: 4,
		max_upward_queue_count: 8,
		max_upward_queue_size: 1024 * 1024,
		max_downward_message_size: 1024 * 1024,
		max_upward_message_size: 50 * 1024,
		max_upward_message_num_per_candidate: 5,
		hrmp_sender_deposit: 0,
		hrmp_recipient_deposit: 0,
		hrmp_channel_max_capacity: 8,
		hrmp_channel_max_total_size: 8 * 1024,
		hrmp_max_parachain_inbound_channels: 4,
		hrmp_channel_max_message_size: 1024 * 1024,
		hrmp_max_parachain_outbound_channels: 4,
		hrmp_max_message_num_per_candidate: 5,
		dispute_period: 6,
		no_show_slots: 2,
		n_delay_tranches: 25,
		needed_approvals: 2,
		relay_vrf_modulo_samples: 2,
		zeroth_delay_tranche_width: 0,
		minimum_validation_upgrade_delay: 5,
		..Default::default()
	}
}

fn polkadot_session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	para_validator: ValidatorId,
	para_assignment: AssignmentId,
	authority_discovery: AuthorityDiscoveryId,
	beefy: BeefyId,
) -> polkadot_runtime::SessionKeys {
	polkadot_runtime::SessionKeys {
		babe,
		grandpa,
		para_validator,
		para_assignment,
		authority_discovery,
		beefy,
	}
}

fn kusama_session_keys(
	babe: BabeId,
	grandpa: GrandpaId,
	para_validator: ValidatorId,
	para_assignment: AssignmentId,
	authority_discovery: AuthorityDiscoveryId,
	beefy: BeefyId,
) -> kusama_runtime::SessionKeys {
	kusama_runtime::SessionKeys {
		babe,
		grandpa,
		para_validator,
		para_assignment,
		authority_discovery,
		beefy,
	}
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{}", seed), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed(
	seed: &str,
) -> (
	AccountId,
	AccountId,
	BabeId,
	GrandpaId,
	ValidatorId,
	AssignmentId,
	AuthorityDiscoveryId,
	BeefyId,
) {
	let keys = get_authority_keys_from_seed_no_beefy(seed);
	(keys.0, keys.1, keys.2, keys.3, keys.4, keys.5, keys.6, get_from_seed::<BeefyId>(seed))
}

/// Helper function to generate stash, controller and session key from seed
pub fn get_authority_keys_from_seed_no_beefy(
	seed: &str,
) -> (AccountId, AccountId, BabeId, GrandpaId, ValidatorId, AssignmentId, AuthorityDiscoveryId) {
	(
		get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
		get_account_id_from_seed::<sr25519::Public>(seed),
		get_from_seed::<BabeId>(seed),
		get_from_seed::<GrandpaId>(seed),
		get_from_seed::<ValidatorId>(seed),
		get_from_seed::<AssignmentId>(seed),
		get_from_seed::<AuthorityDiscoveryId>(seed),
	)
}

fn testnet_accounts() -> Vec<AccountId> {
	vec![
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
	]
}

#[allow(clippy::type_complexity)]
pub fn polkadot_testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)>,
	_root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> serde_json::Value {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	const ENDOWMENT: u128 = 1_000_000 * DOT;
	const STASH: u128 = 100 * DOT;

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().map(|k| (k.clone(), ENDOWMENT)).collect::<Vec<_>>(),
		},
		"session": {
			"keys": initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						polkadot_session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		"staking": {
			"minimumValidatorCount": 1,
			"validatorCount": initial_authorities.len() as u32,
			"stakers": initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), STASH, polkadot_runtime::StakerStatus::<AccountId>::Validator))
				.collect::<Vec<_>>(),
			"invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			"forceEra": Forcing::NotForcing,
			"slashRewardFraction": Perbill::from_percent(10),
		},
		"babe": {
			"epochConfig": Some(polkadot_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		"configuration": {
			"config": default_parachains_host_configuration(),
		},
	})
}

#[allow(clippy::type_complexity)]
pub fn kusama_testnet_genesis(
	initial_authorities: Vec<(
		AccountId,
		AccountId,
		BabeId,
		GrandpaId,
		ValidatorId,
		AssignmentId,
		AuthorityDiscoveryId,
		BeefyId,
	)>,
	_root_key: AccountId,
	endowed_accounts: Option<Vec<AccountId>>,
) -> serde_json::Value {
	let endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(testnet_accounts);

	const ENDOWMENT: u128 = 1_000_000 * KSM;
	const STASH: u128 = 100 * KSM;

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.iter().map(|k| (k.clone(), ENDOWMENT)).collect::<Vec<_>>(),
		},
		"session": {
			"keys": initial_authorities
				.iter()
				.map(|x| {
					(
						x.0.clone(),
						x.0.clone(),
						kusama_session_keys(
							x.2.clone(),
							x.3.clone(),
							x.4.clone(),
							x.5.clone(),
							x.6.clone(),
							x.7.clone(),
						),
					)
				})
				.collect::<Vec<_>>(),
		},
		"staking": {
			"minimumValidatorCount": 1,
			"validatorCount": initial_authorities.len() as u32,
			"stakers": initial_authorities
				.iter()
				.map(|x| (x.0.clone(), x.0.clone(), STASH, kusama_runtime::StakerStatus::<AccountId>::Validator))
				.collect::<Vec<_>>(),
			"invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
			"forceEra": Forcing::NotForcing,
			"slashRewardFraction": Perbill::from_percent(10),
		},
		"babe": {
			"epochConfig": Some(kusama_runtime::BABE_GENESIS_EPOCH_CONFIG),
		},
		"configuration": {
			"config": default_parachains_host_configuration(),
		},
	})
}

fn polkadot_development_config_genesis() -> serde_json::Value {
	polkadot_testnet_genesis(
		vec![get_authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

fn kusama_development_config_genesis() -> serde_json::Value {
	kusama_testnet_genesis(
		vec![get_authority_keys_from_seed("Alice")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Polkadot development config (single validator Alice)
pub fn polkadot_development_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		PolkadotChainSpec::builder(
			polkadot_runtime::WASM_BINARY.ok_or("Polkadot development wasm not available")?,
			Default::default(),
		)
		.with_name("Polakdot Development")
		.with_id("polkadot-dev")
		.with_chain_type(ChainType::Development)
		.with_genesis_config_patch(polkadot_development_config_genesis())
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.with_properties(polkadot_chain_spec_properties())
		.build(),
	))
}

/// Kusama development config (single validator Alice)
pub fn kusama_development_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		KusamaChainSpec::builder(
			kusama_runtime::WASM_BINARY.ok_or("Kusama development wasm not available")?,
			Default::default(),
		)
		.with_name("Kusama Development")
		.with_id("kusama-dev")
		.with_chain_type(ChainType::Development)
		.with_genesis_config_patch(kusama_development_config_genesis())
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.build(),
	))
}

fn polkadot_local_testnet_genesis() -> serde_json::Value {
	polkadot_testnet_genesis(
		vec![get_authority_keys_from_seed("Alice"), get_authority_keys_from_seed("Bob")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Polkadot local testnet config (multivalidator Alice + Bob)
pub fn polkadot_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		PolkadotChainSpec::builder(
			polkadot_runtime::WASM_BINARY.ok_or("Polkadot development wasm not available")?,
			Default::default(),
		)
		.with_name("Polkadot Local Testnet")
		.with_id("polkadot-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(polkadot_local_testnet_genesis())
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.with_properties(polkadot_chain_spec_properties())
		.build(),
	))
}

fn kusama_local_testnet_genesis() -> serde_json::Value {
	kusama_testnet_genesis(
		vec![get_authority_keys_from_seed("Alice"), get_authority_keys_from_seed("Bob")],
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		None,
	)
}

/// Kusama local testnet config (multivalidator Alice + Bob)
pub fn kusama_local_testnet_config() -> Result<Box<dyn ChainSpec>, String> {
	Ok(Box::new(
		KusamaChainSpec::builder(
			kusama_runtime::WASM_BINARY.ok_or("Kusama development wasm not available")?,
			Default::default(),
		)
		.with_name("Kusama Local Testnet")
		.with_id("kusama-local")
		.with_chain_type(ChainType::Local)
		.with_genesis_config_patch(kusama_local_testnet_genesis())
		.with_protocol_id(DEFAULT_PROTOCOL_ID)
		.build(),
	))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_parachains_host_configuration_is_consistent() {
		default_parachains_host_configuration().panic_if_not_consistent();
	}
}
