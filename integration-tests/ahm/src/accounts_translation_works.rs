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

//! Test that account translation works (Para sovereign and derived).

#[cfg(feature = "kusama-ahm")]
use crate::porting_prelude::*;

use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::{accounts::AccountState, types::RcMigrationCheck};
use sp_application_crypto::Ss58Codec;
use sp_runtime::AccountId32;

type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;

pub struct AccountTranslationWorks;

#[cfg(not(feature = "kusama-ahm"))]
pub const TRANSLATIONS: &[(AccountId32, AccountId32)] = &[
	// para 2034: 5Ec4AhPbkXX97KXMcf9v9SkRNG4Gyc3VhcMMuQe9QXfAHnrC ->
	// 5Eg2fntQqFi3EvFWAf71G66Ecjjah26bmFzoANAeHFgj9Lia
	(
		AccountId32::new(hex_literal::hex!(
			"70617261f2070000000000000000000000000000000000000000000000000000"
		)),
		AccountId32::new(hex_literal::hex!(
			"7369626cf2070000000000000000000000000000000000000000000000000000"
		)),
	),
];

#[cfg(feature = "kusama-ahm")]
pub const TRANSLATIONS: &[(AccountId32, AccountId32)] = &[];

impl RcMigrationCheck for AccountTranslationWorks {
	type RcPrePayload = ();

	fn pre_check() -> Self::RcPrePayload {
		// RC must exist
		for (rc_acc, _) in TRANSLATIONS.iter() {
			assert!(frame_system::Account::<RelayRuntime>::contains_key(rc_acc));
		}
	}

	fn post_check(_: Self::RcPrePayload) {
		// RC acc gone
		for (rc_acc, _) in TRANSLATIONS.iter() {
			if !frame_system::Account::<RelayRuntime>::contains_key(rc_acc) {
				continue;
			}

			// If an account still exists, then it must be in the RcAccounts map
			let Some(entry) = pallet_rc_migrator::RcAccounts::<RelayRuntime>::get(rc_acc) else {
				panic!("RC acc did not properly migrate: {rc_acc}");
			};

			match entry {
				AccountState::Migrate =>
					panic!("RC acc did not properly migrate: {}", rc_acc.to_ss58check()),
				AccountState::Preserve | AccountState::Part { .. } => {
					// This is fine
				},
			}
		}
	}
}

impl AhMigrationCheck for AccountTranslationWorks {
	type RcPrePayload = ();
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {}

	fn post_check(_rc_pre: Self::RcPrePayload, _: Self::AhPrePayload) {
		// AH acc exists
		for (_, ah_acc) in TRANSLATIONS.iter() {
			assert!(frame_system::Account::<AssetHubRuntime>::contains_key(ah_acc));
		}
	}
}
