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

//! Multi-block migration that fixes dotUSD total issuance.
//!
//! dotUSD's recorded supply (`AssetDetails::supply`) sits below the real sum of its account
//! balances: some token credits were dropped during cross-chain message handling, which lowered
//! `supply` without moving any balance. The tokens still exist and stay fully backed; only the
//! counter is wrong. A low counter caps how much each account can move, so it freezes part of the
//! balances until the counter is fixed.
//!
//! How it runs:
//!
//! 1. The asset is set to `Frozen`. From then on no balance can change. Transfers, mints, burns,
//!    PSM mint and redeem, pool swaps and XCM transfers of the asset all fail until the migration
//!    is done.
//! 2. `balance + on_hold` is added up over all accounts, as many per block as the weight allows.
//!    This starts in the same step as the freeze.
//! 3. The sum is written as the new supply and the status is put back to what it was.
//!
//! The freeze is what makes the sum exact. XCM messages are still processed while a multi-block
//! migration runs, so without it balances could move between steps.
//!
//! If the migration stops half way, the asset stays frozen. The asset admin can then call
//! `Assets::thaw_asset`, or governance can call `Assets::force_asset_status`.

use codec::{Decode, Encode, MaxEncodedLen};
use core::marker::PhantomData;
use frame_support::{
	migrations::{MigrationId, SteppedMigration, SteppedMigrationError},
	traits::Get,
	weights::{Weight, WeightMeter},
};
use pallet_assets::{
	Account, Asset, AssetDetails, AssetStatus, BalanceOnHold, Config as AssetsConfig,
};
use sp_runtime::traits::{Saturating, Zero};

const LOG_TARGET: &str = "runtime::asset-hub-polkadot::recalculate-asset-supply";

/// Proof size budget per account: its `Account` entry and its hold entry, with trie overhead.
const PROOF_SIZE_PER_ACCOUNT: u64 = 2 * 3 * 1024;

/// Progress of the migration, kept between steps.
#[derive(Encode, Decode, MaxEncodedLen, Clone, PartialEq, Eq, Debug)]
pub struct Cursor<AccountId, Balance> {
	/// Status of the asset before the freeze. Put back at the end.
	prior_status: AssetStatus,
	/// Account count at the start. Cannot change while the asset is frozen.
	accounts: u32,
	/// Running sum of `balance + on_hold`.
	sum: Balance,
	/// Last account added to the sum. `None` before the first one.
	last: Option<AccountId>,
}

pub struct RecalculateSupply<T, I, AssetId>(PhantomData<(T, I, AssetId)>);

impl<T, I, AssetId> RecalculateSupply<T, I, AssetId>
where
	T: AssetsConfig<I>,
	I: 'static,
{
	/// Weight to read one account and its hold.
	pub(crate) fn per_account_weight() -> Weight {
		T::DbWeight::get()
			.reads(2)
			.saturating_add(Weight::from_parts(0, PROOF_SIZE_PER_ACCOUNT))
	}

	/// Deposit a pallet-assets event, the same one its freeze and thaw calls emit.
	fn deposit_event(event: pallet_assets::Event<T, I>) {
		frame_system::Pallet::<T>::deposit_event(
			<T as AssetsConfig<I>>::RuntimeEvent::from(event).into(),
		);
	}

	/// Put back the status from before the freeze. Emits `AssetThawed` if that unfreezes it.
	fn restore_status(asset_id: T::AssetId, prior_status: &AssetStatus) {
		if *prior_status == AssetStatus::Live {
			Self::deposit_event(pallet_assets::Event::AssetThawed { asset_id });
		}
	}

	/// Weight to read and write the asset details once.
	pub(crate) fn details_weight() -> Weight {
		T::DbWeight::get().reads_writes(1, 1)
	}
}

impl<T, I, AssetId> SteppedMigration for RecalculateSupply<T, I, AssetId>
where
	T: AssetsConfig<I>,
	I: 'static,
	AssetId: Get<T::AssetId>,
{
	type Cursor = Cursor<T::AccountId, T::Balance>;
	type Identifier = MigrationId<16>;

	fn id() -> Self::Identifier {
		MigrationId { pallet_id: *b"ah-dotusd-supply", version_from: 0, version_to: 1 }
	}

	fn max_steps() -> Option<u32> {
		Some(500)
	}

	fn step(
		cursor: Option<Self::Cursor>,
		meter: &mut WeightMeter,
	) -> Result<Option<Self::Cursor>, SteppedMigrationError> {
		let asset_id = AssetId::get();

		// First call: freeze the asset so the sum cannot drift, then start counting right away.
		let mut cursor = match cursor {
			Some(cursor) => cursor,
			None => {
				let required = Self::details_weight();
				meter
					.try_consume(required)
					.map_err(|_| SteppedMigrationError::InsufficientWeight { required })?;

				let Some(details) = Asset::<T, I>::get(&asset_id) else {
					log::info!(target: LOG_TARGET, "asset {asset_id:?} missing; nothing to do");
					return Ok(None);
				};
				if details.status == AssetStatus::Destroying {
					log::info!(target: LOG_TARGET, "asset {asset_id:?} is being destroyed; nothing to do");
					return Ok(None);
				}
				let prior_status = details.status.clone();
				let accounts = details.accounts;
				Asset::<T, I>::insert(
					&asset_id,
					AssetDetails { status: AssetStatus::Frozen, ..details },
				);
				if prior_status != AssetStatus::Frozen {
					Self::deposit_event(pallet_assets::Event::AssetFrozen {
						asset_id: asset_id.clone(),
					});
				}
				log::info!(target: LOG_TARGET, "asset {asset_id:?} frozen; counting {accounts} accounts");
				Cursor { prior_status, accounts, sum: Zero::zero(), last: None }
			},
		};

		// Reserve the weight for the last step, so finishing never fails on weight.
		let per_account = Self::per_account_weight();
		let required = Self::details_weight().saturating_add(per_account);
		if meter.remaining().any_lt(required) {
			return Err(SteppedMigrationError::InsufficientWeight { required });
		}
		meter.consume(Self::details_weight());

		// Add up accounts while the weight lasts.
		loop {
			if meter.try_consume(per_account).is_err() {
				return Ok(Some(cursor));
			}
			let mut iter = match &cursor.last {
				Some(last) => Account::<T, I>::iter_prefix_from(
					&asset_id,
					Account::<T, I>::hashed_key_for(&asset_id, last),
				),
				None => Account::<T, I>::iter_prefix(&asset_id),
			};
			let Some((who, account)) = iter.next() else { break };
			let held = <T as AssetsConfig<I>>::Holder::balance_on_hold(asset_id.clone(), &who)
				.unwrap_or_else(Zero::zero);
			cursor.sum = cursor.sum.saturating_add(account.balance).saturating_add(held);
			cursor.last = Some(who);
		}

		// Last step: write the sum and put the status back.
		let Some(details) = Asset::<T, I>::get(&asset_id) else {
			log::warn!(target: LOG_TARGET, "asset {asset_id:?} vanished while counting");
			return Ok(None);
		};
		if details.status != AssetStatus::Frozen {
			log::warn!(
				target: LOG_TARGET,
				"asset {asset_id:?} status changed to {:?} while counting; supply left alone",
				details.status,
			);
			return Ok(None);
		}
		if details.accounts != cursor.accounts {
			log::warn!(
				target: LOG_TARGET,
				"asset {asset_id:?} account count changed while frozen; supply left alone",
			);
			Self::restore_status(asset_id.clone(), &cursor.prior_status);
			Asset::<T, I>::insert(
				&asset_id,
				AssetDetails { status: cursor.prior_status, ..details },
			);
			return Ok(None);
		}
		log::info!(
			target: LOG_TARGET,
			"asset {asset_id:?}: total issuance {:?} -> {:?}; status back to {:?}",
			details.supply,
			cursor.sum,
			cursor.prior_status,
		);
		Self::restore_status(asset_id.clone(), &cursor.prior_status);
		Asset::<T, I>::insert(
			&asset_id,
			AssetDetails { supply: cursor.sum, status: cursor.prior_status, ..details },
		);
		Ok(None)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
		// Remember the status, so `post_upgrade` can check it was put back.
		Ok(Asset::<T, I>::get(AssetId::get()).map(|d| d.status).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		let asset_id = AssetId::get();
		let prior = Option::<AssetStatus>::decode(&mut &state[..])
			.map_err(|_| "recalculate_asset_supply: cannot decode pre_upgrade state")?;
		let Some(details) = Asset::<T, I>::get(&asset_id) else { return Ok(()) };
		// Skipped on purpose.
		if prior == Some(AssetStatus::Destroying) {
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
		ensure!(
			Some(details.status) == prior,
			"recalculate_asset_supply: asset status was not put back"
		);
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Runtime, RuntimeGenesisConfig, RuntimeOrigin, TrustBackedAssetsInstance};
	use codec::Encode;
	use frame_support::{assert_noop, parameter_types, traits::Get};
	use sp_runtime::{BuildStorage, MultiAddress};

	type AssetsPallet = pallet_assets::Pallet<Runtime, TrustBackedAssetsInstance>;
	type AssetStore = pallet_assets::Asset<Runtime, TrustBackedAssetsInstance>;
	type AccountStore = pallet_assets::Account<Runtime, TrustBackedAssetsInstance>;
	type AssetsError = pallet_assets::Error<Runtime, TrustBackedAssetsInstance>;
	type AccountId = <Runtime as frame_system::Config>::AccountId;

	/// dotUSD on Asset Hub Polkadot.
	const DOT_USD: u32 = 7873;

	parameter_types! {
		pub const TestAssetId: u32 = DOT_USD;
	}

	type Migration = RecalculateSupply<Runtime, TrustBackedAssetsInstance, TestAssetId>;
	type TestCursor = <Migration as SteppedMigration>::Cursor;

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

	fn set_status(asset: u32, status: AssetStatus) {
		AssetStore::mutate(asset, |maybe| {
			maybe.as_mut().expect("asset exists").status = status;
		});
	}

	fn supply(asset: u32) -> u128 {
		AssetStore::get(asset).expect("asset exists").supply
	}

	fn status(asset: u32) -> AssetStatus {
		AssetStore::get(asset).expect("asset exists").status
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

	/// Run one step with the given weight limit.
	fn step(cursor: Option<TestCursor>, limit: Weight) -> Option<TestCursor> {
		let mut meter = WeightMeter::with_limit(limit);
		Migration::step(cursor, &mut meter).expect("step works")
	}

	/// Run the migration to the end with the given weight limit per step. Returns the step count.
	fn run_with_limit(limit: Weight) -> u32 {
		let mut cursor = None;
		let mut steps = 0;
		loop {
			steps += 1;
			cursor = step(cursor, limit);
			if cursor.is_none() {
				return steps;
			}
			assert!(steps < 1_000, "migration does not finish");
		}
	}

	fn run() -> u32 {
		run_with_limit(Weight::MAX)
	}

	/// Weight for the freeze, the reserve for the last step, and `n` accounts.
	fn limit_for(n: u64) -> Weight {
		Migration::details_weight()
			.saturating_mul(2)
			.saturating_add(Migration::per_account_weight().saturating_mul(n))
	}

	/// Freeze and count `n` accounts, then run out of weight. The asset needs at least `n`
	/// accounts; with exactly `n`, the step stops before it can see the end.
	fn partial_step(n: u64) -> Option<TestCursor> {
		let cursor = step(None, limit_for(n));
		assert!(cursor.is_some(), "step must stop before the end");
		cursor
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

			run();
			assert_eq!(supply(DOT_USD), real);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
		});
	}

	#[test]
	fn corrects_overcounted_supply() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			let real = real_issuance(DOT_USD);

			corrupt_supply(DOT_USD, real + 123);

			run();
			assert_eq!(supply(DOT_USD), real);
		});
	}

	#[test]
	fn is_idempotent() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 777u128), (acc(11), 223u128)]);
			let real = real_issuance(DOT_USD);
			corrupt_supply(DOT_USD, 2);

			run();
			assert_eq!(supply(DOT_USD), real);

			// Running again changes nothing.
			run();
			assert_eq!(supply(DOT_USD), real);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
		});
	}

	#[test]
	fn missing_asset_is_skipped() {
		new_test_ext().execute_with(|| {
			// dotUSD does not exist in this ext; the migration must finish in one step.
			assert!(AssetStore::get(DOT_USD).is_none());
			assert_eq!(run(), 1);
			assert!(AssetStore::get(DOT_USD).is_none());
		});
	}

	#[test]
	fn skips_destroying_asset() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			corrupt_supply(DOT_USD, 1);

			// Mark the asset as being destroyed; its supply must be left alone.
			set_status(DOT_USD, AssetStatus::Destroying);

			assert_eq!(run(), 1);
			assert_eq!(supply(DOT_USD), 1);
			assert_eq!(status(DOT_USD), AssetStatus::Destroying);
		});
	}

	#[test]
	fn freezes_while_counting() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128), (acc(11), 10u128)]);
			let real = real_issuance(DOT_USD);
			corrupt_supply(DOT_USD, 1);

			// The first step freezes and counts one account, then runs out of weight.
			let cursor = partial_step(1);
			assert_eq!(status(DOT_USD), AssetStatus::Frozen);
			assert_eq!(supply(DOT_USD), 1);

			// Nothing can move while frozen.
			assert_noop!(
				AssetsPallet::transfer(
					RuntimeOrigin::signed(acc(10)),
					DOT_USD.into(),
					MultiAddress::Id(acc(11)),
					1,
				),
				AssetsError::AssetNotLive
			);
			assert_noop!(
				AssetsPallet::mint(
					RuntimeOrigin::signed(owner.clone()),
					DOT_USD.into(),
					MultiAddress::Id(acc(11)),
					1,
				),
				AssetsError::AssetNotLive
			);
			assert_noop!(
				AssetsPallet::burn(
					RuntimeOrigin::signed(owner),
					DOT_USD.into(),
					MultiAddress::Id(acc(10)),
					1,
				),
				AssetsError::AssetNotLive
			);

			// The rest of the run fixes the supply and thaws the asset.
			assert!(step(cursor, Weight::MAX).is_none());
			assert_eq!(supply(DOT_USD), real);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
		});
	}

	#[test]
	fn keeps_prior_frozen_status() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			let real = real_issuance(DOT_USD);
			corrupt_supply(DOT_USD, 1);
			set_status(DOT_USD, AssetStatus::Frozen);

			run();
			assert_eq!(supply(DOT_USD), real);
			// It was frozen before, so it stays frozen.
			assert_eq!(status(DOT_USD), AssetStatus::Frozen);
		});
	}

	#[test]
	fn small_weight_takes_many_steps() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			let balances: Vec<(AccountId, u128)> =
				(10u8..20).map(|n| (acc(n), n as u128 * 1_000)).collect();
			setup_asset(DOT_USD, &owner, &balances);
			let real = real_issuance(DOT_USD);
			corrupt_supply(DOT_USD, 1);

			// Room for two accounts per step: 10 accounts take several steps.
			let steps = run_with_limit(limit_for(2));
			assert!(steps > 3, "took only {steps} steps");
			assert_eq!(supply(DOT_USD), real);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
		});
	}

	#[test]
	fn not_enough_weight_for_one_account_is_reported() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);

			// Weight for the freeze alone: nothing to count with.
			let mut meter = WeightMeter::with_limit(Migration::details_weight());
			assert!(matches!(
				Migration::step(None, &mut meter),
				Err(SteppedMigrationError::InsufficientWeight { .. })
			));

			// Same once counting has started.
			let cursor = partial_step(1);
			let mut meter = WeightMeter::with_limit(Migration::details_weight());
			assert!(matches!(
				Migration::step(cursor, &mut meter),
				Err(SteppedMigrationError::InsufficientWeight { .. })
			));
		});
	}

	#[test]
	fn leaves_supply_alone_if_status_changes_while_counting() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			corrupt_supply(DOT_USD, 1);

			let cursor = partial_step(1);
			// The owner starts destroying the asset in between.
			set_status(DOT_USD, AssetStatus::Destroying);

			assert!(step(cursor, Weight::MAX).is_none());
			assert_eq!(supply(DOT_USD), 1);
			assert_eq!(status(DOT_USD), AssetStatus::Destroying);
		});
	}

	#[test]
	fn thaws_but_leaves_supply_alone_if_account_count_changes() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			corrupt_supply(DOT_USD, 1);

			let cursor = partial_step(1);
			// Cannot happen while frozen; simulate a raw change of the account count.
			AssetStore::mutate(DOT_USD, |maybe| {
				maybe.as_mut().unwrap().accounts += 1;
			});

			assert!(step(cursor, Weight::MAX).is_none());
			assert_eq!(supply(DOT_USD), 1);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
		});
	}

	/// Small deterministic random generator, so failures are reproducible.
	struct Lcg(u64);
	impl Lcg {
		fn next(&mut self) -> u64 {
			self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
			self.0 >> 33
		}
	}

	#[test]
	fn any_step_size_gives_the_same_sum() {
		// Every account count against every step size, including steps that end exactly on
		// the last account.
		for accounts in 1u8..=9 {
			for per_step in 1u64..=10 {
				new_test_ext().execute_with(|| {
					let owner = acc(1);
					let balances: Vec<(AccountId, u128)> =
						(0..accounts).map(|n| (acc(100 + n), 1_000 + n as u128 * 7)).collect();
					setup_asset(DOT_USD, &owner, &balances);
					let real = real_issuance(DOT_USD);
					corrupt_supply(DOT_USD, 3);

					run_with_limit(limit_for(per_step));
					assert_eq!(supply(DOT_USD), real, "accounts {accounts}, per step {per_step}");
					assert_eq!(status(DOT_USD), AssetStatus::Live);
				});
			}
		}
	}

	#[test]
	fn random_balances_and_weights() {
		let mut rng = Lcg(0xD07_05D);
		for round in 0..40 {
			new_test_ext().execute_with(|| {
				let owner = acc(1);
				let n = 1 + (rng.next() % 60) as u8;
				let balances: Vec<(AccountId, u128)> = (0..n)
					.map(|i| (acc(20 + i), 1 + (rng.next() as u128 % 1_000_000_000_000)))
					.collect();
				setup_asset(DOT_USD, &owner, &balances);
				let real = real_issuance(DOT_USD);
				let wrong = rng.next() as u128;
				corrupt_supply(DOT_USD, wrong);
				let prior = if rng.next().is_multiple_of(4) {
					AssetStatus::Frozen
				} else {
					AssetStatus::Live
				};
				set_status(DOT_USD, prior.clone());

				let per_step = 1 + rng.next() % 15;
				run_with_limit(limit_for(per_step));
				assert_eq!(supply(DOT_USD), real, "round {round}");
				assert_eq!(status(DOT_USD), prior, "round {round}");
				// Balances are never touched.
				assert_eq!(real_issuance(DOT_USD), real, "round {round}");
			});
		}
	}

	#[test]
	fn other_assets_are_untouched() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 1_000u128)]);
			setup_asset(1984, &owner, &[(acc(10), 50u128), (acc(11), 70u128)]);
			corrupt_supply(DOT_USD, 1);
			corrupt_supply(1984, 9);
			let usdt_before = AssetStore::get(1984).unwrap();

			run();
			assert_eq!(supply(DOT_USD), 1_000);
			let usdt_after = AssetStore::get(1984).unwrap();
			assert_eq!(usdt_after.supply, 9);
			assert_eq!(usdt_after.status, usdt_before.status);
		});
	}

	#[test]
	fn many_accounts_with_runtime_mbm_weight() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[]);
			// 3000 accounts, far above today's 62.
			for i in 0u32..3_000 {
				let who: AccountId =
					sp_core::crypto::AccountId32::new(sp_io::hashing::blake2_256(&i.encode()));
				AssetsPallet::mint(
					RuntimeOrigin::signed(owner.clone()),
					DOT_USD.into(),
					MultiAddress::Id(who),
					(i as u128) + 1,
				)
				.expect("mint works");
			}
			let real = real_issuance(DOT_USD);
			assert_eq!(real, 3_000 * 3_001 / 2);
			corrupt_supply(DOT_USD, 1);

			let steps = run_with_limit(crate::MbmServiceWeight::get());
			assert_eq!(supply(DOT_USD), real);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
			println!("3000 accounts took {steps} steps at the runtime MBM weight");
		});
	}

	#[test]
	fn cursor_and_id_fit_the_pallet_limits() {
		use codec::MaxEncodedLen;
		let cursor = <TestCursor as MaxEncodedLen>::max_encoded_len();
		let id = <<Migration as SteppedMigration>::Identifier as MaxEncodedLen>::max_encoded_len();
		assert!(cursor as u32 <= <Runtime as pallet_migrations::Config>::CursorMaxLen::get());
		assert!(id as u32 <= <Runtime as pallet_migrations::Config>::IdentifierMaxLen::get());
	}

	#[test]
	fn runs_through_the_migrations_pallet() {
		use frame_support::{
			migrations::MultiStepMigrator,
			traits::{OnInitialize, OnRuntimeUpgrade},
		};
		new_test_ext().execute_with(|| {
			frame_system::Pallet::<Runtime>::set_block_number(1);
			let owner = acc(1);
			let balances: Vec<(AccountId, u128)> =
				(0u8..50).map(|n| (acc(100 + n), 10_000 + n as u128)).collect();
			setup_asset(DOT_USD, &owner, &balances);
			let real = real_issuance(DOT_USD);
			corrupt_supply(DOT_USD, 1);

			// Start all MBMs of the runtime, as the real upgrade does.
			crate::MultiBlockMigrations::on_runtime_upgrade();
			let mut blocks = 0;
			while crate::MultiBlockMigrations::ongoing() {
				blocks += 1;
				assert!(blocks < 1_000, "MBMs do not finish");
				let n = frame_system::Pallet::<Runtime>::block_number() + 1;
				frame_system::Pallet::<Runtime>::set_block_number(n);
				crate::MultiBlockMigrations::on_initialize(n);
				crate::MultiBlockMigrations::step();
			}

			// Not stuck, and ours is recorded as done.
			assert!(pallet_migrations::Cursor::<Runtime>::get().is_none());
			assert!(pallet_migrations::Historic::<Runtime>::contains_key(
				frame_support::BoundedVec::truncate_from(Migration::id().encode())
			));
			assert_eq!(supply(DOT_USD), real);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
			// No failure was reported.
			assert!(!frame_system::Pallet::<Runtime>::events().iter().any(|r| matches!(
				r.event,
				crate::RuntimeEvent::MultiBlockMigrations(
					pallet_migrations::Event::MigrationFailed { .. } |
						pallet_migrations::Event::UpgradeFailed
				)
			)));
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn try_runtime_hooks_pass() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 1_000u128), (acc(11), 2u128)]);
			corrupt_supply(DOT_USD, 1);

			let state = Migration::pre_upgrade().expect("pre_upgrade works");
			run_with_limit(limit_for(1));
			Migration::post_upgrade(state).expect("post_upgrade passes");
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn try_runtime_catches_a_wrong_supply() {
		new_test_ext().execute_with(|| {
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 1_000u128)]);
			corrupt_supply(DOT_USD, 1);
			let state = Migration::pre_upgrade().expect("pre_upgrade works");
			// Migration not run: the check must fail.
			assert!(Migration::post_upgrade(state).is_err());
		});
	}

	/// The pallet-assets events emitted for dotUSD, in order.
	fn freeze_events() -> Vec<&'static str> {
		frame_system::Pallet::<Runtime>::events()
			.into_iter()
			.filter_map(|r| match r.event {
				crate::RuntimeEvent::Assets(pallet_assets::Event::AssetFrozen { asset_id })
					if asset_id == DOT_USD =>
					Some("frozen"),
				crate::RuntimeEvent::Assets(pallet_assets::Event::AssetThawed { asset_id })
					if asset_id == DOT_USD =>
					Some("thawed"),
				_ => None,
			})
			.collect()
	}

	#[test]
	fn emits_frozen_and_thawed_events() {
		new_test_ext().execute_with(|| {
			frame_system::Pallet::<Runtime>::set_block_number(1);
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128), (acc(11), 10u128)]);

			let cursor = partial_step(1);
			assert_eq!(freeze_events(), vec!["frozen"]);
			assert!(step(cursor, Weight::MAX).is_none());
			assert_eq!(freeze_events(), vec!["frozen", "thawed"]);
		});
	}

	#[test]
	fn no_events_if_already_frozen() {
		new_test_ext().execute_with(|| {
			frame_system::Pallet::<Runtime>::set_block_number(1);
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);
			set_status(DOT_USD, AssetStatus::Frozen);

			run();
			assert!(freeze_events().is_empty());
			assert_eq!(status(DOT_USD), AssetStatus::Frozen);
		});
	}

	#[test]
	fn thawed_event_when_bailing_out() {
		new_test_ext().execute_with(|| {
			frame_system::Pallet::<Runtime>::set_block_number(1);
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);

			let cursor = partial_step(1);
			AssetStore::mutate(DOT_USD, |maybe| {
				maybe.as_mut().unwrap().accounts += 1;
			});
			assert!(step(cursor, Weight::MAX).is_none());
			assert_eq!(freeze_events(), vec!["frozen", "thawed"]);
			assert_eq!(status(DOT_USD), AssetStatus::Live);
		});
	}

	#[test]
	fn no_thawed_event_if_status_changed_while_counting() {
		new_test_ext().execute_with(|| {
			frame_system::Pallet::<Runtime>::set_block_number(1);
			let owner = acc(1);
			setup_asset(DOT_USD, &owner, &[(acc(10), 5_000u128)]);

			let cursor = partial_step(1);
			set_status(DOT_USD, AssetStatus::Destroying);
			assert!(step(cursor, Weight::MAX).is_none());
			// We did not touch the status, so we must not claim we thawed it.
			assert_eq!(freeze_events(), vec!["frozen"]);
		});
	}
}
