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

//! TMP weights for `indiv_pallet_coinage`, to be replaced by a re-benchmark on reference hardware.
//!
//! The pallet's `WeightInfo` changed in individuality-community `24033977`. Until this runtime is
//! re-benchmarked, these are the weights of upstream's `next-people-paseo` runtime at that commit
//! (`runtimes/next-people-paseo/src/weights/indiv_pallet_coinage.rs`, STEPS 50, REPEAT 20, on
//! `parity-weights` hardware).

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `indiv_pallet_coinage`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> indiv_pallet_coinage::WeightInfo for WeightInfo<T> {
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 32]`.
	fn split(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 12_438_000 picoseconds.
		Weight::from_parts(11_438_707, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 2_766
			.saturating_add(Weight::from_parts(1_725_522, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
	}
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn transfer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 11_141_000 picoseconds.
		Weight::from_parts(11_880_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:1 w:1)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:1 w:1)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:1 w:1)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:1 w:1)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	fn load_recycler_with_coin() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `997`
		//  Estimated: `11671`
		// Minimum execution time: 1_345_560_000 picoseconds.
		Weight::from_parts(1_366_445_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenMembers` (r:1 w:1)
	/// Proof: `Coinage::PaidUnloadTokenMembers` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidTokenCollectionsCreated` (r:1 w:0)
	/// Proof: `Coinage::PaidTokenCollectionsCreated` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:1 w:1)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:1 w:1)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:1 w:1)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	fn pay_for_recycler_unload_fee_token_with_coin() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1730`
		//  Estimated: `11671`
		// Minimum execution time: 3_415_785_000 picoseconds.
		Weight::from_parts(3_438_176_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(17))
			.saturating_add(T::DbWeight::get().writes(12))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:1 w:1)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:1 w:1)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:1 w:1)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:1 w:1)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn load_recycler_with_external_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1555`
		//  Estimated: `11671`
		// Minimum execution time: 3_289_306_000 picoseconds.
		Weight::from_parts(3_313_502_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(13))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenMembers` (r:1 w:1)
	/// Proof: `Coinage::PaidUnloadTokenMembers` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidTokenCollectionsCreated` (r:1 w:0)
	/// Proof: `Coinage::PaidTokenCollectionsCreated` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:1 w:1)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:1 w:1)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:1 w:1)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	fn pay_for_recycler_unload_fee_token_with_native() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `770`
		//  Estimated: `11671`
		// Minimum execution time: 3_268_788_000 picoseconds.
		Weight::from_parts(3_287_690_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenMembers` (r:1 w:1)
	/// Proof: `Coinage::PaidUnloadTokenMembers` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidTokenCollectionsCreated` (r:1 w:0)
	/// Proof: `Coinage::PaidTokenCollectionsCreated` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:1 w:1)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:1 w:1)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:1 w:1)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	fn pay_for_recycler_unload_fee_token_with_external_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1451`
		//  Estimated: `11671`
		// Minimum execution time: 3_370_600_000 picoseconds.
		Weight::from_parts(3_391_184_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(15))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::NextInstanceId` (r:1 w:1)
	/// Proof: `Coinage::NextInstanceId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:15 w:15)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::SuspendedCollections` (r:15 w:0)
	/// Proof: `Members::SuspendedCollections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::IdentifiersOf` (r:1 w:1)
	/// Proof: `Members::IdentifiersOf` (`max_values`: None, `max_size`: Some(3821), added: 6296, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:0 w:1)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::AssetToInstance` (r:0 w:1)
	/// Proof: `Coinage::AssetToInstance` (`max_values`: None, `max_size`: Some(630), added: 3105, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerCollectionCreated` (r:0 w:15)
	/// Proof: `Coinage::RecyclerCollectionCreated` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingsState` (r:0 w:15)
	/// Proof: `Members::RingsState` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingSize` (r:0 w:15)
	/// Proof: `Members::OnboardingSize` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn create_sufficient_instance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `622`
		//  Estimated: `47805`
		// Minimum execution time: 209_057_000 picoseconds.
		Weight::from_parts(222_111_000, 0)
			.saturating_add(Weight::from_parts(0, 47805))
			.saturating_add(T::DbWeight::get().reads(34))
			.saturating_add(T::DbWeight::get().writes(64))
	}
	/// Storage: UNKNOWN KEY `0xff10ef2d5e48a7aa2ece9a734128340a` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xff10ef2d5e48a7aa2ece9a734128340a` (r:1 w:0)
	/// Storage: `Assets::Asset` (r:2 w:2)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:4 w:4)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::NextInstanceId` (r:1 w:1)
	/// Proof: `Coinage::NextInstanceId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:15 w:15)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::SuspendedCollections` (r:15 w:0)
	/// Proof: `Members::SuspendedCollections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::IdentifiersOf` (r:1 w:1)
	/// Proof: `Members::IdentifiersOf` (`max_values`: None, `max_size`: Some(3821), added: 6296, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PotContributions` (r:1 w:1)
	/// Proof: `Coinage::PotContributions` (`max_values`: None, `max_size`: Some(694), added: 3169, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:0 w:1)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::AssetToInstance` (r:0 w:1)
	/// Proof: `Coinage::AssetToInstance` (`max_values`: None, `max_size`: Some(630), added: 3105, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerCollectionCreated` (r:0 w:15)
	/// Proof: `Coinage::RecyclerCollectionCreated` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingsState` (r:0 w:15)
	/// Proof: `Members::RingsState` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingSize` (r:0 w:15)
	/// Proof: `Members::OnboardingSize` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn create_sponsored_instance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1140`
		//  Estimated: `47805`
		// Minimum execution time: 386_652_000 picoseconds.
		Weight::from_parts(408_137_000, 0)
			.saturating_add(Weight::from_parts(0, 47805))
			.saturating_add(T::DbWeight::get().reads(46))
			.saturating_add(T::DbWeight::get().writes(75))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PotContributions` (r:1 w:1)
	/// Proof: `Coinage::PotContributions` (`max_values`: None, `max_size`: Some(694), added: 3169, mode: `MaxEncodedLen`)
	fn fund_pot() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `824`
		//  Estimated: `7404`
		// Minimum execution time: 97_993_000 picoseconds.
		Weight::from_parts(102_134_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Coinage::PotContributions` (r:1 w:1)
	/// Proof: `Coinage::PotContributions` (`max_values`: None, `max_size`: Some(694), added: 3169, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn withdraw_pot_funds() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1012`
		//  Estimated: `7404`
		// Minimum execution time: 87_222_000 picoseconds.
		Weight::from_parts(92_737_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Coinage::Instances` (r:1 w:1)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	fn charge_load_deposit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `614`
		//  Estimated: `5391`
		// Minimum execution time: 61_345_000 picoseconds.
		Weight::from_parts(64_222_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Coinage::Instances` (r:1 w:1)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:2 w:2)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:2 w:2)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:2)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	fn settle_load_deposits() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1685`
		//  Estimated: `7706`
		// Minimum execution time: 103_775_000 picoseconds.
		Weight::from_parts(108_862_000, 0)
			.saturating_add(Weight::from_parts(0, 7706))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	fn read_instance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `395`
		//  Estimated: `5391`
		// Minimum execution time: 8_713_000 picoseconds.
		Weight::from_parts(9_436_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Storage: `Coinage::Instances` (r:1 w:1)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:2 w:2)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:2 w:2)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:2)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	fn collapse_load_deposits() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1913`
		//  Estimated: `7706`
		// Minimum execution time: 152_867_000 picoseconds.
		Weight::from_parts(159_989_000, 0)
			.saturating_add(Weight::from_parts(0, 7706))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(11))
	}
	/// Storage: `Coinage::Instances` (r:1 w:1)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:2 w:2)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:2 w:2)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:2)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	fn make_instance_sufficient() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1880`
		//  Estimated: `7706`
		// Minimum execution time: 145_837_000 picoseconds.
		Weight::from_parts(152_386_000, 0)
			.saturating_add(Weight::from_parts(0, 7706))
			.saturating_add(T::DbWeight::get().reads(11))
			.saturating_add(T::DbWeight::get().writes(11))
	}
	/// Storage: `Coinage::Instances` (r:1 w:1)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	fn make_instance_sponsored() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `280`
		//  Estimated: `5391`
		// Minimum execution time: 16_216_000 picoseconds.
		Weight::from_parts(17_662_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Coinage::RecyclersLastRemovedRingIndex` (r:1 w:1)
	/// Proof: `Coinage::RecyclersLastRemovedRingIndex` (`max_values`: None, `max_size`: Some(25), added: 2500, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:1)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeys` (r:4 w:0)
	/// Proof: `Members::RingKeys` (`max_values`: None, `max_size`: Some(8226), added: 10701, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:768 w:0)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::CurrentRingIndex` (r:1 w:0)
	/// Proof: `Members::CurrentRingIndex` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingDeletionQueue` (r:1 w:4)
	/// Proof: `Members::RingDeletionQueue` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:1)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `MembersNotifier::SubscribedCollections` (r:1 w:0)
	/// Proof: `MembersNotifier::SubscribedCollections` (`max_values`: None, `max_size`: Some(32), added: 2507, mode: `MaxEncodedLen`)
	/// Storage: `Members::ActiveMembers` (r:1 w:1)
	/// Proof: `Members::ActiveMembers` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:0 w:767)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersDusting` (r:0 w:1)
	/// Proof: `Coinage::RecyclersDusting` (`max_values`: None, `max_size`: Some(17), added: 2492, mode: `MaxEncodedLen`)
	/// Storage: `Members::StaleRings` (r:0 w:1)
	/// Proof: `Members::StaleRings` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// Storage: `Members::OldRoots` (r:0 w:1)
	/// Proof: `Members::OldRoots` (`max_values`: None, `max_size`: Some(336), added: 2811, mode: `MaxEncodedLen`)
	/// Storage: `Members::PendingSuspensions` (r:0 w:1)
	/// Proof: `Members::PendingSuspensions` (`max_values`: None, `max_size`: Some(1066), added: 3541, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersArchives` (r:0 w:1)
	/// Proof: `Coinage::RecyclersArchives` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 767]`.
	/// The range of component `m` is `[0, 767]`.
	fn clean_recycler(n: u32, m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `27319 + m * (46 ±0)`
		//  Estimated: `43794 + m * (2559 ±0)`
		// Minimum execution time: 1_540_560_000 picoseconds.
		Weight::from_parts(589_103_117, 0)
			.saturating_add(Weight::from_parts(0, 43794))
			// Standard Error: 84_790
			.saturating_add(Weight::from_parts(1_801_211, 0).saturating_mul(n.into()))
			// Standard Error: 84_675
			.saturating_add(Weight::from_parts(5_630_917, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(16))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(m.into())))
			.saturating_add(T::DbWeight::get().writes(780))
			.saturating_add(Weight::from_parts(0, 2559).saturating_mul(m.into()))
	}
	/// Storage: `Coinage::ConsumedFreeUnloadTokens` (r:1000 w:1000)
	/// Proof: `Coinage::ConsumedFreeUnloadTokens` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 1000]`.
	fn clean_consumed_free_token(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `349 + n * (45 ±0)`
		//  Estimated: `990 + n * (2527 ±0)`
		// Minimum execution time: 15_490_000 picoseconds.
		Weight::from_parts(16_211_000, 0)
			.saturating_add(Weight::from_parts(0, 990))
			// Standard Error: 2_143
			.saturating_add(Weight::from_parts(1_432_622, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2527).saturating_mul(n.into()))
	}
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeys` (r:4 w:0)
	/// Proof: `Members::RingKeys` (`max_values`: None, `max_size`: Some(8226), added: 10701, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenNextRingToClean` (r:0 w:1)
	/// Proof: `Coinage::PaidUnloadTokenNextRingToClean` (`max_values`: None, `max_size`: Some(8), added: 2483, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenMembers` (r:0 w:767)
	/// Proof: `Coinage::PaidUnloadTokenMembers` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 767]`.
	fn clean_paid_unload_token_ring(_n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `25212`
		//  Estimated: `43794`
		// Minimum execution time: 1_322_563_000 picoseconds.
		Weight::from_parts(1_402_182_713, 0)
			.saturating_add(Weight::from_parts(0, 43794))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(768))
	}
	/// Storage: `Members::Collections` (r:1 w:1)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::SuspendedCollections` (r:1 w:1)
	/// Proof: `Members::SuspendedCollections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenConsumed` (r:2 w:0)
	/// Proof: `Coinage::PaidUnloadTokenConsumed` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenNextRingToClean` (r:0 w:1)
	/// Proof: `Coinage::PaidUnloadTokenNextRingToClean` (`max_values`: None, `max_size`: Some(8), added: 2483, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenDusting` (r:0 w:1)
	/// Proof: `Coinage::PaidUnloadTokenDusting` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidTokenCollectionsCreated` (r:0 w:1)
	/// Proof: `Coinage::PaidTokenCollectionsCreated` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	fn delete_expired_paid_unload_token_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `830`
		//  Estimated: `6052`
		// Minimum execution time: 75_585_000 picoseconds.
		Weight::from_parts(79_868_000, 0)
			.saturating_add(Weight::from_parts(0, 6052))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Coinage::RecyclersDusting` (r:1 w:1)
	/// Proof: `Coinage::RecyclersDusting` (`max_values`: None, `max_size`: Some(17), added: 2492, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:1000 w:1000)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 1000]`.
	fn clean_recycler_dust(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `388 + n * (46 ±0)`
		//  Estimated: `3482 + n * (2559 ±0)`
		// Minimum execution time: 23_693_000 picoseconds.
		Weight::from_parts(24_785_000, 0)
			.saturating_add(Weight::from_parts(0, 3482))
			// Standard Error: 2_308
			.saturating_add(Weight::from_parts(1_468_054, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2559).saturating_mul(n.into()))
	}
	/// Storage: `Coinage::PaidUnloadTokenDusting` (r:1 w:1)
	/// Proof: `Coinage::PaidUnloadTokenDusting` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenConsumed` (r:1000 w:1000)
	/// Proof: `Coinage::PaidUnloadTokenConsumed` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 1000]`.
	fn clean_paid_unload_token_dust(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `386 + n * (45 ±0)`
		//  Estimated: `3469 + n * (2531 ±0)`
		// Minimum execution time: 20_413_000 picoseconds.
		Weight::from_parts(21_645_000, 0)
			.saturating_add(Weight::from_parts(0, 3469))
			// Standard Error: 2_303
			.saturating_add(Weight::from_parts(1_428_841, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2531).saturating_mul(n.into()))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn unload_recycler_into_coin_1() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `6108`
		// Minimum execution time: 31_520_012_000 picoseconds.
		Weight::from_parts(31_567_053_000, 0)
			.saturating_add(Weight::from_parts(0, 6108))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:3 w:2)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn unload_recycler_into_coin_2() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `8667`
		// Minimum execution time: 44_050_973_000 picoseconds.
		Weight::from_parts(44_146_102_000, 0)
			.saturating_add(Weight::from_parts(0, 8667))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:5 w:4)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn unload_recycler_into_coin_4() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `13785`
		// Minimum execution time: 69_273_527_000 picoseconds.
		Weight::from_parts(69_327_254_000, 0)
			.saturating_add(Weight::from_parts(0, 13785))
			.saturating_add(T::DbWeight::get().reads(11))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:9 w:8)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn unload_recycler_into_coin_8() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `24021`
		// Minimum execution time: 116_877_259_000 picoseconds.
		Weight::from_parts(116_976_626_000, 0)
			.saturating_add(Weight::from_parts(0, 24021))
			.saturating_add(T::DbWeight::get().reads(15))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:17 w:16)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn unload_recycler_into_coin_16() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `44493`
		// Minimum execution time: 202_277_374_000 picoseconds.
		Weight::from_parts(202_619_398_000, 0)
			.saturating_add(Weight::from_parts(0, 44493))
			.saturating_add(T::DbWeight::get().reads(23))
			.saturating_add(T::DbWeight::get().writes(18))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:33 w:32)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn unload_recycler_into_coin_32() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `85437`
		// Minimum execution time: 372_498_266_000 picoseconds.
		Weight::from_parts(373_185_589_000, 0)
			.saturating_add(Weight::from_parts(0, 85437))
			.saturating_add(T::DbWeight::get().reads(39))
			.saturating_add(T::DbWeight::get().writes(34))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:65 w:64)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:0 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn unload_recycler_into_coin_max() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `167325`
		// Minimum execution time: 687_148_524_000 picoseconds.
		Weight::from_parts(688_079_335_000, 0)
			.saturating_add(Weight::from_parts(0, 167325))
			.saturating_add(T::DbWeight::get().reads(71))
			.saturating_add(T::DbWeight::get().writes(66))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_prepaid_1() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3199`
		//  Estimated: `6108`
		// Minimum execution time: 31_544_179_000 picoseconds.
		Weight::from_parts(31_610_441_000, 0)
			.saturating_add(Weight::from_parts(0, 6108))
			.saturating_add(T::DbWeight::get().reads(13))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:3 w:2)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_prepaid_2() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3199`
		//  Estimated: `8667`
		// Minimum execution time: 44_153_748_000 picoseconds.
		Weight::from_parts(44_251_079_000, 0)
			.saturating_add(Weight::from_parts(0, 8667))
			.saturating_add(T::DbWeight::get().reads(14))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:5 w:4)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_prepaid_4() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3199`
		//  Estimated: `13785`
		// Minimum execution time: 69_327_217_000 picoseconds.
		Weight::from_parts(69_397_538_000, 0)
			.saturating_add(Weight::from_parts(0, 13785))
			.saturating_add(T::DbWeight::get().reads(16))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:9 w:8)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_prepaid_8() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3199`
		//  Estimated: `24021`
		// Minimum execution time: 117_012_450_000 picoseconds.
		Weight::from_parts(117_095_450_000, 0)
			.saturating_add(Weight::from_parts(0, 24021))
			.saturating_add(T::DbWeight::get().reads(20))
			.saturating_add(T::DbWeight::get().writes(14))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:17 w:16)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_prepaid_16() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3199`
		//  Estimated: `44493`
		// Minimum execution time: 202_330_215_000 picoseconds.
		Weight::from_parts(202_522_636_000, 0)
			.saturating_add(Weight::from_parts(0, 44493))
			.saturating_add(T::DbWeight::get().reads(28))
			.saturating_add(T::DbWeight::get().writes(22))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:33 w:32)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_prepaid_32() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3199`
		//  Estimated: `85437`
		// Minimum execution time: 372_508_058_000 picoseconds.
		Weight::from_parts(372_775_999_000, 0)
			.saturating_add(Weight::from_parts(0, 85437))
			.saturating_add(T::DbWeight::get().reads(44))
			.saturating_add(T::DbWeight::get().writes(38))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:65 w:64)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_prepaid_max() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3199`
		//  Estimated: `167325`
		// Minimum execution time: 687_007_666_000 picoseconds.
		Weight::from_parts(688_211_707_000, 0)
			.saturating_add(Weight::from_parts(0, 167325))
			.saturating_add(T::DbWeight::get().reads(76))
			.saturating_add(T::DbWeight::get().writes(70))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:1 w:0)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_from_output_1() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1441`
		//  Estimated: `10611`
		// Minimum execution time: 277_205_000 picoseconds.
		Weight::from_parts(285_377_000, 0)
			.saturating_add(Weight::from_parts(0, 10611))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_from_output_2() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `10611`
		// Minimum execution time: 31_597_282_000 picoseconds.
		Weight::from_parts(31_632_306_000, 0)
			.saturating_add(Weight::from_parts(0, 10611))
			.saturating_add(T::DbWeight::get().reads(18))
			.saturating_add(T::DbWeight::get().writes(11))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:4 w:3)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_from_output_4() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `11226`
		// Minimum execution time: 58_572_454_000 picoseconds.
		Weight::from_parts(58_643_015_000, 0)
			.saturating_add(Weight::from_parts(0, 11226))
			.saturating_add(T::DbWeight::get().reads(20))
			.saturating_add(T::DbWeight::get().writes(13))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:8 w:7)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_from_output_8() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `21462`
		// Minimum execution time: 106_723_732_000 picoseconds.
		Weight::from_parts(106_805_172_000, 0)
			.saturating_add(Weight::from_parts(0, 21462))
			.saturating_add(T::DbWeight::get().reads(24))
			.saturating_add(T::DbWeight::get().writes(17))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:16 w:15)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_from_output_16() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `41934`
		// Minimum execution time: 192_228_670_000 picoseconds.
		Weight::from_parts(192_371_580_000, 0)
			.saturating_add(Weight::from_parts(0, 41934))
			.saturating_add(T::DbWeight::get().reads(32))
			.saturating_add(T::DbWeight::get().writes(25))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:32 w:31)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_from_output_32() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `82878`
		// Minimum execution time: 363_321_902_000 picoseconds.
		Weight::from_parts(363_533_086_000, 0)
			.saturating_add(Weight::from_parts(0, 82878))
			.saturating_add(T::DbWeight::get().reads(48))
			.saturating_add(T::DbWeight::get().writes(41))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:64 w:63)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_from_output_max() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `164766`
		// Minimum execution time: 677_836_590_000 picoseconds.
		Weight::from_parts(678_076_956_000, 0)
			.saturating_add(Weight::from_parts(0, 164766))
			.saturating_add(T::DbWeight::get().reads(80))
			.saturating_add(T::DbWeight::get().writes(73))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_prepaid_1(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4344 + d * (55 ±0)`
		//  Estimated: `18926 + d * (2568 ±16)`
		// Minimum execution time: 33_549_523_000 picoseconds.
		Weight::from_parts(31_778_899_644, 0)
			.saturating_add(Weight::from_parts(0, 18926))
			// Standard Error: 297_382
			.saturating_add(Weight::from_parts(1_933_597_379, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(18))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(10))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:3 w:2)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_prepaid_2(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4383 + d * (53 ±0)`
		//  Estimated: `18926 + d * (2568 ±55)`
		// Minimum execution time: 46_041_645_000 picoseconds.
		Weight::from_parts(44_237_938_892, 0)
			.saturating_add(Weight::from_parts(0, 18926))
			// Standard Error: 329_134
			.saturating_add(Weight::from_parts(1_941_332_551, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(11))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:5 w:4)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_prepaid_4(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4435 + d * (51 ±0)`
		//  Estimated: `18926 + d * (2568 ±13)`
		// Minimum execution time: 71_251_749_000 picoseconds.
		Weight::from_parts(69_530_435_204, 0)
			.saturating_add(Weight::from_parts(0, 18926))
			// Standard Error: 383_914
			.saturating_add(Weight::from_parts(1_934_571_793, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(21))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(13))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:9 w:8)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_prepaid_8(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4660 + d * (42 ±0)`
		//  Estimated: `24021 + d * (2568 ±22)`
		// Minimum execution time: 118_822_191_000 picoseconds.
		Weight::from_parts(117_227_425_134, 0)
			.saturating_add(Weight::from_parts(0, 24021))
			// Standard Error: 984_068
			.saturating_add(Weight::from_parts(1_938_061_988, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(25))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(18))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:17 w:16)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_prepaid_16(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4487 + d * (72 ±0)`
		//  Estimated: `44493 + d * (2568 ±0)`
		// Minimum execution time: 204_214_743_000 picoseconds.
		Weight::from_parts(202_726_293_587, 0)
			.saturating_add(Weight::from_parts(0, 44493))
			// Standard Error: 1_067_665
			.saturating_add(Weight::from_parts(1_937_697_311, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(35))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(27))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:33 w:32)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_prepaid_32(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4001 + d * (111 ±0)`
		//  Estimated: `85437 + d * (2568 ±0)`
		// Minimum execution time: 374_601_537_000 picoseconds.
		Weight::from_parts(373_363_920_758, 0)
			.saturating_add(Weight::from_parts(0, 85437))
			// Standard Error: 1_785_215
			.saturating_add(Weight::from_parts(1_941_148_202, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(54))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(45))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:65 w:64)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_prepaid_max(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4355 + d * (58 ±0)`
		//  Estimated: `167325 + d * (2568 ±0)`
		// Minimum execution time: 688_857_839_000 picoseconds.
		Weight::from_parts(688_231_774_345, 0)
			.saturating_add(Weight::from_parts(0, 167325))
			// Standard Error: 3_223_205
			.saturating_add(Weight::from_parts(1_948_139_503, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(88))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(78))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:1 w:0)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_from_output_1(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3425 + d * (55 ±0)`
		//  Estimated: `20795 + d * (2568 ±19)`
		// Minimum execution time: 2_229_376_000 picoseconds.
		Weight::from_parts(399_240_214, 0)
			.saturating_add(Weight::from_parts(0, 20795))
			// Standard Error: 248_442
			.saturating_add(Weight::from_parts(1_937_873_849, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(13))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_from_output_2(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4732 + d * (54 ±0)`
		//  Estimated: `20795 + d * (2568 ±81)`
		// Minimum execution time: 33_595_429_000 picoseconds.
		Weight::from_parts(31_802_530_484, 0)
			.saturating_add(Weight::from_parts(0, 20795))
			// Standard Error: 353_018
			.saturating_add(Weight::from_parts(1_938_803_757, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(23))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(15))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:4 w:3)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_from_output_4(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4775 + d * (52 ±0)`
		//  Estimated: `20795 + d * (2568 ±19)`
		// Minimum execution time: 60_489_771_000 picoseconds.
		Weight::from_parts(58_660_466_781, 0)
			.saturating_add(Weight::from_parts(0, 20795))
			// Standard Error: 386_438
			.saturating_add(Weight::from_parts(1_939_366_369, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(25))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(17))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:8 w:7)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_from_output_8(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4973 + d * (44 ±0)`
		//  Estimated: `21462 + d * (2568 ±20)`
		// Minimum execution time: 108_731_786_000 picoseconds.
		Weight::from_parts(107_094_782_874, 0)
			.saturating_add(Weight::from_parts(0, 21462))
			// Standard Error: 742_297
			.saturating_add(Weight::from_parts(1_930_433_208, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(29))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(21))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:16 w:15)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:3 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_from_output_16(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4924 + d * (64 ±0)`
		//  Estimated: `41934 + d * (2568 ±85)`
		// Minimum execution time: 193_982_959_000 picoseconds.
		Weight::from_parts(192_339_276_541, 0)
			.saturating_add(Weight::from_parts(0, 41934))
			// Standard Error: 901_338
			.saturating_add(Weight::from_parts(1_943_076_112, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(37))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(29))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:32 w:31)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_from_output_32(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4356 + d * (113 ±0)`
		//  Estimated: `82878 + d * (2568 ±0)`
		// Minimum execution time: 365_540_535_000 picoseconds.
		Weight::from_parts(364_090_502_326, 0)
			.saturating_add(Weight::from_parts(0, 82878))
			// Standard Error: 1_652_776
			.saturating_add(Weight::from_parts(1_949_694_761, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(56))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(47))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:32)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:64 w:63)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:32 w:32)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:2 w:2)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:2 w:2)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_external_asset_and_loaded_coins_from_output_max(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4724 + d * (58 ±0)`
		//  Estimated: `164766 + d * (2568 ±0)`
		// Minimum execution time: 680_640_795_000 picoseconds.
		Weight::from_parts(679_842_574_440, 0)
			.saturating_add(Weight::from_parts(0, 164766))
			// Standard Error: 3_435_920
			.saturating_add(Weight::from_parts(1_951_334_255, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(89))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(79))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2568).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_non_anonymous_1() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `10611`
		// Minimum execution time: 31_670_620_000 picoseconds.
		Weight::from_parts(31_709_179_000, 0)
			.saturating_add(Weight::from_parts(0, 10611))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(11))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:3 w:2)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_non_anonymous_2() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `10611`
		// Minimum execution time: 44_334_686_000 picoseconds.
		Weight::from_parts(44_394_798_000, 0)
			.saturating_add(Weight::from_parts(0, 10611))
			.saturating_add(T::DbWeight::get().reads(20))
			.saturating_add(T::DbWeight::get().writes(12))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:5 w:4)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_non_anonymous_4() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `13785`
		// Minimum execution time: 69_407_828_000 picoseconds.
		Weight::from_parts(69_505_769_000, 0)
			.saturating_add(Weight::from_parts(0, 13785))
			.saturating_add(T::DbWeight::get().reads(22))
			.saturating_add(T::DbWeight::get().writes(14))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:9 w:8)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_non_anonymous_8() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `24021`
		// Minimum execution time: 116_988_282_000 picoseconds.
		Weight::from_parts(117_154_543_000, 0)
			.saturating_add(Weight::from_parts(0, 24021))
			.saturating_add(T::DbWeight::get().reads(26))
			.saturating_add(T::DbWeight::get().writes(18))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:17 w:16)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_non_anonymous_16() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `44493`
		// Minimum execution time: 202_470_956_000 picoseconds.
		Weight::from_parts(202_718_359_000, 0)
			.saturating_add(Weight::from_parts(0, 44493))
			.saturating_add(T::DbWeight::get().reads(34))
			.saturating_add(T::DbWeight::get().writes(26))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:33 w:32)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_non_anonymous_32() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `85437`
		// Minimum execution time: 373_081_003_000 picoseconds.
		Weight::from_parts(373_879_631_000, 0)
			.saturating_add(Weight::from_parts(0, 85437))
			.saturating_add(T::DbWeight::get().reads(50))
			.saturating_add(T::DbWeight::get().writes(42))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:65 w:64)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recycler_into_external_asset_non_anonymous_max() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `167325`
		// Minimum execution time: 687_140_992_000 picoseconds.
		Weight::from_parts(687_640_841_000, 0)
			.saturating_add(Weight::from_parts(0, 167325))
			.saturating_add(T::DbWeight::get().reads(82))
			.saturating_add(T::DbWeight::get().writes(74))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_1() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3516`
		//  Estimated: `10611`
		// Minimum execution time: 31_470_557_000 picoseconds.
		Weight::from_parts(31_571_444_000, 0)
			.saturating_add(Weight::from_parts(0, 10611))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().writes(11))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:2 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:2 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:4 w:2)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:2 w:2)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_2() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4753`
		//  Estimated: `11226`
		// Minimum execution time: 62_847_754_000 picoseconds.
		Weight::from_parts(63_001_594_000, 0)
			.saturating_add(Weight::from_parts(0, 11226))
			.saturating_add(T::DbWeight::get().reads(25))
			.saturating_add(T::DbWeight::get().writes(13))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:4 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:4 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:4 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:8 w:4)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:4 w:4)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_4() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `7214`
		//  Estimated: `21462`
		// Minimum execution time: 125_779_879_000 picoseconds.
		Weight::from_parts(125_910_860_000, 0)
			.saturating_add(Weight::from_parts(0, 21462))
			.saturating_add(T::DbWeight::get().reads(37))
			.saturating_add(T::DbWeight::get().writes(17))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:8 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:8 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:8 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:16 w:8)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:8 w:8)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_8() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `12134`
		//  Estimated: `41934`
		// Minimum execution time: 250_375_268_000 picoseconds.
		Weight::from_parts(251_156_780_000, 0)
			.saturating_add(Weight::from_parts(0, 41934))
			.saturating_add(T::DbWeight::get().reads(61))
			.saturating_add(T::DbWeight::get().writes(25))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:15 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:16 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:16 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:32 w:16)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:16 w:16)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_16() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `21962`
		//  Estimated: `82878`
		// Minimum execution time: 500_938_316_000 picoseconds.
		Weight::from_parts(501_801_343_000, 0)
			.saturating_add(Weight::from_parts(0, 82878))
			.saturating_add(T::DbWeight::get().reads(108))
			.saturating_add(T::DbWeight::get().writes(41))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:15 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:32 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:32 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:64 w:32)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:32 w:32)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_32() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `41430`
		//  Estimated: `164766`
		// Minimum execution time: 1_001_769_475_000 picoseconds.
		Weight::from_parts(1_004_664_435_000, 0)
			.saturating_add(Weight::from_parts(0, 164766))
			.saturating_add(T::DbWeight::get().reads(188))
			.saturating_add(T::DbWeight::get().writes(73))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:15 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:64 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:64 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:128 w:64)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:64 w:64)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_max() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `80200`
		//  Estimated: `328542`
		// Minimum execution time: 2_006_804_874_000 picoseconds.
		Weight::from_parts(2_011_007_635_000, 0)
			.saturating_add(Weight::from_parts(0, 328542))
			.saturating_add(T::DbWeight::get().reads(348))
			.saturating_add(T::DbWeight::get().writes(137))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_recyclers_into_external_asset_non_anonymous_fee_fail() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1120`
		//  Estimated: `5391`
		// Minimum execution time: 52_409_000 picoseconds.
		Weight::from_parts(55_203_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(5))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unload_archived_recycler_into_external_asset_fee_fail() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1120`
		//  Estimated: `5391`
		// Minimum execution time: 73_323_000 picoseconds.
		Weight::from_parts(79_402_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(5))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:3 w:3)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:3 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersArchives` (r:1 w:1)
	/// Proof: `Coinage::RecyclersArchives` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	fn unload_archived_recycler_into_external_asset_fee_external_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1506`
		//  Estimated: `10611`
		// Minimum execution time: 29_785_841_000 picoseconds.
		Weight::from_parts(29_845_828_000, 0)
			.saturating_add(Weight::from_parts(0, 10611))
			.saturating_add(T::DbWeight::get().reads(13))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersArchives` (r:1 w:1)
	/// Proof: `Coinage::RecyclersArchives` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	fn unload_archived_recycler_into_external_asset_fee_native() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1360`
		//  Estimated: `8799`
		// Minimum execution time: 29_697_460_000 picoseconds.
		Weight::from_parts(29_792_137_000, 0)
			.saturating_add(Weight::from_parts(0, 8799))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:1 w:0)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_from_output_1(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1441`
		//  Estimated: `7404 + d * (2530 ±0)`
		// Minimum execution time: 236_996_000 picoseconds.
		Weight::from_parts(239_070_936, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			// Standard Error: 26_854
			.saturating_add(Weight::from_parts(4_014_342, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(11))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(8))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_from_output_2(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `7404 + d * (2530 ±0)`
		// Minimum execution time: 31_767_319_000 picoseconds.
		Weight::from_parts(31_846_335_993, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			// Standard Error: 202_705
			.saturating_add(Weight::from_parts(4_478_674, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(17))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(10))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:4 w:3)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_from_output_4(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `11226 + d * (2530 ±0)`
		// Minimum execution time: 58_511_866_000 picoseconds.
		Weight::from_parts(58_713_802_634, 0)
			.saturating_add(Weight::from_parts(0, 11226))
			// Standard Error: 324_649
			.saturating_add(Weight::from_parts(3_457_146, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(19))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(12))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:8 w:7)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_from_output_8(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `21462 + d * (2530 ±0)`
		// Minimum execution time: 106_585_594_000 picoseconds.
		Weight::from_parts(106_791_587_991, 0)
			.saturating_add(Weight::from_parts(0, 21462))
			// Standard Error: 446_273
			.saturating_add(Weight::from_parts(1_526_502, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(23))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(16))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:16 w:15)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_from_output_16(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `41934 + d * (2530 ±0)`
		// Minimum execution time: 192_031_966_000 picoseconds.
		Weight::from_parts(192_310_770_468, 0)
			.saturating_add(Weight::from_parts(0, 41934))
			// Standard Error: 732_340
			.saturating_add(Weight::from_parts(6_042_653, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(31))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(24))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:32 w:31)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_from_output_32(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `82878 + d * (2530 ±0)`
		// Minimum execution time: 363_192_749_000 picoseconds.
		Weight::from_parts(363_689_972_664, 0)
			.saturating_add(Weight::from_parts(0, 82878))
			// Standard Error: 1_901_407
			.saturating_add(Weight::from_parts(23_713_404, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(47))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(40))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:64 w:63)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::TotalValueOfDestroyedCoins` (r:1 w:1)
	/// Proof: `Coinage::TotalValueOfDestroyedCoins` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_from_output_max(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3567`
		//  Estimated: `164766 + d * (2530 ±0)`
		// Minimum execution time: 677_496_541_000 picoseconds.
		Weight::from_parts(679_293_463_077, 0)
			.saturating_add(Weight::from_parts(0, 164766))
			// Standard Error: 4_289_105
			.saturating_add(Weight::from_parts(2_715_645, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(79))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(72))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_prepaid_1(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `6108 + d * (2530 ±0)`
		// Minimum execution time: 31_308_870_000 picoseconds.
		Weight::from_parts(31_388_568_062, 0)
			.saturating_add(Weight::from_parts(0, 6108))
			// Standard Error: 157_936
			.saturating_add(Weight::from_parts(2_703_758, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:3 w:2)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_prepaid_2(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `8667 + d * (2530 ±0)`
		// Minimum execution time: 43_978_600_000 picoseconds.
		Weight::from_parts(44_025_433_908, 0)
			.saturating_add(Weight::from_parts(0, 8667))
			// Standard Error: 206_100
			.saturating_add(Weight::from_parts(7_894_145, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:5 w:4)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_prepaid_4(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `13785 + d * (2530 ±0)`
		// Minimum execution time: 69_109_656_000 picoseconds.
		Weight::from_parts(69_303_150_724, 0)
			.saturating_add(Weight::from_parts(0, 13785))
			// Standard Error: 438_484
			.saturating_add(Weight::from_parts(2_308_754, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(11))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(5))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:9 w:8)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_prepaid_8(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `24021 + d * (2530 ±0)`
		// Minimum execution time: 116_667_599_000 picoseconds.
		Weight::from_parts(116_869_209_241, 0)
			.saturating_add(Weight::from_parts(0, 24021))
			// Standard Error: 554_051
			.saturating_add(Weight::from_parts(6_730_944, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(15))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(9))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:17 w:16)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_prepaid_16(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `44493 + d * (2530 ±0)`
		// Minimum execution time: 202_203_903_000 picoseconds.
		Weight::from_parts(202_678_158_512, 0)
			.saturating_add(Weight::from_parts(0, 44493))
			.saturating_add(T::DbWeight::get().reads(23))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(17))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:33 w:32)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_prepaid_32(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `85437 + d * (2530 ±0)`
		// Minimum execution time: 372_528_783_000 picoseconds.
		Weight::from_parts(373_410_450_271, 0)
			.saturating_add(Weight::from_parts(0, 85437))
			.saturating_add(T::DbWeight::get().reads(39))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(33))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:32)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:65 w:64)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[1, 32]`.
	fn unload_recycler_into_coins_prepaid_max(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2484`
		//  Estimated: `167325 + d * (2530 ±0)`
		// Minimum execution time: 686_817_807_000 picoseconds.
		Weight::from_parts(688_479_670_805, 0)
			.saturating_add(Weight::from_parts(0, 167325))
			.saturating_add(T::DbWeight::get().reads(71))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(d.into())))
			.saturating_add(T::DbWeight::get().writes(65))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:0)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	fn as_none_tx_ext_others() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `500`
		//  Estimated: `5391`
		// Minimum execution time: 29_156_000 picoseconds.
		Weight::from_parts(30_460_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(4))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn as_none_tx_ext_unload_recycler_into_external_asset_non_anonymous() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3052`
		//  Estimated: `5391`
		// Minimum execution time: 136_668_000 picoseconds.
		Weight::from_parts(144_213_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(9))
	}
	/// Storage: `Members::Collections` (r:15 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:64 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:64 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 64]`.
	fn as_none_tx_ext_unload_recyclers_into_external_asset_non_anonymous(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2040 + n * (1214 ±0)`
		//  Estimated: `28458 + n * (3667 ±0)`
		// Minimum execution time: 105_885_000 picoseconds.
		Weight::from_parts(87_274_659, 0)
			.saturating_add(Weight::from_parts(0, 28458))
			// Standard Error: 19_614
			.saturating_add(Weight::from_parts(34_730_878, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(15))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3667).saturating_mul(n.into()))
	}
	/// Storage: `Coinage::RecyclersArchives` (r:1 w:0)
	/// Proof: `Coinage::RecyclersArchives` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn as_none_tx_ext_unload_archived_recycler_into_external_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1011`
		//  Estimated: `5391`
		// Minimum execution time: 88_761_000 picoseconds.
		Weight::from_parts(95_414_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(6))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::LockedCoins` (r:1 w:1)
	/// Proof: `Coinage::LockedCoins` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:16 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 15]`.
	fn as_coin_split(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `399`
		//  Estimated: `3523 + n * (2530 ±0)`
		// Minimum execution time: 24_169_000 picoseconds.
		Weight::from_parts(24_110_855, 0)
			.saturating_add(Weight::from_parts(0, 3523))
			// Standard Error: 6_046
			.saturating_add(Weight::from_parts(1_991_381, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(n.into()))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::LockedCoins` (r:1 w:1)
	/// Proof: `Coinage::LockedCoins` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:2 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	fn as_coin_transfer() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `399`
		//  Estimated: `6050`
		// Minimum execution time: 23_181_000 picoseconds.
		Weight::from_parts(24_484_000, 0)
			.saturating_add(Weight::from_parts(0, 6050))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::LockedCoins` (r:1 w:1)
	/// Proof: `Coinage::LockedCoins` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:1 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:1 w:0)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:0)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	fn as_coin_load_recycler_with_coin() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `651`
		//  Estimated: `5391`
		// Minimum execution time: 2_569_411_000 picoseconds.
		Weight::from_parts(2_590_365_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::LockedCoins` (r:1 w:1)
	/// Proof: `Coinage::LockedCoins` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:1 w:1)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenMembers` (r:1 w:0)
	/// Proof: `Coinage::PaidUnloadTokenMembers` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	fn as_coin_pay_for_recycler_unload_fee_token_with_coin() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1076`
		//  Estimated: `5391`
		// Minimum execution time: 2_587_666_000 picoseconds.
		Weight::from_parts(2_609_831_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:2 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::ConsumedFreeUnloadTokens` (r:1 w:1)
	/// Proof: `Coinage::ConsumedFreeUnloadTokens` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn as_unload_token_people_tx_ext() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3859`
		//  Estimated: `8324`
		// Minimum execution time: 29_043_591_000 picoseconds.
		Weight::from_parts(29_325_803_000, 0)
			.saturating_add(Weight::from_parts(0, 8324))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:2 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::ConsumedFreeUnloadTokens` (r:1 w:1)
	/// Proof: `Coinage::ConsumedFreeUnloadTokens` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn as_unload_token_lite_people_tx_ext() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3851`
		//  Estimated: `8324`
		// Minimum execution time: 29_047_337_000 picoseconds.
		Weight::from_parts(29_297_037_000, 0)
			.saturating_add(Weight::from_parts(0, 8324))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Members::Collections` (r:2 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:2 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenConsumed` (r:1 w:1)
	/// Proof: `Coinage::PaidUnloadTokenConsumed` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	fn as_unload_token_paid_tx_ext() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3666`
		//  Estimated: `8324`
		// Minimum execution time: 29_122_923_000 picoseconds.
		Weight::from_parts(29_475_562_000, 0)
			.saturating_add(Weight::from_parts(0, 8324))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclerAliasStates` (r:2 w:1)
	/// Proof: `Coinage::RecyclerAliasStates` (`max_values`: None, `max_size`: Some(84), added: 2559, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersUnloadedCount` (r:1 w:1)
	/// Proof: `Coinage::RecyclersUnloadedCount` (`max_values`: None, `max_size`: Some(21), added: 2496, mode: `MaxEncodedLen`)
	fn as_unload_token_from_output_tx_ext() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3106`
		//  Estimated: `6108`
		// Minimum execution time: 29_500_317_000 picoseconds.
		Weight::from_parts(29_621_015_000, 0)
			.saturating_add(Weight::from_parts(0, 6108))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:1 w:1)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:1 w:1)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::QueuePageIndices` (r:1 w:1)
	/// Proof: `Members::QueuePageIndices` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:1 w:1)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn load_recycler_with_external_asset_unpaid() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1555`
		//  Estimated: `11671`
		// Minimum execution time: 1_415_884_000 picoseconds.
		Weight::from_parts(1_449_349_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(13))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:1 w:0)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Storage: `System::Account` (r:2 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:0)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	fn as_infallible_unpaid_tx_ext() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1192`
		//  Estimated: `6196`
		// Minimum execution time: 2_619_571_000 picoseconds.
		Weight::from_parts(2_647_775_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:10 w:0)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:0)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:0)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Storage: `System::Account` (r:2 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:0)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 10]`.
	fn as_infallible_unpaid_tx_ext_batch(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1192`
		//  Estimated: `6196 + n * (2528 ±0)`
		// Minimum execution time: 2_620_461_000 picoseconds.
		Weight::from_parts(137_376_247, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			// Standard Error: 287_053
			.saturating_add(Weight::from_parts(2_527_740_709, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 2528).saturating_mul(n.into()))
	}
	/// Storage: `Members::Collections` (r:15 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:64 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:64 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::CoinsByOwner` (r:32 w:0)
	/// Proof: `Coinage::CoinsByOwner` (`max_values`: None, `max_size`: Some(55), added: 2530, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::RecyclersCoinToRecycler` (r:32 w:0)
	/// Proof: `Coinage::RecyclersCoinToRecycler` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0xb46f8cf2410626536982497ca62abcfe` (r:1 w:0)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:0)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(265), added: 2740, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 64]`.
	/// The range of component `d` is `[0, 32]`.
	fn validate_unload_calls(r: u32, d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76 + d * (53 ±0) + r * (1244 ±0)`
		//  Estimated: `24397 + d * (2530 ±1) + r * (3667 ±0)`
		// Minimum execution time: 2_219_908_000 picoseconds.
		Weight::from_parts(45_049_561, 0)
			.saturating_add(Weight::from_parts(0, 24397))
			// Standard Error: 74_849
			.saturating_add(Weight::from_parts(35_330_372, 0).saturating_mul(r.into()))
			// Standard Error: 145_683
			.saturating_add(Weight::from_parts(647_090_139, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(r.into())))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(d.into())))
			.saturating_add(Weight::from_parts(0, 2530).saturating_mul(d.into()))
			.saturating_add(Weight::from_parts(0, 3667).saturating_mul(r.into()))
	}
	/// Storage: `Coinage::Instances` (r:1 w:0)
	/// Proof: `Coinage::Instances` (`max_values`: None, `max_size`: Some(1926), added: 4401, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn direct_offboard_coin_into_external_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1051`
		//  Estimated: `5391`
		// Minimum execution time: 77_932_000 picoseconds.
		Weight::from_parts(81_741_000, 0)
			.saturating_add(Weight::from_parts(0, 5391))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Coinage::RecyclersLastRemovedRingIndex` (r:1 w:0)
	/// Proof: `Coinage::RecyclersLastRemovedRingIndex` (`max_values`: None, `max_size`: Some(25), added: 2500, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn authorize_clean_recycler() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1191`
		//  Estimated: `4111`
		// Minimum execution time: 44_252_000 picoseconds.
		Weight::from_parts(47_830_000, 0)
			.saturating_add(Weight::from_parts(0, 4111))
			.saturating_add(T::DbWeight::get().reads(4))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::ConsumedFreeUnloadTokens` (r:2 w:0)
	/// Proof: `Coinage::ConsumedFreeUnloadTokens` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn authorize_clean_consumed_free_token() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `400`
		//  Estimated: `6044`
		// Minimum execution time: 17_600_000 picoseconds.
		Weight::from_parts(18_956_000, 0)
			.saturating_add(Weight::from_parts(0, 6044))
			.saturating_add(T::DbWeight::get().reads(3))
	}
	/// Storage: `Coinage::PaidTokenCollectionsCreated` (r:1 w:0)
	/// Proof: `Coinage::PaidTokenCollectionsCreated` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenNextRingToClean` (r:1 w:0)
	/// Proof: `Coinage::PaidUnloadTokenNextRingToClean` (`max_values`: None, `max_size`: Some(8), added: 2483, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn authorize_clean_paid_unload_token_ring() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `896`
		//  Estimated: `4111`
		// Minimum execution time: 41_980_000 picoseconds.
		Weight::from_parts(45_063_000, 0)
			.saturating_add(Weight::from_parts(0, 4111))
			.saturating_add(T::DbWeight::get().reads(5))
	}
	/// Storage: `Coinage::RecyclersDusting` (r:1 w:0)
	/// Proof: `Coinage::RecyclersDusting` (`max_values`: None, `max_size`: Some(17), added: 2492, mode: `MaxEncodedLen`)
	fn authorize_clean_recycler_dust() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `286`
		//  Estimated: `3482`
		// Minimum execution time: 5_929_000 picoseconds.
		Weight::from_parts(6_745_000, 0)
			.saturating_add(Weight::from_parts(0, 3482))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Coinage::PaidUnloadTokenDusting` (r:1 w:0)
	/// Proof: `Coinage::PaidUnloadTokenDusting` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	fn authorize_clean_paid_unload_token_dust() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `274`
		//  Estimated: `3469`
		// Minimum execution time: 6_064_000 picoseconds.
		Weight::from_parts(6_716_000, 0)
			.saturating_add(Weight::from_parts(0, 3469))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Coinage::PaidTokenCollectionsCreated` (r:1 w:0)
	/// Proof: `Coinage::PaidTokenCollectionsCreated` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidUnloadTokenNextRingToClean` (r:1 w:0)
	/// Proof: `Coinage::PaidUnloadTokenNextRingToClean` (`max_values`: None, `max_size`: Some(8), added: 2483, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingKeysStatus` (r:1 w:0)
	/// Proof: `Members::RingKeysStatus` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn authorize_delete_expired_paid_unload_token_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `890`
		//  Estimated: `4111`
		// Minimum execution time: 43_325_000 picoseconds.
		Weight::from_parts(45_819_000, 0)
			.saturating_add(Weight::from_parts(0, 4111))
			.saturating_add(T::DbWeight::get().reads(5))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Coinage::PaidTokenCollectionsCreated` (r:1 w:1)
	/// Proof: `Coinage::PaidTokenCollectionsCreated` (`max_values`: None, `max_size`: Some(4), added: 2479, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:1)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::SuspendedCollections` (r:1 w:0)
	/// Proof: `Members::SuspendedCollections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::IdentifiersOf` (r:1 w:1)
	/// Proof: `Members::IdentifiersOf` (`max_values`: None, `max_size`: Some(3821), added: 6296, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingsState` (r:0 w:1)
	/// Proof: `Members::RingsState` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingSize` (r:0 w:1)
	/// Proof: `Members::OnboardingSize` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	fn on_poll_create_paid_token_collection() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `684`
		//  Estimated: `7286`
		// Minimum execution time: 32_684_000 picoseconds.
		Weight::from_parts(34_528_000, 0)
			.saturating_add(Weight::from_parts(0, 7286))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(5))
	}
}
