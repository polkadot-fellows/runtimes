// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Runs the dotUSD issuance-correction migration against real Asset Hub Polkadot state.
//!
//! Only dotUSD's own storage is fetched (its `Assets::Asset`, `Assets::Account` and
//! `AssetsHolder::BalancesOnHold` entries), so the pulled state stays tiny. The test prints the
//! supply before, the recomputed real issuance and the supply after.
//!
//! Run against a node (no snapshot file needed):
//!
//! ```sh
//! WS=wss://polkadot-asset-hub-rpc.polkadot.io \
//!   cargo test -p asset-hub-polkadot-runtime --features try-runtime \
//!   dotusd_total_issuance_is_corrected -- --nocapture
//! ```

use crate::{
	migrations::{recalculate_asset_supply::RecalculateSupply, DotUsdAssetId, DotUsdMaxAccounts},
	Block, Runtime, TrustBackedAssetsInstance,
};
use codec::Encode;
use frame_support::{
	storage::storage_prefix, traits::OnRuntimeUpgrade, Blake2_128Concat, StorageHasher,
};
use pallet_assets::BalanceOnHold;
use remote_externalities::{Builder, Mode, OnlineConfig};
use std::env::var;

/// dotUSD on Asset Hub Polkadot.
const DOT_USD: u32 = 7873;

/// The migration under test, wired exactly as it ships in the runtime.
type Migration =
	RecalculateSupply<Runtime, TrustBackedAssetsInstance, DotUsdAssetId, DotUsdMaxAccounts>;
type Asset = pallet_assets::Asset<Runtime, TrustBackedAssetsInstance>;
type Account = pallet_assets::Account<Runtime, TrustBackedAssetsInstance>;

/// The hashed storage prefix that scopes `pallet::item` to a single asset id.
fn asset_prefix(pallet: &[u8], item: &[u8], asset: u32) -> Vec<u8> {
	let mut key = storage_prefix(pallet, item).to_vec();
	key.extend(Blake2_128Concat::hash(&asset.encode()));
	key
}

/// `Σ (balance + on_hold)` over dotUSD's accounts, and the account count.
fn real_issuance() -> (u128, u32) {
	let mut total = 0u128;
	let mut count = 0u32;
	for (who, account) in Account::iter_prefix(DOT_USD) {
		let held =
			<Runtime as pallet_assets::Config<TrustBackedAssetsInstance>>::Holder::balance_on_hold(
				DOT_USD, &who,
			)
			.unwrap_or(0);
		total += account.balance + held;
		count += 1;
	}
	(total, count)
}

#[tokio::test]
async fn dotusd_total_issuance_is_corrected() {
	sp_tracing::try_init_simple();

	// Only runs when WS points at an Asset Hub Polkadot RPC endpoint.
	let Some(uri) = var("WS").ok() else {
		println!("skipping: set WS=wss://... to run this test");
		return;
	};

	// Optionally pin the block (`AT=0x..`), so an external check can read the same state.
	let at = var("AT").ok().map(|h| {
		use core::str::FromStr;
		sp_core::H256::from_str(h.trim()).expect("AT must be a 0x-prefixed block hash")
	});

	// Fetch dotUSD's storage only: its asset details, its accounts, and its holds.
	let mut ext = Builder::<Block>::default()
		.mode(Mode::Online(OnlineConfig {
			at,
			transport_uris: vec![uri],
			child_trie: false,
			// The single asset detail is fetched by its exact key: as a prefix it would be the
			// whole key, which the paged key scan skips.
			hashed_keys: vec![Asset::hashed_key_for(DOT_USD)],
			hashed_prefixes: vec![
				asset_prefix(b"Assets", b"Account", DOT_USD),
				asset_prefix(b"AssetsHolder", b"BalancesOnHold", DOT_USD),
			],
			..Default::default()
		}))
		.build()
		.await
		.unwrap();

	ext.execute_with(|| {
		let supply_before = Asset::get(DOT_USD).expect("dotUSD asset present in state").supply;
		let (computed, accounts) = real_issuance();

		println!("dotUSD accounts fetched:   {accounts}");
		println!("supply BEFORE migration:   {supply_before}");
		println!("recomputed real issuance:  {computed}");
		println!("gap (computed - supply):   {}", computed as i128 - supply_before as i128);

		// Run the single-block migration.
		Migration::on_runtime_upgrade();

		let supply_after = Asset::get(DOT_USD).expect("dotUSD asset still present").supply;
		println!("supply AFTER migration:    {supply_after}");

		// The whole point: recorded issuance now equals the real sum of balances.
		assert_eq!(supply_after, computed, "supply must equal the sum of balances + holds");

		// Balances themselves are untouched.
		let (computed_again, accounts_again) = real_issuance();
		assert_eq!(computed_again, computed, "balances must be unchanged");
		assert_eq!(accounts_again, accounts, "account set must be unchanged");
	});
}
