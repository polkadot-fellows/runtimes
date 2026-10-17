// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Individuality.
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

//! TMP WEIGHTS

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `indiv_pallet_airdrop`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> indiv_pallet_airdrop::WeightInfo for WeightInfo<T> {
	/// Storage: `Airdrop::SupportedAssets` (r:1 w:0)
	/// Proof: `Airdrop::SupportedAssets` (`max_values`: None, `max_size`: Some(626), added: 3101, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:1)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	fn schedule_event() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `502`
		//  Estimated: `4312`
		// Minimum execution time: 71_303_000 picoseconds.
		Weight::from_parts(75_753_000, 0)
			.saturating_add(Weight::from_parts(0, 4312))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `Airdrop::SupportedAssets` (r:1 w:1)
	/// Proof: `Airdrop::SupportedAssets` (`max_values`: None, `max_size`: Some(626), added: 3101, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:0)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn enable_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `444`
		//  Estimated: `7404`
		// Minimum execution time: 71_724_000 picoseconds.
		Weight::from_parts(76_924_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Airdrop::SupportedAssets` (r:1 w:1)
	/// Proof: `Airdrop::SupportedAssets` (`max_values`: None, `max_size`: Some(626), added: 3101, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	fn disable_asset() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `605`
		//  Estimated: `7404`
		// Minimum execution time: 89_463_000 picoseconds.
		Weight::from_parts(96_279_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:1)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	fn remove_scheduled_event() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `983`
		//  Estimated: `7404`
		// Minimum execution time: 105_045_000 picoseconds.
		Weight::from_parts(113_134_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:1 w:1)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	fn participate_with_alias() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1641`
		//  Estimated: `4657`
		// Minimum execution time: 16_429_622_000 picoseconds.
		Weight::from_parts(16_487_761_000, 0)
			.saturating_add(Weight::from_parts(0, 4657))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:1 w:1)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	fn participate_with_account_via_schnorrkel_vrf() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `234`
		//  Estimated: `4265`
		// Minimum execution time: 644_994_000 picoseconds.
		Weight::from_parts(657_575_000, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::SupportedAssets` (r:1 w:0)
	/// Proof: `Airdrop::SupportedAssets` (`max_values`: None, `max_size`: Some(626), added: 3101, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Winners` (r:1 w:1)
	/// Proof: `Airdrop::Winners` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn claim() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1149`
		//  Estimated: `7404`
		// Minimum execution time: 116_692_000 picoseconds.
		Weight::from_parts(124_128_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:2)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	fn start_registration() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `230`
		//  Estimated: `4265`
		// Minimum execution time: 16_823_000 picoseconds.
		Weight::from_parts(18_377_000, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::ValidationData` (r:1 w:0)
	/// Proof: `ParachainSystem::ValidationData` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::LastRelayChainBlockNumber` (r:1 w:0)
	/// Proof: `ParachainSystem::LastRelayChainBlockNumber` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn close_registration() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1020`
		//  Estimated: `7404`
		// Minimum execution time: 106_534_000 picoseconds.
		Weight::from_parts(114_797_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `RelayRandomness::Randomness` (r:1 w:0)
	/// Proof: `RelayRandomness::Randomness` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::EventEntropy` (r:0 w:1)
	/// Proof: `Airdrop::EventEntropy` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	fn capture_entropy() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `415`
		//  Estimated: `4265`
		// Minimum execution time: 21_948_000 picoseconds.
		Weight::from_parts(23_663_000, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Airdrop::Events` (r:1 w:0)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `RelayRandomness::Randomness` (r:1 w:0)
	/// Proof: `RelayRandomness::Randomness` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	fn authorize_capture_entropy() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `415`
		//  Estimated: `4265`
		// Minimum execution time: 12_506_000 picoseconds.
		Weight::from_parts(13_387_000, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			.saturating_add(T::DbWeight::get().reads(2))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:102 w:0)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Winners` (r:0 w:100)
	/// Proof: `Airdrop::Winners` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 100]`.
	fn draw_winners(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `398 + n * (42 ±0)`
		//  Estimated: `5947 + n * (2583 ±1)`
		// Minimum execution time: 24_681_000 picoseconds.
		Weight::from_parts(23_119_134, 0)
			.saturating_add(Weight::from_parts(0, 5947))
			// Standard Error: 12_818
			.saturating_add(Weight::from_parts(6_579_424, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2583).saturating_mul(n.into()))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:2)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	fn close_drawing() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `274`
		//  Estimated: `4265`
		// Minimum execution time: 16_876_000 picoseconds.
		Weight::from_parts(18_261_000, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	fn close_claiming() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `242`
		//  Estimated: `4265`
		// Minimum execution time: 13_819_000 picoseconds.
		Weight::from_parts(15_123_000, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:101 w:100)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 100]`.
	fn clean_up_registrations(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `375 + n * (38 ±0)`
		//  Estimated: `4265 + n * (2580 ±0)`
		// Minimum execution time: 15_350_000 picoseconds.
		Weight::from_parts(13_903_928, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			// Standard Error: 1_654
			.saturating_add(Weight::from_parts(789_967, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2580).saturating_mul(n.into()))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Winners` (r:101 w:100)
	/// Proof: `Airdrop::Winners` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 100]`.
	fn clean_up_winners(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `401 + n * (87 ±0)`
		//  Estimated: `4265 + n * (2596 ±0)`
		// Minimum execution time: 13_325_000 picoseconds.
		Weight::from_parts(12_222_461, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			// Standard Error: 1_727
			.saturating_add(Weight::from_parts(1_036_127, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2596).saturating_mul(n.into()))
	}
	/// Storage: `Airdrop::Events` (r:1 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:1)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::EventEntropy` (r:0 w:1)
	/// Proof: `Airdrop::EventEntropy` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	fn finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `991`
		//  Estimated: `7404`
		// Minimum execution time: 106_825_000 picoseconds.
		Weight::from_parts(114_564_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Airdrop::Events` (r:0 w:1)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	fn transition_clean_up_phase() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_875_000 picoseconds.
		Weight::from_parts(5_470_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Airdrop::Events` (r:1 w:0)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	fn authorize_lifecycle_call() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `274`
		//  Estimated: `4265`
		// Minimum execution time: 6_735_000 picoseconds.
		Weight::from_parts(7_392_000, 0)
			.saturating_add(Weight::from_parts(0, 4265))
			.saturating_add(T::DbWeight::get().reads(1))
	}
}
