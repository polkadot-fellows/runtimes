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

pub mod migration {
	#[cfg(feature = "try-runtime")]
	use crate::{vec, Vec};
	#[cfg(feature = "try-runtime")]
	use codec::Decode;
	use codec::Encode;
	use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
	use sp_core::Get;
	#[cfg(feature = "try-runtime")]
	use sp_runtime::TryRuntimeError;

	/// Test migration to verify XCM V4 to V5 compatibility for ForeignAssets and AssetConversion
	/// storage. This migration doesn't actually alter storage, it only verifies that:
	/// 1. XCM V4 encoded locations can be decoded as V5
	/// 2. Storage keys remain the same between V4 and V5
	/// 3. Re-encoding as V5 produces valid data
	pub struct TestXcmV4ToV5Compatibility<T>(core::marker::PhantomData<T>);

	impl<T> OnRuntimeUpgrade for TestXcmV4ToV5Compatibility<T>
	where
		T: frame_system::Config
			+ pallet_assets::Config<crate::ForeignAssetsInstance>
			+ pallet_asset_conversion::Config<
				PoolId = (
					<T as pallet_asset_conversion::Config>::AssetKind,
					<T as pallet_asset_conversion::Config>::AssetKind,
				),
			>,
		T::AssetKind: From<xcm::v5::Location> + Into<xcm::v5::Location>,
		T::PoolId: Into<(T::AssetKind, T::AssetKind)>,
		<T as pallet_assets::Config<crate::ForeignAssetsInstance>>::AssetId:
			From<xcm::v5::Location> + Into<xcm::v5::Location>,
	{
		fn on_runtime_upgrade() -> Weight {
			let mut weight = T::DbWeight::get().reads(1);

			log::info!("Starting XCM V4 to V5 compatibility test migration");

			// Test ForeignAssets storage items
			weight.saturating_accrue(Self::test_foreign_assets_compatibility());

			weight.saturating_accrue(Self::test_asset_conversion_compatibility());

			log::info!("XCM V4 to V5 compatibility test migration completed successfully");

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
			use frame_support::storage::StorageMap;

			log::info!("Pre-upgrade: Collecting storage info for XCM compatibility test");

			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), TryRuntimeError> {
			use frame_support::storage::StorageMap;

			// Test a few sample conversions to ensure compatibility
			Self::test_sample_location_conversions()?;

			log::info!("XCM V4 to V5 compatibility test migration validated successfully");
			Ok(())
		}
	}

	impl<T> TestXcmV4ToV5Compatibility<T>
	where
		T: frame_system::Config
			+ pallet_assets::Config<crate::ForeignAssetsInstance>
			+ pallet_asset_conversion::Config
			+ pallet_asset_conversion::Config<
				PoolId = (
					<T as pallet_asset_conversion::Config>::AssetKind,
					<T as pallet_asset_conversion::Config>::AssetKind,
				),
			>,
		T::PoolId: Into<(T::AssetKind, T::AssetKind)>,
		T::AssetKind: From<xcm::v5::Location> + Into<xcm::v5::Location>,
		<T as pallet_assets::Config<crate::ForeignAssetsInstance>>::AssetId:
			From<xcm::v5::Location> + Into<xcm::v5::Location>,
	{
		fn test_foreign_assets_compatibility() -> Weight {
			let mut weight = T::DbWeight::get().reads(1);
			let mut tested_assets = 0u32;

			// Test Asset storage items
			for (asset_id, _asset_details) in
				pallet_assets::Asset::<T, crate::ForeignAssetsInstance>::iter()
			{
				let v5_location: xcm::v5::Location = asset_id.clone().into();
				let v4_location: xcm::v4::Location =
					xcm::v4::Location::try_from(v5_location.clone()).unwrap();

				assert_eq!(
					v4_location.encode(),
					v5_location.encode(),
					"Asset ID conversion not stable for {:?}",
					asset_id
				);

				tested_assets += 1;
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 0));

				if tested_assets >= 100 {
					break; // Limit test to avoid excessive weight
				}
			}

			log::info!("Tested {} ForeignAssets for XCM compatibility", tested_assets);
			weight
		}

		fn test_asset_conversion_compatibility() -> Weight {
			let mut weight = T::DbWeight::get().reads(1);
			let mut tested_pools = 0u32;

			// Test Pools storage items
			for (pool_id, _pool_info) in pallet_asset_conversion::Pools::<T>::iter() {
				let (asset1, asset2) = pool_id;
				let v5_asset1: xcm::v5::Location = asset1.clone().into();
				let v5_asset2: xcm::v5::Location = asset2.clone().into();
				let v4_asset1: xcm::v4::Location =
					xcm::v4::Location::try_from(v5_asset1.clone()).unwrap();
				let v4_asset2: xcm::v4::Location =
					xcm::v4::Location::try_from(v5_asset2.clone()).unwrap();

				assert_eq!(
					v5_asset1.encode(),
					v4_asset1.encode(),
					"Pool asset1 conversion not stable for {:?}",
					asset1
				);
				assert_eq!(
					v5_asset2.encode(),
					v4_asset2.encode(),
					"Pool asset2 conversion not stable for {:?}",
					asset2
				);

				tested_pools += 1;
				weight.saturating_accrue(T::DbWeight::get().reads_writes(1, 0));

				if tested_pools >= 50 {
					break; // Limit test to avoid excessive weight
				}
			}

			log::info!("Tested {} AssetConversion pools for XCM compatibility", tested_pools);
			weight
		}

		#[cfg(feature = "try-runtime")]
		fn test_sample_location_conversions() -> Result<(), TryRuntimeError> {
			// Test some common XCM location patterns to ensure V4 -> V5 compatibility
			let test_locations_v4 = vec![
				// Relay chain
				xcm::v4::Location::new(1, xcm::v4::Junctions::Here),
				// Sibling parachain
				xcm::v4::Location::new(1, [xcm::v4::Junction::Parachain(1000)]),
				// Asset on sibling parachain
				xcm::v4::Location::new(
					1,
					[
						xcm::v4::Junction::Parachain(1000),
						xcm::v4::Junction::PalletInstance(50),
						xcm::v4::Junction::GeneralIndex(1984),
					],
				),
				// Global consensus location
				xcm::v4::Location::new(
					1,
					[xcm::v4::Junction::GlobalConsensus(xcm::v4::NetworkId::Polkadot)],
				),
			];

			for v4_location in test_locations_v4 {
				// Test V4 -> V5 conversion
				let v5_location = xcm::v5::Location::try_from(v4_location.clone())
					.map_err(|_| TryRuntimeError::Other("Failed to convert V4 location to V5"))?;

				// Test that we can encode/decode V5 location
				let encoded = v5_location.encode();
				let decoded = xcm::v5::Location::decode(&mut &encoded[..])
					.map_err(|_| TryRuntimeError::Other("Failed to decode V5 location"))?;

				frame_support::ensure!(
					v5_location == decoded,
					"V5 location encode/decode round-trip failed"
				);

				log::info!("Successfully tested V4 -> V5 conversion for: {:?}", v4_location);
			}

			Ok(())
		}
	}
}
