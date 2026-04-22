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

//! # Bulletin Polkadot Runtime genesis config presets

use crate::*;
use alloc::{vec, vec::Vec};
use cumulus_primitives_core::ParaId;
use frame_support::build_struct_json_patch;
use hex_literal::hex;
use parachains_common::{AccountId, AuraId};
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_genesis_builder::PresetId;
use system_parachains_constants::{genesis_presets::*, polkadot::currency::UNITS as DOT};

const BULLETIN_POLKADOT_ED: Balance = ExistentialDeposit::get();
pub const BULLETIN_PARA_ID: ParaId = ParaId::new(1010);

fn bulletin_polkadot_live_genesis(id: ParaId) -> serde_json::Value {
	bulletin_polkadot_genesis(
		vec![
			// Faraday Nodes
			// 155wHcqJ3fcfgtHsjqKHwNEU24pzRkkmZK865xxHeFTXMU8T
			(
				hex!("b4b4a8fdf51911242d0a860640fa8952f4005d0c4542428b4bcfb9815fc6ec55").into(),
				hex!("4e91cfd5145fea6ebd1d1a441b33797e9c19918fa017291c73e56d2590566778")
					.unchecked_into(),
			),
			// yaron
			// 1sXuddoUew7f9F9XTVyns8KjCRRLvpvvUsZUyxZhqtH4RZn
			(
				hex!("268a505e81484de28108b814d0ab5ea13b947d153c19ee26bd280bd886e57815").into(),
				hex!("80d6667f725e501088c081ff924dbe1aa50c67618b0984cb996c3d5fa5500f0f")
					.unchecked_into(),
			),
			// dapestake
			// 1A1WrKowzJD4yQQcETugEV5UWoNo1o7ujuA3f1fBfpxPjZL
			(
				hex!("06def0ef07d9b5153276dd785525839706f4696c8cb86227a2af27fd7495ee63").into(),
				hex!("5e9659d151a03a5902e3135c9e361855f6d1caaea6e53a7d8613d7ad410bf507")
					.unchecked_into(),
			),
		],
		Vec::new(),
		0,
		id,
	)
}

fn bulletin_polkadot_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	endowment: Balance,
	id: ParaId,
) -> serde_json::Value {
	build_struct_json_patch!(RuntimeGenesisConfig {
		balances: BalancesConfig {
			balances: endowed_accounts.iter().cloned().map(|k| (k, endowment)).collect(),
		},
		parachain_info: ParachainInfoConfig { parachain_id: id },
		collator_selection: CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: BULLETIN_POLKADOT_ED * 16,
		},
		session: SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),          // account id
						acc,                  // validator id
						SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		polkadot_xcm: PolkadotXcmConfig { safe_xcm_version: Some(SAFE_XCM_VERSION) },
	})
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
	let patch = match id.as_ref() {
		"live" => bulletin_polkadot_live_genesis(BULLETIN_PARA_ID),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => bulletin_polkadot_genesis(
			// initial collators.
			invulnerables(),
			testnet_accounts(),
			DOT * 1_000_000,
			BULLETIN_PARA_ID,
		),
		sp_genesis_builder::DEV_RUNTIME_PRESET => bulletin_polkadot_genesis(
			// initial collators.
			vec![(
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_from_seed::<AuraId>("Alice"),
			)],
			vec![
				get_account_id_from_seed::<sr25519::Public>("Alice"),
				get_account_id_from_seed::<sr25519::Public>("Bob"),
				get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
				get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
			],
			DOT * 1_000_000,
			BULLETIN_PARA_ID,
		),
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
		PresetId::from("live"),
		PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
		PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
	]
}
