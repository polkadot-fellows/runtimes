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

//! Sanity account balance check of some whales.

#[cfg(feature = "kusama-ahm")]
use crate::porting_prelude::*;

use crate::porting_prelude::RC_DOLLARS;
use hex_literal::hex;
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;
use sp_runtime::AccountId32;

type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;

/// Whale accounts that we care about and minimal total resulting balance.
#[cfg(feature = "polkadot-ahm")]
const WHALES: &[(AccountId32, u128)] = &[
	(
		AccountId32::new(hex!("f5d5714c084c112843aca74f8c498da06cc5a2d63153b825189baa51043b1f0b")),
		100_000_000,
	),
	(
		AccountId32::new(hex!("70f59acb102933da7bb3014e9417745a1f5b1a8ef6dfb141c493597a7b723f26")),
		40_000_000,
	),
	(
		AccountId32::new(hex!("5003aa0a3e9eaf4a3727bccb8dd0ffc7e3b8c936bba435328652a78545b54d25")),
		10_000_000,
	),
	(
		AccountId32::new(hex!("0bd1177d190c955fac5de6a176769fb1b3237c47c3a22a8bff2451a39979634d")),
		10_000_000,
	),
];

#[cfg(feature = "kusama-ahm")]
const WHALES: &[(AccountId32, u128)] = &[];

pub struct BalanceWhaleWatching;

impl RcMigrationCheck for BalanceWhaleWatching {
	type RcPrePayload = ();

	fn pre_check() -> Self::RcPrePayload {
		for (whale, min_amount) in WHALES {
			let acc = frame_system::Account::<RelayRuntime>::get(whale);
			let total_amount = acc.data.free + acc.data.reserved;
			let min_amount = (*min_amount) * RC_DOLLARS;

			assert!(
				total_amount >= min_amount,
				"Whale is missing pre balance {whale:?}: {total_amount} < {min_amount}"
			);
		}
	}

	fn post_check(_: Self::RcPrePayload) {}
}

impl AhMigrationCheck for BalanceWhaleWatching {
	type RcPrePayload = ();
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {}

	fn post_check(_rc_pre_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
		for (whale, min_amount) in WHALES {
			let acc = frame_system::Account::<AssetHubRuntime>::get(whale);
			let total_amount = acc.data.free + acc.data.reserved;
			let min_amount = (*min_amount) * RC_DOLLARS;

			assert!(
				total_amount >= min_amount,
				"Whale is missing post balance {whale:?}: {total_amount} < {min_amount}"
			);
		}
	}
}
