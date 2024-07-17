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

//! Genesis configs presets for the BridgeHubPolkadot runtime

use crate::*;
use sp_std::vec::Vec;
use system_parachains_constants::genesis_presets::*;

const BRIDGE_HUB_POLKADOT_ED: Balance = ExistentialDeposit::get();

fn bridge_hub_polkadot_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, BRIDGE_HUB_POLKADOT_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: BRIDGE_HUB_POLKADOT_ED * 16,
			..Default::default()
		},
		"session": SessionConfig {
			keys: invulnerables
				.into_iter()
				.map(|(acc, aura)| {
					(
						acc.clone(),                            // account id
						acc,                                    // validator id
						SessionKeys { aura }, // session keys
					)
				})
				.collect(),
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		"ethereumSystem": EthereumSystemConfig {
			para_id: id,
			asset_hub_para_id: polkadot_runtime_constants::system_parachain::ASSET_HUB_ID.into(),
			..Default::default()
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

pub fn bridge_hub_polkadot_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	bridge_hub_polkadot_genesis(invulnerables(), testnet_accounts(), para_id)
}

fn bridge_hub_polkadot_development_genesis(para_id: ParaId) -> serde_json::Value {
	bridge_hub_polkadot_local_testnet_genesis(para_id)
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<sp_std::vec::Vec<u8>> {
	let patch = match id.try_into() {
		Ok("development") => bridge_hub_polkadot_development_genesis(1002.into()),
		Ok("local_testnet") => bridge_hub_polkadot_local_testnet_genesis(1002.into()),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
