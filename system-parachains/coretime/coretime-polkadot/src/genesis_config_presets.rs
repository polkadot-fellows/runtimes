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

//! Genesis configs presets for the Polkadot Coretime runtime

use crate::*;
use sp_core::sr25519;
use sp_std::vec::Vec;
use system_parachains_constants::genesis_presets::*;

const CORETIME_POLKADOT_ED: Balance = ExistentialDeposit::get();

fn coretime_polkadot_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, CORETIME_POLKADOT_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: CORETIME_POLKADOT_ED * 16,
			..Default::default()
		},
		"session": SessionConfig {
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
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

pub fn coretime_polkadot_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_polkadot_genesis(invulnerables(), testnet_accounts(), para_id)
}

fn coretime_polkadot_development_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_polkadot_local_testnet_genesis(para_id)
}

fn coretime_polkadot_live_invulnerables() -> Vec<(parachains_common::AccountId, AuraId)> {
	Vec::from([
		(get_account_id_from_seed::<sr25519::Public>("Alice"), get_from_seed::<AuraId>("Alice")),
		(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
	])
}

pub fn coretime_polkadot_live_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_polkadot_genesis(coretime_polkadot_live_invulnerables(), Vec::new(), para_id)
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<sp_std::vec::Vec<u8>> {
	let patch = match id.try_into() {
		Ok("live") => coretime_polkadot_live_genesis(1005.into()),
		Ok("development") => coretime_polkadot_development_genesis(1005.into()),
		Ok("local_testnet") => coretime_polkadot_local_testnet_genesis(1005.into()),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
