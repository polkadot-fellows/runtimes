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
use hex_literal::hex;
use sp_core::crypto::UncheckedInto;
use sp_genesis_builder::PresetId;
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
			..Default::default()
		},
		"polkadotXcm": {
			"safeXcmVersion": Some(SAFE_XCM_VERSION),
		},
		// no need to pass anything to aura, in fact it will panic if we do. Session will take care
		// of this. `aura: Default::default()`
	})
}

fn coretime_polkadot_local_testnet_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_polkadot_genesis(invulnerables(), testnet_accounts(), para_id)
}

fn coretime_polkadot_development_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_polkadot_local_testnet_genesis(para_id)
}

fn coretime_polkadot_live_genesis(para_id: ParaId) -> serde_json::Value {
	coretime_polkadot_genesis(
		vec![
			// Parity polkadot-coretime-collator-a-0
			// 13umUoWwGb765EPzMUrMmYTcEjKfNJiNyCDwdqAvCMzteGzi
			(
				hex!("80b6f570f356fef7b891afa2e1c30fca89bc7a2cddd545fd8a173106fce3a11f").into(),
				hex!("4a69b6ec0eda668471d806db625681a147efc35a4baeacf0bca95d12d13cd942")
					.unchecked_into(),
			),
			// Parity polkadot-coretime-collator-a-1
			// 13NAwtroa2efxgtih1oscJqjxcKpWJeQF8waWPTArBewi2CQ
			(
				hex!("689e1a66fa33b75f66415021aacc4fa23f49306a3c21407748b8b2d39b4abf63").into(),
				hex!("f0d0e90c36f95605510f00a9f0821675bc0c7b70e5c8d113b0426c21d627773b")
					.unchecked_into(),
			),
			// Stakeworld.io
			// 13Jpq4n3PXXaSAbJTMmFD78mXAzs8PzgUUQd5ve8saw7HQS5
			(
				hex!("6610a5024c2a5db3d02056d4344d120ec7be283100d71a6715f09275167e4f38").into(),
				hex!("dcaa0b4c6840028f6d4fa8c460d5a7d687d1f81c9de453ef2f5ead88767fd22a")
					.unchecked_into(),
			),
			// STKD
			// 173Wc3mSdXa9ja9nv7C1z6GQHEBK4HZ9U4NGhHnvmTfJaJb
			(
				hex!("049bec59fb5fe6adea4578250578e89dd7e51ad88c7c92493d6f451c6680925c").into(),
				hex!("7283ea6b8648673305a3e06be6dd83b7bc1840081d50d4deef1ce53eba21e914")
					.unchecked_into(),
			),
			// Staker Space
			// 1k4vuCxwbNcHfsNdQ3MgTGixwvrT7wbLc2XiZj68Gru6bLM
			(
				hex!("20d8c795eef2620fba2bde74dbc36461c07998ebf600ed265b746c1e05c70606").into(),
				hex!("248dbf89d86998772b66900d78e98980ea2afc3c8fe5b93f4b38052f3018a230")
					.unchecked_into(),
			),
			// openbitlab_
			// 12iho9gjSMvF9smJjnihmn9j9Qqr3S1LFD97e8Lkcw4R6Yeb
			(
				hex!("4c0aa0240b2d7485675e52cdb283a87973652f6acb42c830a5a5faa80f7a707e").into(),
				hex!("1c346cb44aa03f8995eeee230970772d6268cd7606740f269bb4e609a01a3a15")
					.unchecked_into(),
			),
			// Math Crypto
			// 112FKz5UNxjXqe3Wowe73a8FHnR5B4R9qi2pbMaXJczGNJsx
			(
				hex!("00f379b621bd73c45c7d155d2a1fe6a04649e3ece7c7e03b70b3a6242bc7c127").into(),
				hex!("e063247ca37058db551a8d99f2f15cfede61fc796acc464a9cdce4c18f6a4659")
					.unchecked_into(),
			),
		],
		Vec::new(),
		para_id,
	)
}

pub fn preset_names() -> Vec<PresetId> {
	vec![PresetId::from("live"), PresetId::from("development"), PresetId::from("local_testnet")]
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
