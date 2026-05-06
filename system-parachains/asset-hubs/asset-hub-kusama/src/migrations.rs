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

/// Unreleased migrations. Add new ones here:
pub type Unreleased = (
	RemoveAhMigratorPallet,
	cumulus_pallet_xcmp_queue::migration::v6::MigrateV5ToV6<crate::Runtime>,
	MigrateBountyAccountAssets,
);

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = pallet_xcm::migration::MigrateToLatestXcmVersion<crate::Runtime>;

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = (Unreleased, Permanent);

frame_support::parameter_types! {
	pub const AhMigratorPalletName: &'static str = "AhMigrator";

	/// Assets that must be moved from the old to the new bounty pot account by
	/// [`MigrateBountyAccountAssets`]. Restricted to USDC (1337) — KSM, USDT and
	/// RMRK are intentionally left at the old derivation.
	pub BountyMigrationAssets: alloc::vec::Vec<xcm::latest::Location> =
		alloc::vec![xcm::latest::Location::new(
			0,
			[
				xcm::latest::Junction::PalletInstance(
					crate::xcm_config::TrustBackedAssetsPalletIndex::get(),
				),
				xcm::latest::Junction::GeneralIndex(1337),
			],
		)];
}

pub type RemoveAhMigratorPallet = frame_support::migrations::RemovePallet<
	AhMigratorPalletName,
	<crate::Runtime as frame_system::Config>::DbWeight,
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
/// accounts. This is not theoretical: a Kusama referendum funding a bounty with
/// USDT is expected to enact shortly before this runtime ships.
pub struct MigrateBountyAccountAssets;
impl frame_support::traits::OnRuntimeUpgrade for MigrateBountyAccountAssets {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		use frame_support::traits::Get;
		use pallet_bounties::TransferAllAssets;
		use sp_runtime::traits::AccountIdConversion;

		let pallet_id = <crate::Runtime as pallet_treasury::Config>::PalletId::get();
		let assets_per_bounty = BountyMigrationAssets::get().len() as u64;

		type Transferer = pallet_bounties::TransferAllFungibles<
			crate::AccountId,
			crate::NativeAndAssets,
			BountyMigrationAssets,
		>;

		let db_weight = <crate::Runtime as frame_system::Config>::DbWeight::get();
		let mut weight = frame_support::weights::Weight::zero();

		for bounty_id in pallet_multi_asset_bounties::Bounties::<crate::Runtime>::iter_keys() {
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
			pallet_multi_asset_bounties::ChildBounties::<crate::Runtime>::iter_keys()
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

#[cfg(not(feature = "runtime-benchmarks"))]
pub use multiblock_migrations::MbmMigrations;

#[cfg(not(feature = "runtime-benchmarks"))]
mod multiblock_migrations {
	use crate::{
		xcm_config::bridging::to_polkadot::{AssetHubPolkadot, DotLocation, EthereumEcosystem},
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
			AssetHubKusamaForeignAssetsReservesProvider,
		>,
		pallet_assets_precompiles::MigrateForeignAssetPrecompileMappings<
			Runtime,
			ForeignAssetsInstance,
			pallet_assets_precompiles::weights::SubstrateWeight<Runtime>,
		>,
	);

	/// This type provides reserves information for `asset_id`. Meant to be used in a migration
	/// running on the Asset Hub Kusama upgrade which changes the Foreign Assets reserve-transfers
	/// and teleports from hardcoded rules to per-asset configured reserves.
	///
	/// The hardcoded rules (see `xcm_config.rs`) migrated here:
	/// 1. Foreign Assets native to sibling parachains are teleportable between the asset's native
	///    chain and Asset Hub ==> `ForeignAssetReserveData { reserve: "Asset's native chain",
	///    teleport: true }`
	/// 2. Foreign assets native to Ethereum Ecosystem have Polkadot Asset Hub as trusted reserve
	///    ==> `ForeignAssetReserveData { reserve: "Asset Hub Polkadot", teleport: false }`
	/// 3. Foreign assets native to Polkadot Ecosystem have Asset Hub Polkadot as trusted reserve
	///    ==> `ForeignAssetReserveData { reserve: "Asset Hub Polkadot", teleport: false }`
	pub struct AssetHubKusamaForeignAssetsReservesProvider;
	impl ForeignAssetsReservesProvider for AssetHubKusamaForeignAssetsReservesProvider {
		type ReserveData = ForeignAssetReserveData;
		fn reserves_for(asset_id: &Location) -> Vec<Self::ReserveData> {
			let reserves = if StartsWith::<DotLocation>::contains(asset_id) {
				// rule 3: Polkadot asset, Asset Hub Polkadot reserve, non teleportable
				vec![(AssetHubPolkadot::get(), false).into()]
			} else if StartsWith::<EthereumEcosystem>::contains(asset_id) {
				// rule 2: Ethereum asset, Asset Hub Polkadot reserve, non teleportable
				vec![(AssetHubPolkadot::get(), false).into()]
			} else {
				match asset_id.unpack() {
					(1, interior) => {
						match interior.first() {
							Some(Junction::Parachain(sibling_para_id))
								if sibling_para_id.ne(
									&kusama_runtime_constants::system_parachain::ASSET_HUB_ID,
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
					target: "runtime::AssetHubKusamaForeignAssetsReservesProvider::reserves_for",
					"unexpected asset id {asset_id:?}",
				);
			}
			reserves
		}

		#[cfg(feature = "try-runtime")]
		fn check_reserves_for(asset_id: &Location, reserves: Vec<Self::ReserveData>) -> bool {
			if StartsWith::<DotLocation>::contains(asset_id) {
				let expected = ForeignAssetReserveData {
					reserve: AssetHubPolkadot::get(),
					teleportable: false,
				};
				// rule 3: Polkadot asset
				reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
			} else if StartsWith::<EthereumEcosystem>::contains(asset_id) {
				let expected = ForeignAssetReserveData {
					reserve: AssetHubPolkadot::get(),
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
									&kusama_runtime_constants::system_parachain::ASSET_HUB_ID,
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
