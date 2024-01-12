// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use integration_tests_common::{
	accounts, collators, get_account_id_from_seed, validators, SAFE_XCM_VERSION,
};

// Substrate
use beefy_primitives::ecdsa_crypto::AuthorityId as BeefyId;
use grandpa_primitives::AuthorityId as GrandpaId;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, storage::Storage};
use sp_runtime::{BuildStorage, Perbill};

// Cumulus
use parachains_common::{AccountId, Balance, BlockNumber};
use polkadot_parachain_primitives::primitives::{HeadData, ValidationCode};
use polkadot_primitives::{AssignmentId, ValidatorId};
use polkadot_runtime_parachains::{
	configuration::HostConfiguration,
	paras::{ParaGenesisArgs, ParaKind},
};
// Polkadot
pub mod polkadot {
	use super::*;
	pub const ED: Balance = polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	const STASH: u128 = 100 * polkadot_runtime_constants::currency::UNITS;

	pub fn get_host_config() -> HostConfiguration<BlockNumber> {
		HostConfiguration {
			max_upward_queue_count: 10,
			max_upward_queue_size: 51200,
			max_upward_message_size: 51200,
			max_upward_message_num_per_candidate: 10,
			max_downward_message_size: 51200,
			hrmp_sender_deposit: 100_000_000_000,
			hrmp_recipient_deposit: 100_000_000_000,
			hrmp_channel_max_capacity: 1000,
			hrmp_channel_max_message_size: 102400,
			hrmp_channel_max_total_size: 102400,
			hrmp_max_parachain_outbound_channels: 30,
			hrmp_max_parachain_inbound_channels: 30,
			..Default::default()
		}
	}

	fn session_keys(
		babe: BabeId,
		grandpa: GrandpaId,
		im_online: ImOnlineId,
		para_validator: ValidatorId,
		para_assignment: AssignmentId,
		authority_discovery: AuthorityDiscoveryId,
		beefy: BeefyId,
	) -> polkadot_runtime::SessionKeys {
		polkadot_runtime::SessionKeys {
			babe,
			grandpa,
			im_online,
			para_validator,
			para_assignment,
			authority_discovery,
			beefy,
		}
	}

	pub fn genesis() -> Storage {
		let genesis_config = polkadot_runtime::RuntimeGenesisConfig {
			system: polkadot_runtime::SystemConfig::default(),
			balances: polkadot_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096))
					.collect(),
			},
			session: polkadot_runtime::SessionConfig {
				keys: validators::initial_authorities()
					.iter()
					.map(|x| {
						(
							x.0.clone(),
							x.0.clone(),
							polkadot::session_keys(
								x.2.clone(),
								x.3.clone(),
								x.4.clone(),
								x.5.clone(),
								x.6.clone(),
								x.7.clone(),
								x.8.clone(),
							),
						)
					})
					.collect::<Vec<_>>(),
			},
			staking: polkadot_runtime::StakingConfig {
				validator_count: validators::initial_authorities().len() as u32,
				minimum_validator_count: 1,
				stakers: validators::initial_authorities()
					.iter()
					.map(|x| {
						(x.0.clone(), x.1.clone(), STASH, polkadot_runtime::StakerStatus::Validator)
					})
					.collect(),
				invulnerables: validators::initial_authorities()
					.iter()
					.map(|x| x.0.clone())
					.collect(),
				force_era: pallet_staking::Forcing::ForceNone,
				slash_reward_fraction: Perbill::from_percent(10),
				..Default::default()
			},
			babe: polkadot_runtime::BabeConfig {
				authorities: Default::default(),
				epoch_config: Some(polkadot_runtime::BABE_GENESIS_EPOCH_CONFIG),
				..Default::default()
			},
			configuration: polkadot_runtime::ConfigurationConfig { config: get_host_config() },
			paras: polkadot_runtime::ParasConfig {
				paras: vec![
					(
						asset_hub_polkadot::PARA_ID.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								asset_hub_polkadot_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
					(
						bridge_hub_polkadot::PARA_ID.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								bridge_hub_polkadot_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
					(
						penpal::PARA_ID_A.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								penpal_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
					(
						penpal::PARA_ID_B.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								penpal_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
				],
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Kusama
pub mod kusama {
	use super::*;
	pub const ED: Balance = kusama_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
	use kusama_runtime_constants::currency::UNITS as KSM;
	const ENDOWMENT: u128 = 1_000_000 * KSM;
	const STASH: u128 = 100 * KSM;

	pub fn get_host_config() -> HostConfiguration<BlockNumber> {
		HostConfiguration {
			max_upward_queue_count: 10,
			max_upward_queue_size: 51200,
			max_upward_message_size: 51200,
			max_upward_message_num_per_candidate: 10,
			max_downward_message_size: 51200,
			hrmp_sender_deposit: 5_000_000_000_000,
			hrmp_recipient_deposit: 5_000_000_000_000,
			hrmp_channel_max_capacity: 1000,
			hrmp_channel_max_message_size: 102400,
			hrmp_channel_max_total_size: 102400,
			hrmp_max_parachain_outbound_channels: 30,
			hrmp_max_parachain_inbound_channels: 30,
			..Default::default()
		}
	}

	fn session_keys(
		babe: BabeId,
		grandpa: GrandpaId,
		im_online: ImOnlineId,
		para_validator: ValidatorId,
		para_assignment: AssignmentId,
		authority_discovery: AuthorityDiscoveryId,
		beefy: BeefyId,
	) -> kusama_runtime::SessionKeys {
		kusama_runtime::SessionKeys {
			babe,
			grandpa,
			im_online,
			para_validator,
			para_assignment,
			authority_discovery,
			beefy,
		}
	}

	pub fn genesis() -> Storage {
		let genesis_config = kusama_runtime::RuntimeGenesisConfig {
			system: kusama_runtime::SystemConfig::default(),
			balances: kusama_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.map(|k: &AccountId| (k.clone(), ENDOWMENT))
					.collect(),
			},
			session: kusama_runtime::SessionConfig {
				keys: validators::initial_authorities()
					.iter()
					.map(|x| {
						(
							x.0.clone(),
							x.0.clone(),
							kusama::session_keys(
								x.2.clone(),
								x.3.clone(),
								x.4.clone(),
								x.5.clone(),
								x.6.clone(),
								x.7.clone(),
								x.8.clone(),
							),
						)
					})
					.collect::<Vec<_>>(),
			},
			staking: kusama_runtime::StakingConfig {
				validator_count: validators::initial_authorities().len() as u32,
				minimum_validator_count: 1,
				stakers: validators::initial_authorities()
					.iter()
					.map(|x| {
						(x.0.clone(), x.1.clone(), STASH, kusama_runtime::StakerStatus::Validator)
					})
					.collect(),
				invulnerables: validators::initial_authorities()
					.iter()
					.map(|x| x.0.clone())
					.collect(),
				force_era: pallet_staking::Forcing::NotForcing,
				slash_reward_fraction: Perbill::from_percent(10),
				..Default::default()
			},
			babe: kusama_runtime::BabeConfig {
				authorities: Default::default(),
				epoch_config: Some(kusama_runtime::BABE_GENESIS_EPOCH_CONFIG),
				..Default::default()
			},
			configuration: kusama_runtime::ConfigurationConfig { config: get_host_config() },
			paras: kusama_runtime::ParasConfig {
				paras: vec![
					(
						asset_hub_kusama::PARA_ID.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								asset_hub_kusama_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
					(
						bridge_hub_kusama::PARA_ID.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								bridge_hub_kusama_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
					(
						penpal::PARA_ID_A.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								penpal_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
					(
						penpal::PARA_ID_B.into(),
						ParaGenesisArgs {
							genesis_head: HeadData::default(),
							validation_code: ValidationCode(
								penpal_runtime::WASM_BINARY.unwrap().to_vec(),
							),
							para_kind: ParaKind::Parachain,
						},
					),
				],
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Asset Hub Polkadot
pub mod asset_hub_polkadot {
	use super::*;
	pub const PARA_ID: u32 = 1000;
	pub const ED: Balance = parachains_common::polkadot::currency::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let genesis_config = asset_hub_polkadot_runtime::RuntimeGenesisConfig {
			system: asset_hub_polkadot_runtime::SystemConfig::default(),
			balances: asset_hub_polkadot_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096 * 4096))
					.collect(),
			},
			parachain_info: asset_hub_polkadot_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			collator_selection: asset_hub_polkadot_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables_asset_hub_polkadot()
					.iter()
					.cloned()
					.map(|(acc, _)| acc)
					.collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: asset_hub_polkadot_runtime::SessionConfig {
				keys: collators::invulnerables_asset_hub_polkadot()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                      // account id
							acc,                                              // validator id
							asset_hub_polkadot_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			polkadot_xcm: asset_hub_polkadot_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Asset Hub Kusama
pub mod asset_hub_kusama {
	use super::*;
	pub const PARA_ID: u32 = 1000;
	pub const ED: Balance = parachains_common::kusama::currency::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let genesis_config = asset_hub_kusama_runtime::RuntimeGenesisConfig {
			system: asset_hub_kusama_runtime::SystemConfig::default(),
			balances: asset_hub_kusama_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096 * 4096))
					.collect(),
			},
			parachain_info: asset_hub_kusama_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			collator_selection: asset_hub_kusama_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables()
					.iter()
					.cloned()
					.map(|(acc, _)| acc)
					.collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: asset_hub_kusama_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                    // account id
							acc,                                            // validator id
							asset_hub_kusama_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			polkadot_xcm: asset_hub_kusama_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Bridge Hub Polkadot
pub mod bridge_hub_polkadot {
	use super::*;
	pub const PARA_ID: u32 = 1002;
	pub const ED: Balance = parachains_common::polkadot::currency::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let genesis_config = bridge_hub_polkadot_runtime::RuntimeGenesisConfig {
			system: bridge_hub_polkadot_runtime::SystemConfig::default(),
			balances: bridge_hub_polkadot_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096))
					.collect(),
			},
			parachain_info: bridge_hub_polkadot_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			collator_selection: bridge_hub_polkadot_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables()
					.iter()
					.cloned()
					.map(|(acc, _)| acc)
					.collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: bridge_hub_polkadot_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                       // account id
							acc,                                               // validator id
							bridge_hub_polkadot_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			polkadot_xcm: bridge_hub_polkadot_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Bridge Hub Kusama
pub mod bridge_hub_kusama {
	use super::*;
	pub const PARA_ID: u32 = 1002;
	pub const ED: Balance = parachains_common::kusama::currency::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let genesis_config = bridge_hub_kusama_runtime::RuntimeGenesisConfig {
			system: bridge_hub_kusama_runtime::SystemConfig::default(),
			balances: bridge_hub_kusama_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096))
					.collect(),
			},
			parachain_info: bridge_hub_kusama_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			collator_selection: bridge_hub_kusama_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables()
					.iter()
					.cloned()
					.map(|(acc, _)| acc)
					.collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: bridge_hub_kusama_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                     // account id
							acc,                                             // validator id
							bridge_hub_kusama_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			polkadot_xcm: bridge_hub_kusama_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Collectives
pub mod collectives {
	use super::*;
	pub const PARA_ID: u32 = 1001;
	pub const ED: Balance = parachains_common::polkadot::currency::EXISTENTIAL_DEPOSIT;

	pub fn genesis() -> Storage {
		let genesis_config = collectives_polkadot_runtime::RuntimeGenesisConfig {
			system: collectives_polkadot_runtime::SystemConfig::default(),
			balances: collectives_polkadot_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096))
					.collect(),
			},
			parachain_info: collectives_polkadot_runtime::ParachainInfoConfig {
				parachain_id: PARA_ID.into(),
				..Default::default()
			},
			collator_selection: collectives_polkadot_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables()
					.iter()
					.cloned()
					.map(|(acc, _)| acc)
					.collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: collectives_polkadot_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                                        // account id
							acc,                                                // validator id
							collectives_polkadot_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			polkadot_xcm: collectives_polkadot_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}

// Penpal
pub mod penpal {
	use super::*;
	pub const PARA_ID_A: u32 = 2000;
	pub const PARA_ID_B: u32 = 2001;
	pub const ED: Balance = penpal_runtime::EXISTENTIAL_DEPOSIT;

	pub fn genesis(para_id: u32) -> Storage {
		let genesis_config = penpal_runtime::RuntimeGenesisConfig {
			system: penpal_runtime::SystemConfig::default(),
			balances: penpal_runtime::BalancesConfig {
				balances: accounts::init_balances()
					.iter()
					.cloned()
					.map(|k| (k, ED * 4096 * 4096))
					.collect(),
			},
			parachain_info: penpal_runtime::ParachainInfoConfig {
				parachain_id: para_id.into(),
				..Default::default()
			},
			collator_selection: penpal_runtime::CollatorSelectionConfig {
				invulnerables: collators::invulnerables()
					.iter()
					.cloned()
					.map(|(acc, _)| acc)
					.collect(),
				candidacy_bond: ED * 16,
				..Default::default()
			},
			session: penpal_runtime::SessionConfig {
				keys: collators::invulnerables()
					.into_iter()
					.map(|(acc, aura)| {
						(
							acc.clone(),                          // account id
							acc,                                  // validator id
							penpal_runtime::SessionKeys { aura }, // session keys
						)
					})
					.collect(),
			},
			polkadot_xcm: penpal_runtime::PolkadotXcmConfig {
				safe_xcm_version: Some(SAFE_XCM_VERSION),
				..Default::default()
			},
			sudo: penpal_runtime::SudoConfig {
				key: Some(get_account_id_from_seed::<sr25519::Public>("Alice")),
			},
			..Default::default()
		};

		genesis_config.build_storage().unwrap()
	}
}
