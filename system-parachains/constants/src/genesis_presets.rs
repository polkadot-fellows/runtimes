// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
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

#[cfg(not(feature = "std"))]
use alloc::format;
use alloc::vec::Vec;
use parachains_common::AuraId;
use polkadot_primitives::{AccountId, AccountPublic};
use sp_core::{sr25519, Pair, Public};
use sp_runtime::traits::IdentifyAccount;

/// Invulnerable Collators
pub fn invulnerables() -> Vec<(parachains_common::AccountId, AuraId)> {
	Vec::from([
		(get_account_id_from_seed::<sr25519::Public>("Alice"), get_from_seed::<AuraId>("Alice")),
		(get_account_id_from_seed::<sr25519::Public>("Bob"), get_from_seed::<AuraId>("Bob")),
	])
}

/// Test accounts
pub fn testnet_accounts() -> Vec<AccountId> {
	Vec::from([
		get_account_id_from_seed::<sr25519::Public>("Alice"),
		get_account_id_from_seed::<sr25519::Public>("Bob"),
		get_account_id_from_seed::<sr25519::Public>("Charlie"),
		get_account_id_from_seed::<sr25519::Public>("Dave"),
		get_account_id_from_seed::<sr25519::Public>("Eve"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie"),
		get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
		get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
		get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
		get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
		get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
		get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
		// The following are ethereum derived addresses.
		// Import into your wallet by deriving from this seed:
		// "bottom drive obey lake curtain smoke basket hold race lonely fit walk"
		// BIP44 Serial [1, 5]
		// subxt_signer::eth::dev::alith()
		array_bytes::dehexify_array_then_into(
			"f24ff3a9cf04c71dbc94d0b566f7a27b94566caceeeeeeeeeeeeeeeeeeeeeeee",
		)
		.unwrap(),
		// subxt_signer::eth::dev::baltathar()
		array_bytes::dehexify_array_then_into(
			"3cd0a705a2dc65e5b1e1205896baa2be8a07c6e0eeeeeeeeeeeeeeeeeeeeeeee",
		)
		.unwrap(),
		// subxt_signer::eth::dev::charleth()
		array_bytes::dehexify_array_then_into(
			"798d4ba9baf0064ec19eb4f0a1a45785ae9d6dfceeeeeeeeeeeeeeeeeeeeeeee",
		)
		.unwrap(),
		// subxt_signer::eth::dev::dorothy()
		array_bytes::dehexify_array_then_into(
			"773539d4ac0e786233d90a233654ccee26a613d9eeeeeeeeeeeeeeeeeeeeeeee",
		)
		.unwrap(),
		// subxt_signer::eth::dev::ethan()
		array_bytes::dehexify_array_then_into(
			"ff64d3f6efe2317ee2807d223a0bdc4c0c49dfdbeeeeeeeeeeeeeeeeeeeeeeee",
		)
		.unwrap(),
	])
}

/// Test accounts extended `with`.
pub fn testnet_accounts_with(extra: impl IntoIterator<Item = AccountId>) -> Vec<AccountId> {
	let mut accounts = testnet_accounts();
	accounts.extend(extra);
	accounts
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(&format!("//{seed}"), None)
		.expect("static values are valid; qed")
		.public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
	AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
	AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// The default XCM version to set in genesis config.
pub const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
