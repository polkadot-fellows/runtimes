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

/// Unreleased migrations. Add new ones here:
pub type Unreleased = ();

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>;

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = (Unreleased, Permanent);

/// MBM migrations to apply on runtime upgrade.
pub type MbmMigrations =
	assets_common::migrations::foreign_assets_reserves::ForeignAssetsReservesMigration<
		Runtime,
		ForeignAssetsInstance,
		AssetHubKusamaForeignAssetsReservesProvider,
	>;

/// This type provides reserves information for `asset_id`. Meant to be used in a migration running
/// on the Asset Hub Kusama upgrade which changes the Foreign Assets reserve-transfers and
/// teleports from hardcoded rules to per-asset configured reserves.
///
/// The hardcoded rules (see `xcm_config.rs`) migrated here:
/// 1. Foreign Assets native to sibling parachains are teleportable between the asset's native chain
///    and Asset Hub.
///  ----> `ForeignAssetReserveData { reserve: "Asset's native chain", teleport: true }`
/// 2. Foreign assets native to Ethereum Ecosystem have Polkadot Asset Hub as trusted reserve.
///  ----> `ForeignAssetReserveData { reserve: "Asset Hub Polkadot", teleport: false }`
/// 3. Foreign assets native to Polkadot Ecosystem have Asset Hub Polkadot as trusted reserve.
///  ----> `ForeignAssetReserveData { reserve: "Asset Hub Polkadot", teleport: false }`
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
							if sibling_para_id
								.ne(&kusama_runtime_constants::system_parachain::ASSET_HUB_ID) =>
						{
							// rule 1: sibling parachain asset, sibling parachain reserve,
							// teleportable
							vec![ForeignAssetReserveData {
								reserve: Location::new(1, Junction::Parachain(*sibling_para_id)),
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
				"unexpected asset id {:?}", asset_id,
			);
		}
		reserves
	}

	#[cfg(feature = "try-runtime")]
	fn check_reserves_for(asset_id: &Location, reserves: Vec<Self::ReserveData>) -> bool {
		if StartsWith::<DotLocation>::contains(asset_id) {
			let expected =
				ForeignAssetReserveData { reserve: AssetHubPolkadot::get(), teleportable: false };
			// rule 3: Polkadot asset
			reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
		} else if StartsWith::<EthereumEcosystem>::contains(asset_id) {
			let expected =
				ForeignAssetReserveData { reserve: AssetHubPolkadot::get(), teleportable: false };
			// rule 2: Ethereum asset
			reserves.len() == 1 && expected.eq(reserves.get(0).unwrap())
		} else {
			match asset_id.unpack() {
				(1, interior) => {
					match interior.first() {
						Some(Junction::Parachain(sibling_para_id))
							if sibling_para_id
								.ne(&kusama_runtime_constants::system_parachain::ASSET_HUB_ID) =>
						{
							let expected = ForeignAssetReserveData {
								reserve: Location::new(1, Junction::Parachain(*sibling_para_id)),
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
