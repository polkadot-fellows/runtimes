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

//! Genesis configs presets for the CoretimeKusama runtime

use crate::*;
use hex_literal::hex;
use sp_core::crypto::UncheckedInto;
use sp_genesis_builder::PresetId;
use sp_std::vec::Vec;
use system_parachains_constants::genesis_presets::*;

const CORETIME_KUSAMA_ED: Balance = ExistentialDeposit::get();

fn coretime_kusama_genesis(
	invulnerables: Vec<(AccountId, AuraId)>,
	endowed_accounts: Vec<AccountId>,
	id: ParaId,
) -> serde_json::Value {
	serde_json::json!({
		"balances": BalancesConfig {
			balances: endowed_accounts
				.iter()
				.cloned()
				.map(|k| (k, CORETIME_KUSAMA_ED * 4096 * 4096))
				.collect(),
		},
		"parachainInfo": ParachainInfoConfig {
			parachain_id: id,
			..Default::default()
		},
		"collatorSelection": CollatorSelectionConfig {
			invulnerables: invulnerables.iter().cloned().map(|(acc, _)| acc).collect(),
			candidacy_bond: CORETIME_KUSAMA_ED * 16,
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
			..Default::default()
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

pub fn coretime_kusama_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_kusama_genesis(invulnerables(), testnet_accounts(), para_id)
}

fn coretime_kusama_development_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_kusama_local_testnet_genesis(para_id)
}

fn coretime_kusama_live_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_kusama_genesis(
		vec![
			// HRn3a4qLmv1ejBHvEbnjaiEWjt154iFi2Wde7bXKGUwGvtL
			(
				hex!("d6a941f3a15918925170cc4e703c0beacc8915e2a04b3e86985915d2d84d2d52").into(),
				hex!("4491cfc3ef17b4e02c66a7161f34fcacabf86ad64a783c1dbbe74e4ef82a7966")
					.unchecked_into(),
			),
			// Cx9Uu2sxp3Xt1QBUbGQo7j3imTvjWJrqPF1PApDoy6UVkWP
			(
				hex!("10a59d610a39fc102624c8e8aa1096f0188f3fdd24b226c6a27eeed5b4774e12").into(),
				hex!("04e3a3ecadbd493eb64ab2c19d215ccbc9eebea686dc3cea4833194674a8285e")
					.unchecked_into(),
			),
			// CdW8izFcLeicL3zZUQaC3a39AGeNSTgc9Jb5E5sjREPryA2
			(
				hex!("026d79399d627961c528d648413b2aa54595245d97158a8b90900287dee28216").into(),
				hex!("de05506c73f35cf0bd50652b719369c2e20be9bf2c8522bd6cb61059a0cb0033")
					.unchecked_into(),
			),
			// H1tAQMm3eizGcmpAhL9aA9gR844kZpQfkU7pkmMiLx9jSzE
			(
				hex!("c46ff658221e07564fde2764017590264f9dfced3538e283856c43e0ee456e51").into(),
				hex!("786b7889aecde64fc8942c1d52e2d7220da83636275edfd467624a06ffc3c935")
					.unchecked_into(),
			),
			// J11Rp4mjz3vRb2DL51HqRGRjhuEQRyXgtuFskebXb8zMZ9s
			(
				hex!("f00168a3d082a8ccf93945b1f173fdaecc1ce76fc09bbde18423640194be7212").into(),
				hex!("0a2cee67864d1d4c9433bfd45324b8f72425f096e01041546be48c5d3bc9a746")
					.unchecked_into(),
			),
			// DtuntvQBh9vajFTnd42aTTCiuCyY3ep6EVwhhPji2ejyyhW
			(
				hex!("3a6a0745688c52b4709f65fa2e4508dfa0940ccc0d282cd16be9bc043b2f4a04").into(),
				hex!("064842b69c1e8dc6e2263dedd129d96488cae3f6953631da4ebba097c241eb23")
					.unchecked_into(),
			),
			// HmatizNhXrZtXwQK2LfntvjCy3x1EuKs1WnRQ6CP3KkNfmA
			(
				hex!("e5c49f7bc76b9e1b91566945e2eb539d960da57ca8e9ccd0e6030e4b11b60099").into(),
				hex!("7e126fa970a75ae2cd371d01ee32e9387f0b256832e408ca8ea7b254e6bcde7d")
					.unchecked_into(),
			),
			// HPUEzi4v3YJmhBfSbcGEFFiNKPAGVnGkfDiUzBNTR7j1CxT
			(
				hex!("d4e6d6256f56677bcdbc0543f1a2c40aa82497b33af1748fc10113b1e2a1b460").into(),
				hex!("cade3f02e0acf9e85d9a4f919abeaeda12b55202c74f78d506ccd1ea2e16a271")
					.unchecked_into(),
			),
		],
		Vec::new(),
		para_id,
	)
}

/// Provides the names of the predefined genesis configs for this runtime.
pub fn preset_names() -> Vec<PresetId> {
	vec![PresetId::from("development"), PresetId::from("local_testnet")]
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &sp_genesis_builder::PresetId) -> Option<sp_std::vec::Vec<u8>> {
	let patch = match id.try_into() {
		Ok("live") => coretime_kusama_live_genesis(1005.into()),
		Ok("development") => coretime_kusama_development_genesis(1005.into()),
		Ok("local_testnet") => coretime_kusama_local_testnet_genesis(1005.into()),
		_ => return None,
	};
	Some(
		serde_json::to_string(&patch)
			.expect("serialization to json is expected to work. qed.")
			.into_bytes(),
	)
}
