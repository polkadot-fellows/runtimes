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

use hex_literal::hex;
use sp_runtime::AccountId32;
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;

type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;

pub struct AccountTranslationWorks;

pub const TRANSLATIONS: &[(AccountId32, AccountId32)] = &[
	// para 2034: 5Ec4AhPbkXX97KXMcf9v9SkRNG4Gyc3VhcMMuQe9QXfAHnrC -> 5Eg2fntQqFi3EvFWAf71G66Ecjjah26bmFzoANAeHFgj9Lia
	(AccountId32::new(hex!("70617261f2070000000000000000000000000000000000000000000000000000")), AccountId32::new(hex!("7369626cf2070000000000000000000000000000000000000000000000000000"))),
];

impl RcMigrationCheck for AccountTranslationWorks {
	type RcPrePayload = ();

	fn pre_check() -> Self::RcPrePayload {
		for (rc_acc, ah_acc) in TRANSLATIONS.iter() {
			assert!(frame_system::Account::<RelayRuntime>::contains_key(rc_acc));
			assert!(!frame_system::Account::<RelayRuntime>::contains_key(ah_acc));
		}
	}

	fn post_check(_: Self::RcPrePayload) {
		for (rc_acc, ah_acc) in TRANSLATIONS.iter() {
			if frame_system::Account::<AssetHubRuntime>::contains_key(rc_acc) {
				panic!("RC acc should not exist on AH: {}, {:?}", rc_acc, frame_system::Account::<AssetHubRuntime>::get(rc_acc));
			}
			assert!(!frame_system::Account::<AssetHubRuntime>::contains_key(ah_acc));
		}
	}
}

impl AhMigrationCheck for AccountTranslationWorks {
	type RcPrePayload = ();
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// AH acc exists
		for (rc_acc, ah_acc) in TRANSLATIONS.iter() {
			assert!(!frame_system::Account::<AssetHubRuntime>::contains_key(rc_acc));
			assert!(frame_system::Account::<AssetHubRuntime>::contains_key(ah_acc));
		}
	}

	fn post_check(_rc_pre: Self::RcPrePayload, _: Self::AhPrePayload) {
		// AH acc exists
		for (rc_acc, ah_acc) in TRANSLATIONS.iter() {
			assert!(frame_system::Account::<AssetHubRuntime>::contains_key(ah_acc));
			println!("AH acc: {}, {:?}", ah_acc, frame_system::Account::<AssetHubRuntime>::get(ah_acc));
			if frame_system::Account::<AssetHubRuntime>::contains_key(rc_acc) {
				panic!("RC acc should not exist on AH: {}, {:?}", rc_acc, frame_system::Account::<AssetHubRuntime>::get(rc_acc));
			}			
		}
	}
}
