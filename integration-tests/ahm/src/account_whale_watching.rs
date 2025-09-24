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

use hex_literal::hex;
use pallet_ah_migrator::types::AhMigrationCheck;
use pallet_rc_migrator::types::RcMigrationCheck;
use sp_runtime::AccountId32;

type RelayRuntime = polkadot_runtime::Runtime;
type AssetHubRuntime = asset_hub_polkadot_runtime::Runtime;

/// Whale accounts that we care about and minimal total resulting balance.
#[cfg(feature = "polkadot-ahm")]
const WHALES: &[(AccountId32, usize)] = &[
	// TODO
];

/// Whale accounts that we care about and minimal total resulting balance.
#[cfg(feature = "kusama-ahm")]
const WHALES: &[(AccountId32, u128)] = &[
	(
		AccountId32::new(hex!("f1c5ca0368e7a567945a59aaea92b9be1e0794fe5e077d017462b7ce8fc1ed7c")),
		38000,
	), // Bifrost staking proxy
];

pub struct BalanceWhaleWatching;

impl RcMigrationCheck for BalanceWhaleWatching {
	type RcPrePayload = ();

	fn pre_check() -> Self::RcPrePayload {
		for (whale, min_amount) in WHALES {
			let acc = frame_system::Account::<RelayRuntime>::get(whale);
			let total_amount = acc.data.free + acc.data.reserved;

			assert!(
				total_amount >= (*min_amount) * RC_DOLLARS,
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

			assert!(
				total_amount >= (*min_amount) * RC_DOLLARS,
				"Whale is missing post balance {whale:?}: {total_amount} < {min_amount}"
			);
		}
	}
}
