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

//! The runtime migrations per release.
use crate::{Runtime, TrustBackedAssetsInstance};
#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
use frame_support::parameter_types;

/// Provides the initial `LastIssuanceTimestamp` for the DAP V1->V2 migration.
///
/// Uses the start of the active era (ms since unix epoch) so the catch-up drip covers
/// the gap between the last era boundary and the migration. Falls back to 0 (no catch-up)
/// if no era is active.
pub struct DapLastIssuanceTimestamp;
impl frame_support::traits::Get<u64> for DapLastIssuanceTimestamp {
	fn get() -> u64 {
		pallet_staking_async::ActiveEra::<Runtime>::get()
			.and_then(|era| era.start)
			.unwrap_or(0)
	}
}

/// Default DAP budget allocation: 15% buffer, 85% staker rewards, 0% validator incentive.
///
/// Matches the previous `EraPayout` split (15% treasury / 85% stakers), now enforced
/// at the DAP drip level instead of at era payout time. The 15% share initially
/// accumulates in the DAP buffer and can be redirected by governance.
pub struct DefaultDapBudget;
impl frame_support::traits::Get<pallet_dap::BudgetAllocationMap> for DefaultDapBudget {
	fn get() -> pallet_dap::BudgetAllocationMap {
		use sp_runtime::Perbill;
		use sp_staking::budget::BudgetRecipientList;

		let recipients = <Runtime as pallet_dap::Config>::BudgetRecipients::recipients();
		// Order matches `pallet_dap::Config::BudgetRecipients`:
		// [dap (buffer), StakerRewardRecipient, ValidatorIncentiveRecipient]
		let percentages =
			[Perbill::from_percent(15), Perbill::from_percent(85), Perbill::from_percent(0)];

		let mut map = pallet_dap::BudgetAllocationMap::new();
		for ((key, _), perbill) in recipients.into_iter().zip(percentages) {
			let _ = map.try_insert(key, perbill);
		}
		map
	}
}

parameter_types! {
	// Account `15jAYzPdLorBGAj4LLGaqohpzpw4mEohVkzszNpaBPbnDaXn` (Nomination Pool #296)
	// has trapped funds on PAH. See issue: https://github.com/paritytech/polkadot-sdk/issues/10993.
	pub TrappedBalanceMember: crate::AccountId = crate::AccountId::from(
		hex_literal::hex!("d11964e74f0571827c231ee07fc7268fc835499db3a0089c9e6f02c2435f50fc")
	);
}

parameter_types! {
	pub const AhMigratorPalletName: &'static str = "AhMigrator";
	pub const StateTrieMigrationName: &'static str = "StateTrieMigration";
}

pub type RemoveAhMigratorPallet = frame_support::migrations::RemovePallet<
	AhMigratorPalletName,
	<Runtime as frame_system::Config>::DbWeight,
>;

/// Remove the `StateTrieMigration` pallet's storage. The state trie migration on Asset Hub
/// Polkadot is complete and the pallet has been removed from the runtime, see
/// <https://github.com/polkadot-fellows/runtimes/issues/905>.
pub type RemoveStateTrieMigrationPallet = frame_support::migrations::RemovePallet<
	StateTrieMigrationName,
	<Runtime as frame_system::Config>::DbWeight,
>;
/// Moves the funds of every `pallet-multi-asset-bounties` bounty and child-bounty
/// from the previous account derivation to the new one introduced by
/// <https://github.com/paritytech/polkadot-sdk/pull/11052>.
///
/// Until v2.2.2 the local wrapper in `system-parachains-common` derived the
/// bounty pot accounts as
/// `Treasury::PalletId.into_sub_account_truncating(("mbt", id))`, with `"mbt"`
/// passed as a `&str` (SCALE-encoded as a length-prefixed sequence). Starting
/// from `pallet-multi-asset-bounties` 0.4.0 the prefix is supplied as a fixed
/// `[u8; 3]` (`*b"mbt"`), which encodes as 3 raw bytes — a different seed and
/// therefore a different sub-account. Same story for child bounties (`"mcb"`).
///
/// Without this migration, any funds sitting at the old (`&str`-derived)
/// accounts at the moment of the runtime upgrade would no longer be reachable
/// by the pallet, which after the upgrade only knows the new (`[u8; 3]`-derived)
/// accounts.
///
/// Reuses the runtime's `pallet_bounties::Config::TransferAllAssets`, which
/// sweeps every asset listed in `treasury::BountyRelevantAssets` (DOT, USDT,
/// USDC, on PAH). Native account collisions with the legacy bounties
/// pallet are not possible — legacy uses `"bt"`/`"cb"` prefixes, multi-asset
/// uses `"mbt"`/`"mcb"`, so the derived accounts are disjoint.
pub struct MigrateBountyAccountAssets;
impl frame_support::traits::OnRuntimeUpgrade for MigrateBountyAccountAssets {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		use frame_support::traits::Get;
		use pallet_bounties::TransferAllAssets;
		use sp_runtime::traits::AccountIdConversion;

		let pallet_id = <Runtime as pallet_treasury::Config>::PalletId::get();
		let assets_per_bounty = crate::treasury::BountyRelevantAssets::get().len() as u64;

		type Transferer = <Runtime as pallet_bounties::Config>::TransferAllAssets;

		let db_weight = <Runtime as frame_system::Config>::DbWeight::get();
		let mut weight = frame_support::weights::Weight::zero();

		for bounty_id in pallet_multi_asset_bounties::Bounties::<Runtime>::iter_keys() {
			// Old: `&str "mbt"` (length-prefixed encoding).
			let old: crate::AccountId = pallet_id.into_sub_account_truncating(("mbt", bounty_id));
			// New: `[u8; 3] *b"mbt"` (raw 3 bytes).
			let new: crate::AccountId = pallet_id.into_sub_account_truncating((
				pallet_multi_asset_bounties::BountyAccountPrefix::get(),
				bounty_id,
			));
			let _ = Transferer::force_transfer_all_assets(&old, &new);
			// `TransferAllFungibles` iterates the relevant assets twice and does at
			// most one read + one write per asset.
			weight = weight.saturating_add(
				db_weight.reads_writes(2 * assets_per_bounty, 2 * assets_per_bounty),
			);
		}

		for (parent_id, child_id) in
			pallet_multi_asset_bounties::ChildBounties::<Runtime>::iter_keys()
		{
			let old: crate::AccountId =
				pallet_id.into_sub_account_truncating(("mcb", parent_id, child_id));
			let new: crate::AccountId = pallet_id.into_sub_account_truncating((
				pallet_multi_asset_bounties::ChildBountyAccountPrefix::get(),
				parent_id,
				child_id,
			));
			let _ = Transferer::force_transfer_all_assets(&old, &new);
			weight = weight.saturating_add(
				db_weight.reads_writes(2 * assets_per_bounty, 2 * assets_per_bounty),
			);
		}

		weight
	}
}

/// Creates the PGAS asset with [`pallet_assets::Pallet::force_create`] from the root origin.
///
/// [`indiv_pallet_pgas::migration::CreatePgasAsset`] is not usable here: it creates the asset
/// through `fungibles::Create`, an unprivileged path that must follow `AssetIdAllocator`, so with
/// `AutoIncAssetId` the only id it may use is `NextAssetId`, never
/// [`crate::individuality::PGAS_ASSET_ID`]. `force_create` from `ForceOrigin` may pick any unused
/// id instead (<https://github.com/paritytech/polkadot-sdk/pull/12378>).
///
/// `PGAS_ASSET_ID` lies below the sequence (`NextAssetId` started at `50_000_000` and is
/// `50_000_548` at the time of writing), so `AutoIncAssetId::advance_from` leaves `NextAssetId`
/// untouched and permissionless asset creation is not affected.
///
/// Idempotent: a no-op if the asset already exists.
pub struct ForceCreatePgasAsset;
impl frame_support::traits::OnRuntimeUpgrade for ForceCreatePgasAsset {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		const LOG_TARGET: &str = "runtime::asset-hub-polkadot::migrations";
		type Assets = pallet_assets::Pallet<Runtime, TrustBackedAssetsInstance>;
		type AssetsWeightInfo =
			<Runtime as pallet_assets::Config<TrustBackedAssetsInstance>>::WeightInfo;

		let db_weight = <Runtime as frame_system::Config>::DbWeight::get();
		let asset_id = <Runtime as indiv_pallet_pgas::Config>::PgasAssetId::get();

		if pallet_assets::Asset::<Runtime, TrustBackedAssetsInstance>::contains_key(asset_id) {
			log::info!(target: LOG_TARGET, "PGAS asset already exists; skipping.");
			return db_weight.reads(1);
		}

		// Same asset as `indiv_pallet_pgas::Pallet::do_create_pgas_asset` would create: owned and
		// administered by `PgasAdmin`, sufficient, with `PgasMinBalance` as the minimum balance.
		match Assets::force_create(
			crate::RuntimeOrigin::root(),
			asset_id.into(),
			<Runtime as indiv_pallet_pgas::Config>::PgasAdmin::get().into(),
			true,
			<Runtime as indiv_pallet_pgas::Config>::PgasMinBalance::get(),
		) {
			Ok(()) => log::info!(target: LOG_TARGET, "PGAS asset created."),
			Err(e) => log::error!(target: LOG_TARGET, "failed to create PGAS asset: {e:?}"),
		}

		// `force_create` is benchmarked with a single `Asset` write; `AutoIncAssetId::advance_from`
		// adds a `NextAssetId` read and, as `PGAS_ASSET_ID` is below the sequence, no write.
		db_weight
			.reads(1)
			.saturating_add(<AssetsWeightInfo as pallet_assets::WeightInfo>::force_create())
			.saturating_add(db_weight.reads(1))
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		let details = pallet_assets::Asset::<Runtime, TrustBackedAssetsInstance>::get(
			<Runtime as indiv_pallet_pgas::Config>::PgasAssetId::get(),
		)
		.ok_or("PGAS asset must exist after migration")?;
		frame_support::ensure!(
			details.owner == <Runtime as indiv_pallet_pgas::Config>::PgasAdmin::get(),
			"PGAS asset must be owned by `PgasAdmin`"
		);
		frame_support::ensure!(details.is_sufficient, "PGAS asset must be sufficient");
		frame_support::ensure!(
			details.min_balance == <Runtime as indiv_pallet_pgas::Config>::PgasMinBalance::get(),
			"PGAS asset must use `PgasMinBalance`"
		);
		Ok(())
	}
}

/// Unreleased migrations. Add new ones here:
pub type Unreleased = (
	// no-op if member has no trapped balance, so second run is safe.
	pallet_nomination_pools::migration::unversioned::ClaimTrappedBalance<
		Runtime,
		TrappedBalanceMember,
	>,
	RemoveAhMigratorPallet,
	RemoveStateTrieMigrationPallet,
	// Remove an old staking value.
	crate::staking::RemoveMarchTIValue,
	cumulus_pallet_xcmp_queue::migration::v6::MigrateV5ToV6<Runtime>,
	cumulus_pallet_xcmp_queue::migration::v7::MigrateV6ToV7<Runtime>,
	cumulus_pallet_parachain_system::migration::Migration<Runtime>,
	// DAP V1->V2: seed `BudgetAllocation` and `LastIssuanceTimestamp`, credit a one-shot
	// catch-up drip. Required when moving staking to non-minting mode (see SDK PR #11616).
	pallet_dap::migrations::MigrateV1ToV2<
		Runtime,
		DapLastIssuanceTimestamp,
		DefaultDapBudget,
		crate::dynamic_params::staking_election::MaxEraDuration,
	>,
	MigrateBountyAccountAssets,
	ForceCreatePgasAsset,
	pallet_staking_async::migrations::SetWeightedPointsFormulaStartEra<Runtime>,
);

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = Unreleased;

#[cfg(not(feature = "runtime-benchmarks"))]
pub use multiblock_migrations::MbmMigrations;

#[cfg(not(feature = "runtime-benchmarks"))]
mod multiblock_migrations {
	use crate::{
		xcm_config::bridging::{
			to_ethereum::EthereumLocation,
			to_kusama::{AssetHubKusama, KsmLocation},
		},
		*,
	};
	use alloc::{vec, vec::Vec};
	use assets_common::{
		local_and_foreign_assets::ForeignAssetReserveData,
		migrations::foreign_assets_reserves::ForeignAssetsReservesProvider,
	};
	use frame_support::traits::Contains;
	use xcm::v5::{Junction, Location};
	use xcm_builder::StartsWith;

	/// MBM migrations to apply on runtime upgrade.
	pub type MbmMigrations = (
		assets_common::migrations::foreign_assets_reserves::ForeignAssetsReservesMigration<
			Runtime,
			ForeignAssetsInstance,
			AssetHubPolkadotForeignAssetsReservesProvider,
		>,
		pallet_assets_precompiles::MigrateForeignAssetPrecompileMappings<
			Runtime,
			ForeignAssetsInstance,
			pallet_assets_precompiles::weights::SubstrateWeight<Runtime>,
		>,
		// Not added: we do it with a manual TX
		//pallet_revive::migrations::v3::Migration<Runtime>,
		// Mandatory companion to `pallet_revive::Config::Deposit` becoming
		// `PGasDeposit`.
		pallet_revive::migrations::v4::Migration<Runtime>,
	);

	/// This type provides reserves information for `asset_id`. Meant to be used in a migration
	/// running on the Asset Hub Polkadot upgrade which changes the Foreign Assets
	/// reserve-transfers and teleports from hardcoded rules to per-asset configured reserves.
	///
	/// The hardcoded rules (see `xcm_config.rs`) migrated here:
	/// 1. Foreign Assets native to sibling parachains are teleportable between the asset's native
	///    chain and Asset Hub ==> `ForeignAssetReserveData { reserve: "Asset's native chain",
	///    teleport: true }`
	/// 2. Foreign assets native to Ethereum Ecosystem have Ethereum as trusted reserve. ==>
	///    `ForeignAssetReserveData { reserve: "Ethereum", teleport: false }`
	/// 3. Foreign assets native to Kusama Ecosystem have Asset Hub Kusama as trusted reserve. ==>
	///    `ForeignAssetReserveData { reserve: "Asset Hub Kusama", teleport: false }`
	pub struct AssetHubPolkadotForeignAssetsReservesProvider;
	impl ForeignAssetsReservesProvider for AssetHubPolkadotForeignAssetsReservesProvider {
		type ReserveData = ForeignAssetReserveData;
		fn reserves_for(asset_id: &Location) -> Vec<Self::ReserveData> {
			let reserves = if StartsWith::<KsmLocation>::contains(asset_id) {
				// rule 3: Kusama asset, Asset Hub Kusama reserve, non teleportable
				vec![(AssetHubKusama::get(), false).into()]
			} else if StartsWith::<EthereumLocation>::contains(asset_id) {
				// rule 2: Ethereum asset, Ethereum reserve, non teleportable
				vec![(EthereumLocation::get(), false).into()]
			} else {
				match asset_id.unpack() {
					(1, interior) => {
						match interior.first() {
							Some(Junction::Parachain(sibling_para_id))
								if sibling_para_id.ne(
									&polkadot_runtime_constants::system_parachain::ASSET_HUB_ID,
								) =>
							{
								// rule 1: sibling parachain asset, sibling parachain reserve,
								// teleportable
								vec![ForeignAssetReserveData {
									reserve: Location::new(
										1,
										Junction::Parachain(*sibling_para_id),
									),
									teleportable: true,
								}]
							},
							_ => vec![],
						}
					},
					_ => vec![],
				}
			};
			if reserves.is_empty() {
				log::error!(
					target: "runtime::AssetHubPolkadotForeignAssetsReservesProvider::reserves_for",
					"unexpected asset id {asset_id:?}",
				);
			}
			reserves
		}

		#[cfg(feature = "try-runtime")]
		fn check_reserves_for(asset_id: &Location, reserves: Vec<Self::ReserveData>) -> bool {
			if StartsWith::<KsmLocation>::contains(asset_id) {
				let expected =
					ForeignAssetReserveData { reserve: AssetHubKusama::get(), teleportable: false };
				// rule 3: Kusama asset
				reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
			} else if StartsWith::<EthereumLocation>::contains(asset_id) {
				let expected = ForeignAssetReserveData {
					reserve: EthereumLocation::get(),
					teleportable: false,
				};
				// rule 2: Ethereum asset
				reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
			} else {
				match asset_id.unpack() {
					(1, interior) => {
						match interior.first() {
							Some(Junction::Parachain(sibling_para_id))
								if sibling_para_id.ne(
									&polkadot_runtime_constants::system_parachain::ASSET_HUB_ID,
								) =>
							{
								let expected = ForeignAssetReserveData {
									reserve: Location::new(
										1,
										Junction::Parachain(*sibling_para_id),
									),
									teleportable: true,
								};
								// rule 1: sibling parachain asset
								reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
							},
							// unexpected asset
							_ => false,
						}
					},
					// unexpected asset
					_ => false,
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{
		individuality::{PgasAdmin, PgasMinBalance, PGAS_ASSET_ID},
		AccountId, AssetDeposit, Balances, ExistentialDeposit, RuntimeGenesisConfig, RuntimeOrigin,
	};
	use frame_support::{assert_noop, assert_ok, traits::OnRuntimeUpgrade};
	use sp_runtime::BuildStorage;

	type Assets = pallet_assets::Pallet<Runtime, TrustBackedAssetsInstance>;
	type Asset = pallet_assets::Asset<Runtime, TrustBackedAssetsInstance>;
	type NextAssetId = pallet_assets::NextAssetId<Runtime, TrustBackedAssetsInstance>;

	/// `NextAssetId` on Asset Hub Polkadot at the time of writing, just as an example.
	const EXAMPLE_LIVE_NEXT_ASSET_ID: u32 = 50_000_548;

	fn new_test_ext() -> sp_io::TestExternalities {
		sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
		)
	}

	fn assert_pgas_asset_matches_pallet_config() {
		let details = Asset::get(PGAS_ASSET_ID).expect("PGAS asset exists");
		assert_eq!(details.owner, PgasAdmin::get());
		assert_eq!(details.admin, PgasAdmin::get());
		assert!(details.is_sufficient);
		assert_eq!(details.min_balance, PgasMinBalance::get());
	}

	#[test]
	fn force_create_pgas_asset_leaves_next_asset_id_untouched() {
		new_test_ext().execute_with(|| {
			NextAssetId::put(EXAMPLE_LIVE_NEXT_ASSET_ID);

			let _ = ForceCreatePgasAsset::on_runtime_upgrade();
			assert_pgas_asset_matches_pallet_config();
			assert_eq!(NextAssetId::get(), Some(EXAMPLE_LIVE_NEXT_ASSET_ID));

			// Idempotent.
			let _ = ForceCreatePgasAsset::on_runtime_upgrade();
			assert_pgas_asset_matches_pallet_config();
			assert_eq!(NextAssetId::get(), Some(EXAMPLE_LIVE_NEXT_ASSET_ID));
		});
	}

	#[test]
	fn permissionless_create_is_unaffected_by_pgas_asset() {
		new_test_ext().execute_with(|| {
			NextAssetId::put(EXAMPLE_LIVE_NEXT_ASSET_ID);
			let _ = ForceCreatePgasAsset::on_runtime_upgrade();

			let creator = AccountId::from([1u8; 32]);
			assert_ok!(Balances::force_set_balance(
				RuntimeOrigin::root(),
				creator.clone().into(),
				AssetDeposit::get() + ExistentialDeposit::get(),
			));

			// Ids below the sequence, PGAS's neighbourhood included, stay out of reach.
			assert_noop!(
				Assets::create(
					RuntimeOrigin::signed(creator.clone()),
					(PGAS_ASSET_ID - 1).into(),
					creator.clone().into(),
					1,
				),
				pallet_assets::Error::<Runtime, TrustBackedAssetsInstance>::BadAssetId
			);
			// The next permissionless asset gets the same id it would have gotten before.
			assert_ok!(Assets::create(
				RuntimeOrigin::signed(creator.clone()),
				EXAMPLE_LIVE_NEXT_ASSET_ID.into(),
				creator.into(),
				1,
			));
			assert_eq!(NextAssetId::get(), Some(EXAMPLE_LIVE_NEXT_ASSET_ID + 1));
		});
	}

	#[test]
	fn force_create_pgas_asset_preserves_absent_next_asset_id() {
		new_test_ext().execute_with(|| {
			NextAssetId::kill();

			let _ = ForceCreatePgasAsset::on_runtime_upgrade();
			assert_pgas_asset_matches_pallet_config();
			assert!(!NextAssetId::exists());
		});
	}
}
