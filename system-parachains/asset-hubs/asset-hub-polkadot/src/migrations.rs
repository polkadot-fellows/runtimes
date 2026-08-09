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
}

pub type RemoveAhMigratorPallet = frame_support::migrations::RemovePallet<
	AhMigratorPalletName,
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

/// Creates PGAS while the `pallet-assets` auto-increment guard is suspended.
pub struct CreatePgasAssetWithSuspendedAssetIds;
impl frame_support::traits::OnRuntimeUpgrade for CreatePgasAssetWithSuspendedAssetIds {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let next_asset_id =
			pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::take();
		let weight = indiv_pallet_pgas::migration::CreatePgasAsset::<Runtime>::on_runtime_upgrade();
		if let Some(next_asset_id) = next_asset_id {
			pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::put(next_asset_id);
		}

		weight.saturating_add(<Runtime as frame_system::Config>::DbWeight::get().reads_writes(1, 2))
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		frame_support::ensure!(
			pallet_assets::Asset::<Runtime, TrustBackedAssetsInstance>::contains_key(
				crate::individuality::PGAS_ASSET_ID
			),
			"PGAS asset must exist after migration"
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
	// Remove an old staking value.
	crate::staking::RemoveMarchTIValue,
	cumulus_pallet_xcmp_queue::migration::v6::MigrateV5ToV6<Runtime>,
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
	// Creates the PGAS asset under the pallet-derived admin account. `pallet-pgas` cannot mint
	// until it exists.
	//
	// `NextAssetId` rejects a requested id while it is `Some` and does not match that value. The
	// wrapper takes the value before creation and restores it immediately after. The take and put
	// are atomic within this upgrade block, and a fresh chain with no value round-trips as `None`.
	// The guard is the only obstacle to this fixed-id creation.
	//
	// Once the SDK pin includes <https://github.com/paritytech/polkadot-sdk/pull/12378>, replace
	// this wrapper with a bounded allocator such as the `ReservedFloorAllocator` sketch from the
	// review: it must reserve ids greater than or equal to `PGAS_ASSET_ID` by rule, not distance.
	CreatePgasAssetWithSuspendedAssetIds,
);

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{individuality::PGAS_ASSET_ID, RuntimeGenesisConfig};
	use frame_support::traits::OnRuntimeUpgrade;
	use sp_runtime::BuildStorage;

	#[test]
	fn pgas_asset_exists_after_create_pgas_asset_migration() {
		let mut ext = sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
		);
		ext.execute_with(|| {
			let next_asset_id = 50_000_000u32;
			pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::put(next_asset_id);
			let _ = CreatePgasAssetWithSuspendedAssetIds::on_runtime_upgrade();
			assert!(pallet_assets::Asset::<Runtime, TrustBackedAssetsInstance>::contains_key(
				PGAS_ASSET_ID
			));
			assert_eq!(
				pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::get(),
				Some(next_asset_id)
			);

			let _ = CreatePgasAssetWithSuspendedAssetIds::on_runtime_upgrade();
			assert!(pallet_assets::Asset::<Runtime, TrustBackedAssetsInstance>::contains_key(
				PGAS_ASSET_ID
			));
			assert_eq!(
				pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::get(),
				Some(next_asset_id)
			);
		});
	}

	#[test]
	fn pgas_asset_migration_preserves_absent_next_asset_id() {
		let mut ext = sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
		);
		ext.execute_with(|| {
			pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::kill();
			let _ = CreatePgasAssetWithSuspendedAssetIds::on_runtime_upgrade();
			assert!(pallet_assets::Asset::<Runtime, TrustBackedAssetsInstance>::contains_key(
				PGAS_ASSET_ID
			));
			assert!(!pallet_assets::NextAssetId::<Runtime, TrustBackedAssetsInstance>::exists());
		});
	}
}

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = (Unreleased, Permanent);

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
		//
		// Mandatory companion to `pallet_revive::Config::Deposit` becoming
		// `PGasDeposit` (see `lib.rs`). It records every existing code-upload deposit in
		// `NativeDepositOf` and converts each contract's native `StorageDepositReserve` hold into
		// PGAS. Without it, `refund_on_hold` finds no `NativeDepositOf` credit and no PGAS on hold
		// for contracts deployed before the switch, so `settle_pgas_refund` caps the refund at
		// zero: partial storage-deposit refunds would silently return nothing and leave the
		// native hold stuck. Its phases 1 and 2 are no-ops unless `Deposit` supports PGAS, so it
		// must not be added before that switch, and both must ship together.
		//
		// The `version_from: 3` in its `MigrationId` is only part of the identifier —
		// `pallet-migrations` gates on whether that id is already in `Historic`, not on a version
		// chain — so this runs even though revive's v3 MBM above was skipped in favour of a manual
		// transaction.
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
