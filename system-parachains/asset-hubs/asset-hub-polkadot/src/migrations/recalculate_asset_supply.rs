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

//! Single-block migration that fixes dotUSD total issuance.
//!
//! dotUSD's recorded supply (`AssetDetails::supply`) sits below the real sum of its account
//! balances: some token credits were dropped during cross-chain message handling, which lowered
//! `supply` without moving any balance. The tokens still exist and stay fully backed; only the
//! counter is wrong. A low counter caps how much each account can move, so it freezes part of the
//! balances until the counter is fixed.
//!
//! The fix sums every dotUSD account in one block and writes the total back as `supply`. It runs
//! only while the asset is small enough to sum in a single block: if the account count is above
//! `MaxAccounts`, the migration does nothing.

use core::marker::PhantomData;
use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
};
use pallet_assets::{Account, Asset, AssetStatus, BalanceOnHold, Config as AssetsConfig};
use sp_runtime::traits::{Saturating, Zero};

/// Logging target for this migration.
const LOG_TARGET: &str = "runtime::asset-hub-polkadot::recalculate-asset-supply";

/// Proof size charged per account the migration reads: its `Account` entry plus its hold entry,
/// about 3 KiB each (value plus trie path), matching the generated `pallet-assets` weights.
const PROOF_SIZE_PER_ACCOUNT: u64 = 2 * 3 * 1024;

/// Single-block migration that recomputes the recorded total issuance of one asset.
///
/// `AssetId` is the asset to fix (dotUSD). `MaxAccounts` is the hard ceiling: if the asset has more
/// accounts than this, the sum would not fit in a single block, so the migration does nothing.
///
/// See the [module documentation](self) for the rationale.
pub struct RecalculateSupply<T, I, AssetId, MaxAccounts>(PhantomData<(T, I, AssetId, MaxAccounts)>);

impl<T, I, AssetId, MaxAccounts> OnRuntimeUpgrade for RecalculateSupply<T, I, AssetId, MaxAccounts>
where
	T: AssetsConfig<I>,
	I: 'static,
	AssetId: Get<T::AssetId>,
	MaxAccounts: Get<u32>,
{
	fn on_runtime_upgrade() -> Weight {
		let asset_id = AssetId::get();
		let db = T::DbWeight::get();

		// Skip if the asset is gone or being destroyed (its supply is drained on purpose).
		let details = match Asset::<T, I>::get(&asset_id) {
			Some(details) if details.status != AssetStatus::Destroying => details,
			_ => {
				log::info!(target: LOG_TARGET, "asset {asset_id:?} missing or destroying; skipping");
				return db.reads(1);
			},
		};

		// Too many accounts to sum safely in one block: do nothing.
		let max = MaxAccounts::get();
		if details.accounts > max {
			log::warn!(
				target: LOG_TARGET,
				"asset {asset_id:?} has {} accounts, above the ceiling {max}; skipping",
				details.accounts,
			);
			return db.reads(1);
		}

		// Sum `balance + on_hold` across every account.
		let mut computed = T::Balance::zero();
		let mut count = 0u64;
		for (who, account) in Account::<T, I>::iter_prefix(&asset_id) {
			let held = <T as AssetsConfig<I>>::Holder::balance_on_hold(asset_id.clone(), &who)
				.unwrap_or_else(Zero::zero);
			computed = computed.saturating_add(account.balance).saturating_add(held);
			count = count.saturating_add(1);
		}

		if details.supply != computed {
			log::info!(
				target: LOG_TARGET,
				"asset {asset_id:?}: correcting total issuance {:?} -> {computed:?}",
				details.supply,
			);
			Asset::<T, I>::mutate(&asset_id, |maybe_details| {
				if let Some(details) = maybe_details {
					details.supply = computed;
				}
			});
		}

		// One read for the details, two reads per account (`Account` + hold), one write.
		db.reads_writes(1u64.saturating_add(count.saturating_mul(2)), 1)
			.saturating_add(Weight::from_parts(0, count.saturating_mul(PROOF_SIZE_PER_ACCOUNT)))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
		// Nothing to carry over: `post_upgrade` recomputes the invariant from scratch.
		Ok(alloc::vec::Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		let asset_id = AssetId::get();
		let Some(details) = Asset::<T, I>::get(&asset_id) else { return Ok(()) };
		// Skipped on purpose: asset being destroyed, or too many accounts to fix in one block.
		if details.status == AssetStatus::Destroying || details.accounts > MaxAccounts::get() {
			return Ok(());
		}

		let mut computed = T::Balance::zero();
		for (who, account) in Account::<T, I>::iter_prefix(&asset_id) {
			let held = <T as AssetsConfig<I>>::Holder::balance_on_hold(asset_id.clone(), &who)
				.unwrap_or_else(Zero::zero);
			computed = computed.saturating_add(account.balance).saturating_add(held);
		}

		ensure!(
			details.supply == computed,
			"recalculate_asset_supply: supply still differs from the sum of balances"
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Runtime, RuntimeGenesisConfig, RuntimeOrigin, TrustBackedAssetsInstance};
	use alloc::{vec, vec::Vec};
	use frame_support::{parameter_types, traits::OnRuntimeUpgrade};
	use pallet_assets::BalanceOnHold;
	use sp_runtime::{BuildStorage, MultiAddress};

	type AssetsPallet = pallet_assets::Pallet<Runtime, TrustBackedAssetsInstance>;
	type AssetStore = pallet_assets::Asset<Runtime, TrustBackedAssetsInstance>;
	type AccountStore = pallet_assets::Account<Runtime, TrustBackedAssetsInstance>;
	type AccountId = <Runtime as frame_system::Config>::AccountId;

	/// dotUSD on Asset Hub Polkadot.
	const DOT_USD: u32 = 7873;

	parameter_types! {
		pub const TestAssetId: u32 = DOT_USD;
		// High enough that the normal tests always run.
		pub const MaxBig: u32 = 1_000;
		// Low enough to trip the ceiling in the skip test.
		pub const MaxTiny: u32 = 2;
	}

	type Migration = RecalculateSupply<Runtime, TrustBackedAssetsInstance, TestAssetId, MaxBig>;
	type TinyMigration =
		RecalculateSupply<Runtime, TrustBackedAssetsInstance, TestAssetId, MaxTiny>;

	fn new_test_ext() -> sp_io::TestExternalities {
		sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
		)
	}

	fn acc(n: u8) -> AccountId {
		[n; 32].into()
	}

	/// `force_create` the asset (Root path, charges no deposit) owned by `owner`, then mint the
	/// given balances. The owner is the issuer, so no DOT funding is needed.
	fn setup_asset(asset: u32, owner: &AccountId, balances: &[(AccountId, u128)]) {
		AssetsPallet::force_create(
			RuntimeOrigin::root(),
			asset.into(),
			MultiAddress::Id(owner.clone()),
			true, // is_sufficient, so beneficiary accounts need no ED
			1,    // min_balance
		)
		.expect("force_create works");
		for (who, amount) in balances {
			AssetsPallet::mint(
				RuntimeOrigin::signed(owner.clone()),
				asset.into(),
				MultiAddress::Id(who.clone()),
				*amount,
			)
			.expect("mint works");
		}
	}

	/// Overwrite the recorded supply of `asset`, simulating the on-chain mismatch.
	fn corrupt_supply(asset: u32, wrong_supply: u128) {
		AssetStore::mutate(asset, |maybe| {
			maybe.as_mut().expect("asset exists").supply = wrong_supply;
		});
	}

	/// The real sum of `balance + on_hold` across the asset's accounts.
	fn real_issuance(asset: u32) -> u128 {
		let mut total = 0u128;
		for (who, account) in AccountStore::iter_prefix(asset) {
			let held = <Runtime as pallet_assets::Config<TrustBackedAssetsInstance>>::Holder::balance_on_hold(asset, &who).unwrap_or(0);
			total += account.balance + held;
		}
		total
	}

	#[test]
	fn corrects_undercounted_supply() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			let balances = [(acc(10), 600_000u128), (acc(11), 400_000u128), (acc(12), 1u128)];
			setup_asset(DOT_USD, &owner, &balances);

			let real = real_issuance(DOT_USD);
			assert_eq!(real, 1_000_001);

			// Simulate the mismatch: supply burned far below the real balances.
			corrupt_supply(DOT_USD, 1);

			Migration::on_runtime_upgrade();
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);
		});
	}

	#[test]
	fn corrects_overcounted_supply() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			let real = real_issuance(DOT_USD);

			corrupt_supply(DOT_USD, real + 123);

			Migration::on_runtime_upgrade();
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);
		});
	}

	#[test]
	fn is_idempotent() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 777u128), (acc(11), 223u128)]);
			let real = real_issuance(DOT_USD);
			corrupt_supply(DOT_USD, 2);

			Migration::on_runtime_upgrade();
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);

			// Running again changes nothing.
			Migration::on_runtime_upgrade();
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);
		});
	}

	#[test]
	fn missing_asset_is_skipped() {
		new_test_ext().execute_with(|| {
			// dotUSD does not exist in this ext; the migration must finish without error.
			assert!(AssetStore::get(DOT_USD).is_none());
			Migration::on_runtime_upgrade();
			assert!(AssetStore::get(DOT_USD).is_none());
		});
	}

	#[test]
	fn skips_when_over_ceiling() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			// Three accounts, above the ceiling of two.
			let balances: Vec<(AccountId, u128)> =
				vec![(acc(10), 100), (acc(11), 200), (acc(12), 300)];
			setup_asset(DOT_USD, &owner, &balances);
			assert!(AssetStore::get(DOT_USD).unwrap().accounts > MaxTiny::get());

			// Wrong supply stays wrong: the migration must not touch it.
			corrupt_supply(DOT_USD, 1);
			TinyMigration::on_runtime_upgrade();
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, 1);
		});
	}

	#[test]
	fn skips_destroying_asset() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			corrupt_supply(DOT_USD, 1);

			// Mark the asset as being destroyed; its supply must be left alone.
			AssetStore::mutate(DOT_USD, |maybe| {
				maybe.as_mut().unwrap().status = AssetStatus::Destroying;
			});

			Migration::on_runtime_upgrade();
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, 1);
		});
	}
}
