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

//! Genesis configs presets for the AssetHubKusama runtime

use crate::{xcm_config::UniversalLocation, *};
use alloc::vec::Vec;
use sp_genesis_builder::PresetId;
use system_parachains_constants::genesis_presets::*;
use xcm::latest::prelude::*;
use xcm_builder::GlobalConsensusConvertsFor;
use xcm_executor::traits::ConvertLocation;

const ASSET_HUB_KUSAMA_ED: Balance = ExistentialDeposit::get();

fn asset_hub_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
	foreign_assets: Vec<(Location, AccountId, Balance)>,
	foreign_assets_endowed_accounts: Vec<(Location, AccountId, Balance)>,
) -> serde_json::Value {
	serde_json::json!({
		"balances": BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, ASSET_HUB_KUSAMA_ED * 4096 * 4096))
				.collect(),
			dev_accounts: None,
		},
		"parachainInfo": ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: ASSET_HUB_KUSAMA_ED * 16,
			..Default::default()
		},
		"session": SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                         // account id
						acc,                                 // validator id
						SessionKeys { aura }, 	// session keys
					)
				})
				.collect(),
			..Default::default()
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"staking": {
			"validatorCount": 1000,
			"devStakers": Some((4_000, 15_000)),
		},
		"foreignAssets": ForeignAssetsConfig {
			assets: foreign_assets
				.into_iter()
				.map(|asset| (asset.0, asset.1, false, asset.2))
				.collect(),
			accounts: foreign_assets_endowed_accounts
				.into_iter()
				.map(|asset| (asset.0, asset.1, asset.2))
				.collect(),
			..Default::default()
		},
		"revive": ReviveConfig {
			mapped_accounts: endowed_accounts.iter().filter(|x| ! pallet_revive::is_eth_derived(x)).cloned().collect(),
			accounts: Vec::new(),
			debug_settings: None,
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

pub fn asset_hub_kusama_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	asset_hub_kusama_genesis(
		invulnerables(),
		testnet_accounts(),
		para_id,
		vec![
			// bridged DOT
			(
				Location::new(2, [GlobalConsensus(Polkadot)]),
				GlobalConsensusConvertsFor::<UniversalLocation, AccountId>::convert_location(
					&Location { parents: 2, interior: [GlobalConsensus(Polkadot)].into() },
				)
				.unwrap(),
				10000000,
			),
		],
		vec![
			// bridged DOT to Bob
			(
				Location::new(2, [GlobalConsensus(Polkadot)]),
				get_account_id_from_seed::<sp_core::sr25519::Public>("Bob"),
				10000000 * 4096 * 4096,
			),
		],
	)
}

fn asset_hub_kusama_development_genesis(para_id: ParaId) -> serde_json::Value {
	asset_hub_kusama_genesis(
		invulnerables(),
		testnet_accounts_with([
			// Make sure `StakingPot` is funded for benchmarking purposes.
			StakingPot::get(),
		]),
		para_id,
		vec![],
		vec![],
	)
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
		sp_genesis_builder::DEV_RUNTIME_PRESET => asset_hub_kusama_development_genesis(1000.into()),
		sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET =>
			asset_hub_kusama_local_testnet_genesis(1000.into()),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
