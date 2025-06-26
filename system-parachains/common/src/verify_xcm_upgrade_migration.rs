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
	use codec::{Decode, Encode};
	use frame_support::traits::OnRuntimeUpgrade;
	#[cfg(feature = "try-runtime")]
	use sp_runtime::TryRuntimeError;

	#[cfg(feature = "try-runtime")]
	const LOG_TARGET: &str = "runtime::verify_xcm_upgrade_migration";

	mod asset_as_v4 {
		use frame_support::{storage_alias, Blake2_128Concat};

		#[storage_alias]
		pub type Asset<T: frame_system::Config + pallet_assets::Config<I>, I: 'static> = StorageMap<
			pallet_assets::Pallet<T, I>,
			Blake2_128Concat,
			xcm::v4::Location,
			pallet_assets::AssetDetails<
				<T as pallet_assets::Config<I>>::Balance,
				<T as frame_system::Config>::AccountId,
				pallet_assets::DepositBalanceOf<T, I>,
			>,
		>;

		#[storage_alias]
		pub type Pools<T: pallet_asset_conversion::Config> = StorageMap<
			pallet_asset_conversion::Pallet<T>,
			Blake2_128Concat,
			<T as pallet_asset_conversion::Config>::PoolId,
			pallet_asset_conversion::PoolInfo<xcm::v4::Location>,
		>;
	}

	mod asset_as_v5 {
		use frame_support::{storage_alias, Blake2_128Concat};

		#[storage_alias]
		pub type Asset<T: frame_system::Config + pallet_assets::Config<I>, I: 'static> = StorageMap<
			pallet_assets::Pallet<T, I>,
			Blake2_128Concat,
			xcm::v5::Location,
			pallet_assets::AssetDetails<
				<T as pallet_assets::Config<I>>::Balance,
				<T as frame_system::Config>::AccountId,
				pallet_assets::DepositBalanceOf<T, I>,
			>,
		>;

		#[storage_alias]
		pub type Pools<T: pallet_asset_conversion::Config> = StorageMap<
			pallet_asset_conversion::Pallet<T>,
			Blake2_128Concat,
			<T as pallet_asset_conversion::Config>::PoolId,
			pallet_asset_conversion::PoolInfo<xcm::v5::Location>,
		>;
	}

	/// Test migration to verify XCM V4 to V5 compatibility for ForeignAssets and AssetConversion
	/// storage. This migration doesn't actually alter storage, it only verifies that:
	/// 1. XCM V4 encoded locations can be decoded as V5
	/// 2. Location encoding remains the same between V4 and V5
	pub struct VerifyXcmV4ToV5Compatibility<T, I = crate::ForeignAssetsInstance>(
		core::marker::PhantomData<(T, I)>,
	);

	impl<T, I> OnRuntimeUpgrade for VerifyXcmV4ToV5Compatibility<T, I>
	where
		T: frame_system::Config + pallet_assets::Config<I> + pallet_asset_conversion::Config,
		I: 'static,
	{
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, TryRuntimeError> {
			log::info!(target: LOG_TARGET, "XCM V4 to V5 upgrade compatibility test starting");

			log::info!("Starting XCM V4 to V5 compatibility test migration");

			// Ensure ForeignAssets storage items
			Self::ensure_foreign_assets_compatibility()?;

			// Ensure AssetConversion storage items
			Self::ensure_asset_conversion_compatibility()?;

			log::info!("XCM V4 to V5 compatibility test migration completed successfully");

			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_state: Vec<u8>) -> Result<(), TryRuntimeError> {
			// Test a few sample conversions to ensure compatibility
			Self::ensure_sample_location_conversions()?;

			log::info!(target: LOG_TARGET, "XCM V4 to V5 compatibility test migration validated successfully");
			Ok(())
		}
	}

	impl<T, I> VerifyXcmV4ToV5Compatibility<T, I>
	where
		T: frame_system::Config + pallet_assets::Config<I> + pallet_asset_conversion::Config,
		I: 'static,
	{
		#[cfg(feature = "try-runtime")]
		fn ensure_foreign_assets_compatibility() -> Result<(), TryRuntimeError> {
			let tested_assets = 0u32;

			let v4_keys: Vec<_> = asset_as_v4::Asset::<T, I>::iter_keys().collect();
			let v5_keys: Vec<_> = asset_as_v5::Asset::<T, I>::iter_keys().collect();

			if v4_keys.len() != v5_keys.len() {
				log::error!(target: LOG_TARGET, "Asset key count mismatch: V4 has {} keys, V5 has {} keys", v4_keys.len(), v5_keys.len());
				return Err(TryRuntimeError::Other("Asset key count mismatch between V4 and V5"));
			}

			for (idx, (v4_key, v5_key)) in v4_keys.iter().zip(v5_keys.iter()).enumerate() {
				if v4_key.encode() != v5_key.encode() {
					log::error!(target: LOG_TARGET, "Asset key mismatch at index {}: V4 = {:?}, V5 = {:?}", idx, v4_key, v5_key);
					return Err(TryRuntimeError::Other("Asset key mismatch between V4 and V5"));
				}
			}

			log::info!(target: LOG_TARGET, "Tested {} ForeignAssets for XCM compatibility", tested_assets);
			Ok(())
		}

		#[cfg(feature = "try-runtime")]
		fn ensure_asset_conversion_compatibility() -> Result<(), TryRuntimeError> {
			let tested_pools = 0u32;

			let v4_pool_keys: Vec<_> = asset_as_v4::Pools::<T>::iter_keys().collect();
			let v5_pool_keys: Vec<_> = asset_as_v5::Pools::<T>::iter_keys().collect();

			if v4_pool_keys.len() != v5_pool_keys.len() {
				log::error!(target: LOG_TARGET, "Pool key count mismatch: V4 has {} keys, V5 has {} keys", v4_pool_keys.len(), v5_pool_keys.len());
				return Err(TryRuntimeError::Other("Pool key count mismatch between V4 and V5"));
			}

			for (idx, (v4_pool_key, v5_pool_key)) in
				v4_pool_keys.iter().zip(v5_pool_keys.iter()).enumerate()
			{
				if v4_pool_key != v5_pool_key {
					log::error!(target: LOG_TARGET, "Pool key mismatch at index {}: V4 = {:?}, V5 = {:?}", idx, v4_pool_key, v5_pool_key);
					return Err(TryRuntimeError::Other("Pool key mismatch between V4 and V5"));
				}
			}

			log::info!(target: LOG_TARGET, "Tested {} AssetConversion pools for XCM compatibility", tested_pools);

			Ok(())
		}

		#[cfg(feature = "try-runtime")]
		fn ensure_sample_location_conversions() -> Result<(), TryRuntimeError> {
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

				// Test V4 encoded -> V5 decoded compatibility
				let encoded_v4 = v4_location.encode();
				let decoded_v5 = xcm::v5::Location::decode(&mut &encoded_v4[..]).map_err(|_| {
					TryRuntimeError::Other("Failed to decode V4 encoded location as V5")
				})?;

				// try-from is compatible
				frame_support::ensure!(
					decoded_v5 == v5_location,
					"V4 encoded -> V5 decoded should match try_from conversion"
				);

				// encode/decode is compatible
				frame_support::ensure!(
					encoded_v4 == decoded_v5.encode(),
					"V4 encoded should match V5 re-encoded"
				);

				log::info!(target: LOG_TARGET, "Successfully tested V4 -> V5 conversion for: {:?}", v4_location);
			}

			Ok(())
		}
	}
}
