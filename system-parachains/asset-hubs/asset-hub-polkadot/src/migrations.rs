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
use crate::Runtime;
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

/// Default DAP budget allocation: 15% buffer, 85% staker rewards.
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
		// [dap (buffer), StakerRewardRecipient]
		let percentages = [Perbill::from_percent(15), Perbill::from_percent(85)];

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
	// DAP V1->V2: seed `BudgetAllocation` and `LastIssuanceTimestamp`, credit a one-shot
	// catch-up drip. Required when moving staking to non-minting mode (see SDK PR #11616).
	pallet_dap::migrations::MigrateV1ToV2<
		Runtime,
		DapLastIssuanceTimestamp,
		DefaultDapBudget,
		crate::dynamic_params::staking_election::MaxEraDuration,
	>,
);

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = pallet_xcm::migration::MigrateToLatestXcmVersion<Runtime>;

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
