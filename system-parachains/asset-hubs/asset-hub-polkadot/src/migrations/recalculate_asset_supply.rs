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

//! Multi-block migration that fixes `pallet-assets` total issuance.
//!
//! An asset's recorded total issuance (`AssetDetails::supply`) should equal the sum of every
//! account's balance plus what it holds.
//!
//! On Asset Hub Polkadot `supply` is too low for **dotUSD** (`7873`): some token credits were
//! dropped during cross-chain message handling, which lowered `supply` without changing any
//! balance. The tokens still exist and stay fully backed; only the counter is off. Because `supply`
//! caps how much each account can move, the low counter freezes part of the balances until it is
//! corrected.

use alloc::vec::Vec;
use codec::{Decode, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_support::{
	migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
	traits::Get,
	weights::WeightMeter,
};
use pallet_assets::{Account, Asset, AssetStatus, BalanceOnHold, Config as AssetsConfig};
use scale_info::TypeInfo;
use sp_runtime::traits::{Saturating, Zero};

/// Logging target for this migration.
const LOG_TARGET: &str = "runtime::asset-hub-polkadot::recalculate-asset-supply";

/// Globally unique identifier bytes for this migration (exactly 26 bytes).
const MIGRATION_ID: &[u8; 26] = b"ahp-assets-supply-recalc-1";

/// Progress cursor for [`RecalculateSupply`].
///
/// The migration walks the `TargetAssets` list one asset at a time; within an asset it walks that
/// asset's accounts, accumulating `balance + on_hold` into `partial_issuance` and remembering the
/// `last_account` so the next step resumes exactly where this one stopped.
#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, PartialEq, Eq)]
pub struct Cursor<AccountId, Balance> {
	/// Index into the `TargetAssets` list of the asset currently being summed.
	asset_index: u32,
	/// Running total of `balance + on_hold` over the accounts summed so far for this asset.
	partial_issuance: Balance,
	/// The last account summed for this asset; `None` means summing has not started yet.
	last_account: Option<AccountId>,
}

/// Multi-block migration that corrects the recorded total issuance of the assets listed by
/// `TargetAssets` in the `pallet-assets` instance `I`.
///
/// See the [module documentation](self) for the rationale and the correctness argument.
pub struct RecalculateSupply<T, I, TargetAssets>(PhantomData<(T, I, TargetAssets)>);

impl<T, I, TargetAssets> SteppedMigration for RecalculateSupply<T, I, TargetAssets>
where
	T: AssetsConfig<I>,
	I: 'static,
	TargetAssets: Get<Vec<T::AssetId>>,
{
	type Cursor = Cursor<T::AccountId, T::Balance>;
	// A fixed-length pallet id keeps the identifier construction infallible.
	type Identifier = MigrationId<26>;

	fn id() -> Self::Identifier {
		MigrationId { pallet_id: *MIGRATION_ID, version_from: 0, version_to: 1 }
	}

	fn step(
		cursor: Option<Self::Cursor>,
		meter: &mut WeightMeter,
	) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
		let targets = TargetAssets::get();

		// Each loop turn does at most: read one account (via the iterator) and its held balance,
		// then possibly read and rewrite one `AssetDetails`. Charging `reads_writes(2, 1)` every
		// turn covers both the summing turn (2 reads) and the finalizing turn (1 read + 1 write)
		// without ever under-charging, and keeps metering trivial to reason about.
		let required = T::DbWeight::get().reads_writes(2, 1);

		// Not even one turn fits in the remaining budget: ask the scheduler for a fresh block.
		if meter.remaining().any_lt(required) {
			return Err(SteppedMigrationError::InsufficientWeight { required });
		}

		let mut state = cursor.unwrap_or(Cursor {
			asset_index: 0,
			partial_issuance: Zero::zero(),
			last_account: None,
		});

		loop {
			// Reserve this turn's weight up front; stop cleanly once the block is full.
			if meter.try_consume(required).is_err() {
				break;
			}

			// Past the end of the list: the migration is complete.
			let Some(asset_id) = targets.get(state.asset_index as usize).cloned() else {
				return Ok(None);
			};

			// Only live assets are repaired. A missing asset, or one being destroyed (its supply is
			// being drained on purpose), must not have its `supply` rewritten: skip to the next.
			let should_skip = match Asset::<T, I>::get(&asset_id) {
				None => true,
				Some(details) => details.status == AssetStatus::Destroying,
			};
			if should_skip {
				state = Cursor {
					asset_index: state.asset_index.saturating_add(1),
					partial_issuance: Zero::zero(),
					last_account: None,
				};
				continue;
			}

			// Resume this asset's account iteration after the last account we summed, or start from
			// the first account if we have not begun this asset yet.
			let mut iter = match &state.last_account {
				Some(account) => Account::<T, I>::iter_prefix_from(
					asset_id.clone(),
					Account::<T, I>::hashed_key_for(&asset_id, account),
				),
				None => Account::<T, I>::iter_prefix(asset_id.clone()),
			};

			match iter.next() {
				// Another account to add into the running total.
				Some((who, account)) => {
					let held =
						<T as AssetsConfig<I>>::Holder::balance_on_hold(asset_id.clone(), &who)
							.unwrap_or_else(Zero::zero);
					state.partial_issuance =
						state.partial_issuance.saturating_add(account.balance).saturating_add(held);
					state.last_account = Some(who);
				},
				// No more accounts: commit the recomputed supply for this asset and advance.
				None => {
					let computed = state.partial_issuance;
					Asset::<T, I>::mutate(&asset_id, |maybe_details| {
						if let Some(details) = maybe_details {
							if details.supply != computed {
								log::info!(
									target: LOG_TARGET,
									"asset {:?}: correcting total issuance {:?} -> {:?}",
									asset_id,
									details.supply,
									computed,
								);
								details.supply = computed;
							}
						}
					});
					state = Cursor {
						asset_index: state.asset_index.saturating_add(1),
						partial_issuance: Zero::zero(),
						last_account: None,
					};
				},
			}
		}

		Ok(Some(state))
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		// Nothing to carry over: `post_upgrade` recomputes the invariant from scratch.
		Ok(Vec::new())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		for asset_id in TargetAssets::get() {
			let Some(details) = Asset::<T, I>::get(&asset_id) else { continue };
			if details.status == AssetStatus::Destroying {
				continue;
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
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Runtime, RuntimeGenesisConfig, RuntimeOrigin, TrustBackedAssetsInstance};
	use alloc::{vec, vec::Vec};
	use frame_support::{
		traits::Get,
		weights::{Weight, WeightMeter},
	};
	use pallet_assets::BalanceOnHold;
	use sp_runtime::{BuildStorage, MultiAddress};

	type AssetsPallet = pallet_assets::Pallet<Runtime, TrustBackedAssetsInstance>;
	type AssetStore = pallet_assets::Asset<Runtime, TrustBackedAssetsInstance>;
	type AccountStore = pallet_assets::Account<Runtime, TrustBackedAssetsInstance>;
	type AccountId = <Runtime as frame_system::Config>::AccountId;

	/// dotUSD on Asset Hub Polkadot.
	const DOT_USD: u32 = 7873;

	/// Target list used by the tests: dotUSD only, mirroring the runtime wiring.
	pub struct TestTargets;
	impl Get<Vec<u32>> for TestTargets {
		fn get() -> Vec<u32> {
			vec![DOT_USD]
		}
	}

	type Migration = RecalculateSupply<Runtime, TrustBackedAssetsInstance, TestTargets>;

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

	/// Overwrite the recorded supply of `asset`, simulating the on-chain corruption.
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

	/// Drive the migration to completion, one bounded step per call; returns the step count.
	/// `per_step` bounds the weight each step receives, forcing multi-step execution when small.
	fn run_to_completion(per_step: Weight) -> u32 {
		let mut cursor = None;
		let mut steps = 0;
		loop {
			let mut meter = WeightMeter::with_limit(per_step);
			cursor = Migration::step(cursor, &mut meter).expect("step never fails");
			steps += 1;
			if cursor.is_none() {
				break;
			}
			assert!(steps < 10_000, "migration should terminate");
		}
		steps
	}

	#[test]
	fn corrects_undercounted_supply() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			let balances = [(acc(10), 600_000u128), (acc(11), 400_000u128), (acc(12), 1u128)];
			setup_asset(DOT_USD, &owner, &balances);

			let real = real_issuance(DOT_USD);
			assert_eq!(real, 1_000_001);
			// Supply correctly equals the sum right after minting.
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);

			// Simulate the executor bug: supply burned far below the real balances.
			corrupt_supply(DOT_USD, 1);
			assert_ne!(AssetStore::get(DOT_USD).unwrap().supply, real);

			// One generous step is enough for this small asset.
			let steps = run_to_completion(Weight::MAX);
			assert_eq!(steps, 1);
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);
		});
	}

	#[test]
	fn corrects_overcounted_supply() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			let real = real_issuance(DOT_USD);

			// Legacy-style overcount: supply above the real balances.
			corrupt_supply(DOT_USD, real + 123);

			run_to_completion(Weight::MAX);
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);
		});
	}

	#[test]
	fn spans_multiple_blocks_and_resumes() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			let balances: Vec<(AccountId, u128)> =
				(20u8..60u8).map(|n| (acc(n), n as u128 * 1_000)).collect();
			setup_asset(DOT_USD, &owner, &balances);
			let real = real_issuance(DOT_USD);

			corrupt_supply(DOT_USD, 1);

			// Starve each step so only a few accounts are processed per block, forcing the cursor
			// to carry a partial sum across many steps.
			let per_step = <Runtime as frame_system::Config>::DbWeight::get()
				.reads_writes(2, 1)
				.saturating_mul(3);
			let steps = run_to_completion(per_step);
			assert!(steps > 1, "expected the migration to span multiple steps, took {steps}");
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

			run_to_completion(Weight::MAX);
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);

			// Running again changes nothing.
			run_to_completion(Weight::MAX);
			assert_eq!(AssetStore::get(DOT_USD).unwrap().supply, real);
		});
	}

	#[test]
	fn missing_asset_is_skipped() {
		new_test_ext().execute_with(|| {
			// dotUSD does not exist in this ext; the migration must finish without error.
			assert!(AssetStore::get(DOT_USD).is_none());
			let steps = run_to_completion(Weight::MAX);
			assert_eq!(steps, 1);
			assert!(AssetStore::get(DOT_USD).is_none());
		});
	}
}
