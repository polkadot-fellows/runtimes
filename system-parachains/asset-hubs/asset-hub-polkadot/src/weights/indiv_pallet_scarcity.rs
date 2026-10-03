// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! TMP weights for `indiv_pallet_scarcity`, to be replaced by a benchmark on reference hardware.
//!
//! Until this runtime is benchmarked, these are the weights of upstream's `next-asset-hub-paseo`
//! runtime at individuality-community `24033977`
//! (`runtimes/next-asset-hub-paseo/src/weights/indiv_pallet_scarcity.rs`, STEPS 50, REPEAT 50, on
//! `parity-weights` hardware).

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `indiv_pallet_scarcity`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> indiv_pallet_scarcity::WeightInfo for WeightInfo<T> {
	/// Storage: `Scarcity::NextCollectionId` (r:1 w:1)
	/// Proof: `Scarcity::NextCollectionId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:0 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	fn create_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1835`
		//  Estimated: `3766`
		// Minimum execution time: 63_407_000 picoseconds.
		Weight::from_parts(66_440_000, 0)
			.saturating_add(Weight::from_parts(0, 3766))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemMetadata` (r:100 w:100)
	/// Proof: `Scarcity::ItemMetadata` (`max_values`: None, `max_size`: Some(347), added: 2822, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:0 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// The range of component `m` is `[0, 100]`.
	fn define_item(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2031`
		//  Estimated: `3766 + m * (2822 ±0)`
		// Minimum execution time: 76_363_000 picoseconds.
		Weight::from_parts(84_430_196, 0)
			.saturating_add(Weight::from_parts(0, 3766))
			// Standard Error: 19_086
			.saturating_add(Weight::from_parts(56_319_554, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(m.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(m.into())))
			.saturating_add(Weight::from_parts(0, 2822).saturating_mul(m.into()))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NextInstanceId` (r:1 w:1)
	/// Proof: `Scarcity::NextInstanceId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadata` (r:100 w:100)
	/// Proof: `Scarcity::InstanceMetadata` (`max_values`: None, `max_size`: Some(339), added: 2814, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadataCount` (r:0 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceDeposits` (r:0 w:1)
	/// Proof: `Scarcity::InstanceDeposits` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// The range of component `m` is `[0, 100]`.
	fn mint(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2148`
		//  Estimated: `3766 + m * (2814 ±0)`
		// Minimum execution time: 98_734_000 picoseconds.
		Weight::from_parts(113_388_221, 0)
			.saturating_add(Weight::from_parts(0, 3766))
			// Standard Error: 16_892
			.saturating_add(Weight::from_parts(52_807_949, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(m.into())))
			.saturating_add(T::DbWeight::get().writes(8))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(m.into())))
			.saturating_add(Weight::from_parts(0, 2814).saturating_mul(m.into()))
	}
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:0)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	fn transfer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `302`
		//  Estimated: `3553`
		// Minimum execution time: 24_264_000 picoseconds.
		Weight::from_parts(25_896_000, 0)
			.saturating_add(Weight::from_parts(0, 3553))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Scarcity::InstanceMetadataCount` (r:1 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceDeposits` (r:1 w:1)
	/// Proof: `Scarcity::InstanceDeposits` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadata` (r:101 w:100)
	/// Proof: `Scarcity::InstanceMetadata` (`max_values`: None, `max_size`: Some(339), added: 2814, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// The range of component `m` is `[0, 100]`.
	fn burn(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2281 + m * (331 ±0)`
		//  Estimated: `3804 + m * (2814 ±0)`
		// Minimum execution time: 84_854_000 picoseconds.
		Weight::from_parts(96_363_711, 0)
			.saturating_add(Weight::from_parts(0, 3804))
			// Standard Error: 6_077
			.saturating_add(Weight::from_parts(5_869_325, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(m.into())))
			.saturating_add(T::DbWeight::get().writes(6))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(m.into())))
			.saturating_add(Weight::from_parts(0, 2814).saturating_mul(m.into()))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	fn nominate_collection_owner() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `200`
		//  Estimated: `3602`
		// Minimum execution time: 16_952_000 picoseconds.
		Weight::from_parts(18_089_000, 0)
			.saturating_add(Weight::from_parts(0, 3602))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::CollectionMetadata` (r:1 w:1)
	/// Proof: `Scarcity::CollectionMetadata` (`max_values`: None, `max_size`: Some(335), added: 2810, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	fn set_collection_metadata() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2031`
		//  Estimated: `3800`
		// Minimum execution time: 78_609_000 picoseconds.
		Weight::from_parts(81_782_000, 0)
			.saturating_add(Weight::from_parts(0, 3800))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemMetadata` (r:1 w:1)
	/// Proof: `Scarcity::ItemMetadata` (`max_values`: None, `max_size`: Some(347), added: 2822, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	fn set_item_metadata() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2106`
		//  Estimated: `3812`
		// Minimum execution time: 88_651_000 picoseconds.
		Weight::from_parts(92_061_000, 0)
			.saturating_add(Weight::from_parts(0, 3812))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Scarcity::Instances` (r:1 w:0)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:0)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadata` (r:1 w:1)
	/// Proof: `Scarcity::InstanceMetadata` (`max_values`: None, `max_size`: Some(339), added: 2814, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadataCount` (r:1 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	fn set_instance_metadata() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2350`
		//  Estimated: `3804`
		// Minimum execution time: 95_443_000 picoseconds.
		Weight::from_parts(98_777_000, 0)
			.saturating_add(Weight::from_parts(0, 3804))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Scarcity::Instances` (r:1 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadataCount` (r:1 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceDeposits` (r:1 w:1)
	/// Proof: `Scarcity::InstanceDeposits` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadata` (r:101 w:100)
	/// Proof: `Scarcity::InstanceMetadata` (`max_values`: None, `max_size`: Some(339), added: 2814, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Locked` (r:0 w:1)
	/// Proof: `Scarcity::Locked` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	/// The range of component `m` is `[0, 100]`.
	fn force_burn(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2452 + m * (331 ±0)`
		//  Estimated: `3804 + m * (2814 ±0)`
		// Minimum execution time: 102_283_000 picoseconds.
		Weight::from_parts(113_003_325, 0)
			.saturating_add(Weight::from_parts(0, 3804))
			// Standard Error: 6_021
			.saturating_add(Weight::from_parts(5_887_390, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(m.into())))
			.saturating_add(T::DbWeight::get().writes(8))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(m.into())))
			.saturating_add(Weight::from_parts(0, 2814).saturating_mul(m.into()))
	}
	/// Storage: `Scarcity::Instances` (r:1 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:2 w:2)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:0)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Locked` (r:0 w:1)
	/// Proof: `Scarcity::Locked` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	fn force_transfer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `586`
		//  Estimated: `6116`
		// Minimum execution time: 40_145_000 picoseconds.
		Weight::from_parts(43_086_000, 0)
			.saturating_add(Weight::from_parts(0, 6116))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Scarcity::NftsByOwner` (r:2 w:2)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:0)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Locked` (r:0 w:1)
	/// Proof: `Scarcity::Locked` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	fn transfer_by_holder() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `414`
		//  Estimated: `6116`
		// Minimum execution time: 28_010_000 picoseconds.
		Weight::from_parts(29_517_000, 0)
			.saturating_add(Weight::from_parts(0, 6116))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemMetadata` (r:1 w:0)
	/// Proof: `Scarcity::ItemMetadata` (`max_values`: None, `max_size`: Some(347), added: 2822, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	fn delete_item() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2106`
		//  Estimated: `3812`
		// Minimum execution time: 75_840_000 picoseconds.
		Weight::from_parts(79_184_000, 0)
			.saturating_add(Weight::from_parts(0, 3812))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:0)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::CollectionMetadata` (r:1 w:0)
	/// Proof: `Scarcity::CollectionMetadata` (`max_values`: None, `max_size`: Some(335), added: 2810, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CollectionMinters` (r:0 w:1)
	/// Proof: `NftClaims::CollectionMinters` (`max_values`: None, `max_size`: Some(65), added: 2540, mode: `MaxEncodedLen`)
	fn delete_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2073`
		//  Estimated: `3800`
		// Minimum execution time: 74_027_000 picoseconds.
		Weight::from_parts(77_652_000, 0)
			.saturating_add(Weight::from_parts(0, 3800))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:2 w:2)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(301), added: 2776, mode: `MaxEncodedLen`)
	fn claim_collection_ownership() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4477`
		//  Estimated: `6542`
		// Minimum execution time: 118_456_000 picoseconds.
		Weight::from_parts(122_297_000, 0)
			.saturating_add(Weight::from_parts(0, 6542))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Locked` (r:1 w:1)
	/// Proof: `Scarcity::Locked` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:2 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:0)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	fn as_scarcity_pipeline() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `519`
		//  Estimated: `6116`
		// Minimum execution time: 32_204_000 picoseconds.
		Weight::from_parts(33_790_000, 0)
			.saturating_add(Weight::from_parts(0, 6116))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(2))
	}
}
