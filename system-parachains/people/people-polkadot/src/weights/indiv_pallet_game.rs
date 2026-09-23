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

/// Weight functions for `indiv_pallet_game`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> indiv_pallet_game::WeightInfo for WeightInfo<T> {
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::StoredPhaseDurations` (r:1 w:0)
	/// Proof: `Game::StoredPhaseDurations` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	/// Storage: `Game::GameIndex` (r:1 w:1)
	/// Proof: `Game::GameIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:2 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::SupportedAssets` (r:1 w:0)
	/// Proof: `Airdrop::SupportedAssets` (`max_values`: None, `max_size`: Some(626), added: 3101, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `Game::GameHistory` (r:0 w:1)
	/// Proof: `Game::GameHistory` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:16)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn new_game(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `791`
		//  Estimated: `7404 + n * (3275 ±0)`
		// Minimum execution time: 14_372_000 picoseconds.
		Weight::from_parts(44_696_565, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			// Standard Error: 140_992
			.saturating_add(Weight::from_parts(89_353_094, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(7))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Game::Game` (r:1 w:0)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	fn get_game() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `230`
		//  Estimated: `1559`
		// Minimum execution time: 5_537_000 picoseconds.
		Weight::from_parts(6_224_000, 0)
			.saturating_add(Weight::from_parts(0, 1559))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Game::GameSchedules` (r:1 w:0)
	/// Proof: `Game::GameSchedules` (`max_values`: Some(1), `max_size`: Some(121849), added: 122344, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 12]`.
	fn get_game_schedules(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `137 + n * (714 ±0)`
		//  Estimated: `123334`
		// Minimum execution time: 10_961_000 picoseconds.
		Weight::from_parts(5_619_656, 0)
			.saturating_add(Weight::from_parts(0, 123334))
			// Standard Error: 7_195
			.saturating_add(Weight::from_parts(5_965_386, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn unix_time() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `94`
		//  Estimated: `1493`
		// Minimum execution time: 4_131_000 picoseconds.
		Weight::from_parts(4_519_000, 0)
			.saturating_add(Weight::from_parts(0, 1493))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Game::Game` (r:0 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	fn put_game() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 587_000 picoseconds.
		Weight::from_parts(723_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Game::GameSchedules` (r:0 w:1)
	/// Proof: `Game::GameSchedules` (`max_values`: Some(1), `max_size`: Some(121849), added: 122344, mode: `MaxEncodedLen`)
	fn put_game_schedules() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 22_080_000 picoseconds.
		Weight::from_parts(23_711_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `System::ParentHash` (r:1 w:0)
	/// Proof: `System::ParentHash` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:0)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::ShuffleRecognized` (r:1 w:0)
	/// Proof: `Game::ShuffleRecognized` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	/// Storage: `Game::ShuffleNotRecognized` (r:1 w:0)
	/// Proof: `Game::ShuffleNotRecognized` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerToIndex` (r:1 w:0)
	/// Proof: `Game::PlayerToIndex` (`max_values`: None, `max_size`: Some(90), added: 2565, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingsState` (r:1 w:0)
	/// Proof: `Members::RingsState` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `Members::ActiveMembers` (r:1 w:0)
	/// Proof: `Members::ActiveMembers` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThresholdSchedule` (r:1 w:0)
	/// Proof: `Score::PersonhoodThresholdSchedule` (`max_values`: Some(1), `max_size`: Some(81), added: 576, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceSchedule` (r:1 w:0)
	/// Proof: `Score::AbsenceGraceSchedule` (`max_values`: Some(1), `max_size`: Some(49), added: 544, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceRatio` (r:0 w:1)
	/// Proof: `Score::AbsenceGraceRatio` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThreshold` (r:0 w:1)
	/// Proof: `Score::PersonhoodThreshold` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `Game::Game` (r:0 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	fn shuffles_base() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `391`
		//  Estimated: `4111`
		// Minimum execution time: 32_130_000 picoseconds.
		Weight::from_parts(33_877_000, 0)
			.saturating_add(Weight::from_parts(0, 4111))
			.saturating_add(T::DbWeight::get().reads(11))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Game::Players` (r:2 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:0)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Game::ShuffleNotRecognized` (r:0 w:10)
	/// Proof: `Game::ShuffleNotRecognized` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 10]`.
	fn shuffle_step_insert(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `672`
		//  Estimated: `6100`
		// Minimum execution time: 26_058_000 picoseconds.
		Weight::from_parts(25_494_264, 0)
			.saturating_add(Weight::from_parts(0, 6100))
			// Standard Error: 4_146
			.saturating_add(Weight::from_parts(2_191_442, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
	}
	/// Storage: `Game::ShuffleNotRecognized` (r:20 w:10)
	/// Proof: `Game::ShuffleNotRecognized` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerToIndex` (r:2 w:2)
	/// Proof: `Game::PlayerToIndex` (`max_values`: None, `max_size`: Some(90), added: 2565, mode: `MaxEncodedLen`)
	/// Storage: `Game::IndexToPlayer` (r:0 w:10)
	/// Proof: `Game::IndexToPlayer` (`max_values`: None, `max_size`: Some(46), added: 2521, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 10]`.
	fn shuffle_step_retrieve(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `399 + n * (90 ±0)`
		//  Estimated: `4891 + n * (5098 ±0)`
		// Minimum execution time: 21_503_000 picoseconds.
		Weight::from_parts(12_722_959, 0)
			.saturating_add(Weight::from_parts(0, 4891))
			// Standard Error: 33_100
			.saturating_add(Weight::from_parts(13_936_298, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 5098).saturating_mul(n.into()))
	}
	/// Storage: `Game::PlayerToIndex` (r:2 w:0)
	/// Proof: `Game::PlayerToIndex` (`max_values`: None, `max_size`: Some(90), added: 2565, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[2, 100]`.
	fn shuffle_step_compute_weights(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1151`
		//  Estimated: `6120`
		// Minimum execution time: 20_787_000 picoseconds.
		Weight::from_parts(25_290_181, 0)
			.saturating_add(Weight::from_parts(0, 6120))
			// Standard Error: 1_405
			.saturating_add(Weight::from_parts(51_237, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Members::RingsState` (r:1 w:0)
	/// Proof: `Members::RingsState` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `Members::ActiveMembers` (r:1 w:0)
	/// Proof: `Members::ActiveMembers` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThresholdSchedule` (r:1 w:0)
	/// Proof: `Score::PersonhoodThresholdSchedule` (`max_values`: Some(1), `max_size`: Some(81), added: 576, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceSchedule` (r:1 w:0)
	/// Proof: `Score::AbsenceGraceSchedule` (`max_values`: Some(1), `max_size`: Some(49), added: 544, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceRatio` (r:0 w:1)
	/// Proof: `Score::AbsenceGraceRatio` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThreshold` (r:0 w:1)
	/// Proof: `Score::PersonhoodThreshold` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn shuffle_step_start_session() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `424`
		//  Estimated: `4111`
		// Minimum execution time: 17_588_000 picoseconds.
		Weight::from_parts(18_646_000, 0)
			.saturating_add(Weight::from_parts(0, 4111))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::NftClaimCreditAwards` (r:1 w:0)
	/// Proof: `Game::NftClaimCreditAwards` (`max_values`: None, `max_size`: Some(78014), added: 80489, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:0)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	fn player_process_step1() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `312`
		//  Estimated: `81479`
		// Minimum execution time: 20_361_000 picoseconds.
		Weight::from_parts(21_687_000, 0)
			.saturating_add(Weight::from_parts(0, 81479))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThreshold` (r:1 w:0)
	/// Proof: `Score::PersonhoodThreshold` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceRatio` (r:1 w:0)
	/// Proof: `Score::AbsenceGraceRatio` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerAttendanceHistory` (r:1 w:1)
	/// Proof: `Game::PlayerAttendanceHistory` (`max_values`: None, `max_size`: Some(98), added: 2573, mode: `MaxEncodedLen`)
	/// Storage: `Game::GameParticipantCount` (r:1 w:1)
	/// Proof: `Game::GameParticipantCount` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerToIndex` (r:1 w:0)
	/// Proof: `Game::PlayerToIndex` (`max_values`: None, `max_size`: Some(90), added: 2565, mode: `MaxEncodedLen`)
	/// Storage: `Game::IndexToPlayer` (r:90 w:0)
	/// Proof: `Game::IndexToPlayer` (`max_values`: None, `max_size`: Some(46), added: 2521, mode: `MaxEncodedLen`)
	/// Storage: `Game::AwardedNftClaimCredits` (r:1 w:1)
	/// Proof: `Game::AwardedNftClaimCredits` (`max_values`: None, `max_size`: Some(77), added: 2552, mode: `MaxEncodedLen`)
	/// Storage: `Game::NftClaimCreditAwards` (r:1 w:1)
	/// Proof: `Game::NftClaimCreditAwards` (`max_values`: None, `max_size`: Some(78014), added: 80489, mode: `MaxEncodedLen`)
	/// Storage: `Game::NftClaimCreditBlocks` (r:1 w:1)
	/// Proof: `Game::NftClaimCreditBlocks` (`max_values`: None, `max_size`: Some(178), added: 2653, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(229), added: 2704, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:2 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::PendingNftClaimCreditRootInfo` (r:0 w:1)
	/// Proof: `Game::PendingNftClaimCreditRootInfo` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 10]`.
	fn player_process_step1_inner_loop(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1796 + r * (513 ±0)`
		//  Estimated: `81479 + r * (22689 ±0)`
		// Minimum execution time: 240_824_000 picoseconds.
		Weight::from_parts(105_356_630, 0)
			.saturating_add(Weight::from_parts(0, 81479))
			// Standard Error: 148_362
			.saturating_add(Weight::from_parts(145_148_992, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(13))
			.saturating_add(T::DbWeight::get().reads((9_u64).saturating_mul(r.into())))
			.saturating_add(T::DbWeight::get().writes(10))
			.saturating_add(Weight::from_parts(0, 22689).saturating_mul(r.into()))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	fn player_process_step2() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `165`
		//  Estimated: `1559`
		// Minimum execution time: 15_302_000 picoseconds.
		Weight::from_parts(16_406_000, 0)
			.saturating_add(Weight::from_parts(0, 1559))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Game::IndexToPlayer` (r:101 w:100)
	/// Proof: `Game::IndexToPlayer` (`max_values`: None, `max_size`: Some(46), added: 2521, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerToIndex` (r:101 w:100)
	/// Proof: `Game::PlayerToIndex` (`max_values`: None, `max_size`: Some(90), added: 2565, mode: `MaxEncodedLen`)
	/// Storage: `Game::AwardedNftClaimCredits` (r:101 w:100)
	/// Proof: `Game::AwardedNftClaimCredits` (`max_values`: None, `max_size`: Some(77), added: 2552, mode: `MaxEncodedLen`)
	fn player_process_step2_inner_loop() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `21038`
		//  Estimated: `260055`
		// Minimum execution time: 305_690_000 picoseconds.
		Weight::from_parts(314_945_000, 0)
			.saturating_add(Weight::from_parts(0, 260055))
			.saturating_add(T::DbWeight::get().reads(303))
			.saturating_add(T::DbWeight::get().writes(300))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:0)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::GameHistory` (r:0 w:1)
	/// Proof: `Game::GameHistory` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	fn process_cancelling() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `172`
		//  Estimated: `3545`
		// Minimum execution time: 8_646_000 picoseconds.
		Weight::from_parts(9_393_000, 0)
			.saturating_add(Weight::from_parts(0, 3545))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Game::ShuffleRecognized` (r:101 w:100)
	/// Proof: `Game::ShuffleRecognized` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	/// Storage: `Game::ShuffleNotRecognized` (r:101 w:100)
	/// Proof: `Game::ShuffleNotRecognized` (`max_values`: None, `max_size`: Some(74), added: 2549, mode: `MaxEncodedLen`)
	fn process_cancelling_step_shuffle() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `14190`
		//  Estimated: `258439`
		// Minimum execution time: 194_533_000 picoseconds.
		Weight::from_parts(201_628_000, 0)
			.saturating_add(Weight::from_parts(0, 258439))
			.saturating_add(T::DbWeight::get().reads(202))
			.saturating_add(T::DbWeight::get().writes(200))
	}
	/// Storage: `Game::Players` (r:2 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerToIndex` (r:1 w:1)
	/// Proof: `Game::PlayerToIndex` (`max_values`: None, `max_size`: Some(90), added: 2565, mode: `MaxEncodedLen`)
	/// Storage: `Game::IndexToPlayer` (r:0 w:10)
	/// Proof: `Game::IndexToPlayer` (`max_values`: None, `max_size`: Some(46), added: 2521, mode: `MaxEncodedLen`)
	/// Storage: `Game::AwardedNftClaimCredits` (r:0 w:1)
	/// Proof: `Game::AwardedNftClaimCredits` (`max_values`: None, `max_size`: Some(77), added: 2552, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 10]`.
	fn process_cancelling_step_player(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `418 + n * (4 ±0)`
		//  Estimated: `6100`
		// Minimum execution time: 22_178_000 picoseconds.
		Weight::from_parts(24_387_805, 0)
			.saturating_add(Weight::from_parts(0, 6100))
			// Standard Error: 30_822
			.saturating_add(Weight::from_parts(1_017_067, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::StmtAccountToAlias` (r:1 w:0)
	/// Proof: `Game::StmtAccountToAlias` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::ArchivedPlayers` (r:1 w:0)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:16 w:16)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653adef12e42f3e487e9b14095` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653adef12e42f3e487e9b14095` (r:1 w:1)
	/// Storage: `Game::CommunicationIdentifiers` (r:0 w:1)
	/// Proof: `Game::CommunicationIdentifiers` (`max_values`: None, `max_size`: Some(113), added: 2588, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn sign_up_with_invite(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `558 + n * (178 ±0)`
		//  Estimated: `4008 + n * (3275 ±0)`
		// Minimum execution time: 47_426_000 picoseconds.
		Weight::from_parts(63_153_931, 0)
			.saturating_add(Weight::from_parts(0, 4008))
			// Standard Error: 81_591
			.saturating_add(Weight::from_parts(653_141_237, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(6))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::StmtAccountToAlias` (r:1 w:0)
	/// Proof: `Game::StmtAccountToAlias` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::ArchivedPlayers` (r:1 w:0)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayDepositAmount` (r:1 w:0)
	/// Proof: `Game::PlayDepositAmount` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(229), added: 2704, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:16 w:16)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653adef12e42f3e487e9b14095` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653adef12e42f3e487e9b14095` (r:1 w:1)
	/// Storage: `Game::CommunicationIdentifiers` (r:0 w:1)
	/// Proof: `Game::CommunicationIdentifiers` (`max_values`: None, `max_size`: Some(113), added: 2588, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn sign_up_with_account_new(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `661 + n * (178 ±0)`
		//  Estimated: `4111 + n * (3275 ±0)`
		// Minimum execution time: 84_503_000 picoseconds.
		Weight::from_parts(99_267_151, 0)
			.saturating_add(Weight::from_parts(0, 4111))
			// Standard Error: 81_648
			.saturating_add(Weight::from_parts(654_591_840, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(7))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::StmtAccountToAlias` (r:1 w:0)
	/// Proof: `Game::StmtAccountToAlias` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::ArchivedPlayers` (r:1 w:0)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:0)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:16 w:16)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// Storage: `Game::CommunicationIdentifiers` (r:0 w:1)
	/// Proof: `Game::CommunicationIdentifiers` (`max_values`: None, `max_size`: Some(113), added: 2588, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn sign_up_with_account_recognized(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2092 + n * (178 ±0)`
		//  Estimated: `4657 + n * (3275 ±0)`
		// Minimum execution time: 41_160_000 picoseconds.
		Weight::from_parts(153_048_373, 0)
			.saturating_add(Weight::from_parts(0, 4657))
			// Standard Error: 1_497_868
			.saturating_add(Weight::from_parts(16_183_056_822, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::AliasToStmtAccount` (r:1 w:1)
	/// Proof: `Game::AliasToStmtAccount` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653a6b79c57e6a095239282c04` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653a6b79c57e6a095239282c04` (r:1 w:1)
	/// Storage: `Game::StmtAccountToAlias` (r:1 w:2)
	/// Proof: `Game::StmtAccountToAlias` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:2 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::ArchivedPlayers` (r:2 w:0)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653acecc1507dc1ddd7295951c` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653acecc1507dc1ddd7295951c` (r:1 w:1)
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThreshold` (r:1 w:0)
	/// Proof: `Score::PersonhoodThreshold` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::Root` (r:1 w:0)
	/// Proof: `Members::Root` (`max_values`: None, `max_size`: Some(1192), added: 3667, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:16 w:16)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// Storage: `Game::CommunicationIdentifiers` (r:0 w:1)
	/// Proof: `Game::CommunicationIdentifiers` (`max_values`: None, `max_size`: Some(113), added: 2588, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn sign_up_with_alias(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2097 + n * (178 ±0)`
		//  Estimated: `6100 + n * (3275 ±8)`
		// Minimum execution time: 111_524_000 picoseconds.
		Weight::from_parts(342_139_772, 0)
			.saturating_add(Weight::from_parts(0, 6100))
			// Standard Error: 1_416_661
			.saturating_add(Weight::from_parts(16_207_107_100, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(14))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(9))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Game::LiteInvites` (r:1 w:1)
	/// Proof: `Game::LiteInvites` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::StmtAccountToAlias` (r:1 w:0)
	/// Proof: `Game::StmtAccountToAlias` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::ArchivedPlayers` (r:1 w:0)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:16 w:16)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653adef12e42f3e487e9b14095` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653adef12e42f3e487e9b14095` (r:1 w:1)
	/// Storage: `Game::CommunicationIdentifiers` (r:0 w:1)
	/// Proof: `Game::CommunicationIdentifiers` (`max_values`: None, `max_size`: Some(113), added: 2588, mode: `MaxEncodedLen`)
	/// Storage: `NetworkSuffix::NetworkSuffix` (r:1 w:0)
	/// Proof: `NetworkSuffix::NetworkSuffix` (`max_values`: Some(1), `max_size`: Some(17), added: 512, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn sign_up_with_account_lite_invite(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `663 + n * (178 ±0)`
		//  Estimated: `4625 + n * (3275 ±0)`
		// Minimum execution time: 55_345_000 picoseconds.
		Weight::from_parts(68_160_924, 0)
			.saturating_add(Weight::from_parts(0, 4625))
			// Standard Error: 89_110
			.saturating_add(Weight::from_parts(653_742_859, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(7))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:91 w:91)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerToIndex` (r:1 w:0)
	/// Proof: `Game::PlayerToIndex` (`max_values`: None, `max_size`: Some(90), added: 2565, mode: `MaxEncodedLen`)
	/// Storage: `Game::IndexToPlayer` (r:90 w:0)
	/// Proof: `Game::IndexToPlayer` (`max_values`: None, `max_size`: Some(46), added: 2521, mode: `MaxEncodedLen`)
	/// Storage: `Game::AwardedNftClaimCredits` (r:90 w:90)
	/// Proof: `Game::AwardedNftClaimCredits` (`max_values`: None, `max_size`: Some(77), added: 2552, mode: `MaxEncodedLen`)
	/// Storage: `Game::NftClaimCreditAwards` (r:1 w:1)
	/// Proof: `Game::NftClaimCreditAwards` (`max_values`: None, `max_size`: Some(78014), added: 80489, mode: `MaxEncodedLen`)
	/// Storage: `Game::NftClaimCreditBlocks` (r:90 w:90)
	/// Proof: `Game::NftClaimCreditBlocks` (`max_values`: None, `max_size`: Some(178), added: 2653, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:9 w:9)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThreshold` (r:1 w:0)
	/// Proof: `Score::PersonhoodThreshold` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceRatio` (r:1 w:0)
	/// Proof: `Score::AbsenceGraceRatio` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `Game::PlayerAttendanceHistory` (r:9 w:9)
	/// Proof: `Game::PlayerAttendanceHistory` (`max_values`: None, `max_size`: Some(98), added: 2573, mode: `MaxEncodedLen`)
	/// Storage: `Game::GameParticipantCount` (r:1 w:1)
	/// Proof: `Game::GameParticipantCount` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `Game::PendingNftClaimCreditRootInfo` (r:0 w:1)
	/// Proof: `Game::PendingNftClaimCreditRootInfo` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// The range of component `e` is `[9, 90]`.
	/// The range of component `n` is `[0, 9]`.
	fn report(e: u32, n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6727 + e * (252 ±0) + n * (233 ±0)`
		//  Estimated: `81479 + e * (2570 ±29) + n * (2573 ±0)`
		// Minimum execution time: 487_190_000 picoseconds.
		Weight::from_parts(173_050_518, 0)
			.saturating_add(Weight::from_parts(0, 81479))
			// Standard Error: 89_257
			.saturating_add(Weight::from_parts(28_155_344, 0).saturating_mul(e.into()))
			// Standard Error: 761_452
			.saturating_add(Weight::from_parts(24_724_201, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(14))
			.saturating_add(T::DbWeight::get().reads((4_u64).saturating_mul(e.into())))
			.saturating_add(T::DbWeight::get().reads((4_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(10))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(e.into())))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2570).saturating_mul(e.into()))
			.saturating_add(Weight::from_parts(0, 2573).saturating_mul(n.into()))
	}
	/// Storage: `Game::ArchivedPlayers` (r:1 w:0)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Game` (r:1 w:0)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:1)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(229), added: 2704, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653ad861ea1ebf4800d4b89f4f` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653ad861ea1ebf4800d4b89f4f` (r:1 w:1)
	/// Storage: `Score::Participants` (r:0 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	fn offboard_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `302`
		//  Estimated: `3767`
		// Minimum execution time: 57_012_000 picoseconds.
		Weight::from_parts(59_998_000, 0)
			.saturating_add(Weight::from_parts(0, 3767))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Game::ArchivedPlayers` (r:1 w:1)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Game::Players` (r:1 w:1)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Game` (r:1 w:0)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Game::AliasToStmtAccount` (r:1 w:1)
	/// Proof: `Game::AliasToStmtAccount` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653acecc1507dc1ddd7295951c` (r:1 w:1)
	/// Proof: UNKNOWN KEY `0x3a73746174656d656e745f616c6c6f77616e63653acecc1507dc1ddd7295951c` (r:1 w:1)
	/// Storage: `Score::Participants` (r:0 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Game::StmtAccountToAlias` (r:0 w:1)
	/// Proof: `Game::StmtAccountToAlias` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	fn offboard_person() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `509`
		//  Estimated: `3974`
		// Minimum execution time: 33_860_000 picoseconds.
		Weight::from_parts(35_869_000, 0)
			.saturating_add(Weight::from_parts(0, 3974))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `Game::ArchivedPlayers` (r:1 w:1)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:0 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	fn kickout() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `330`
		//  Estimated: `3593`
		// Minimum execution time: 21_241_000 picoseconds.
		Weight::from_parts(22_699_000, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Game::AvailableInvites` (r:1 w:1)
	/// Proof: `Game::AvailableInvites` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn grant_invites() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `109`
		//  Estimated: `3517`
		// Minimum execution time: 10_753_000 picoseconds.
		Weight::from_parts(11_634_000, 0)
			.saturating_add(Weight::from_parts(0, 3517))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Game::PendingInvites` (r:10000 w:10000)
	/// Proof: `Game::PendingInvites` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
	/// Storage: `Game::AvailableInvites` (r:0 w:1)
	/// Proof: `Game::AvailableInvites` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 10000]`.
	fn remove_available_and_pending_invites(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `763 + n * (52 ±0)`
		//  Estimated: `990 + n * (2571 ±0)`
		// Minimum execution time: 12_811_000 picoseconds.
		Weight::from_parts(13_040_000, 0)
			.saturating_add(Weight::from_parts(0, 990))
			// Standard Error: 5_437
			.saturating_add(Weight::from_parts(1_982_469, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2571).saturating_mul(n.into()))
	}
	/// Storage: `Game::AvailableInvites` (r:1 w:1)
	/// Proof: `Game::AvailableInvites` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Game::PendingInvites` (r:1 w:1)
	/// Proof: `Game::PendingInvites` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
	fn set_invite_ticket() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `187`
		//  Estimated: `3561`
		// Minimum execution time: 17_857_000 picoseconds.
		Weight::from_parts(18_972_000, 0)
			.saturating_add(Weight::from_parts(0, 3561))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Game::PendingInvites` (r:1 w:1)
	/// Proof: `Game::PendingInvites` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
	/// Storage: `Game::AvailableInvites` (r:1 w:1)
	/// Proof: `Game::AvailableInvites` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn cancel_invite_ticket() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `231`
		//  Estimated: `3561`
		// Minimum execution time: 16_628_000 picoseconds.
		Weight::from_parts(17_745_000, 0)
			.saturating_add(Weight::from_parts(0, 3561))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Game::GameSchedules` (r:1 w:1)
	/// Proof: `Game::GameSchedules` (`max_values`: Some(1), `max_size`: Some(121849), added: 122344, mode: `MaxEncodedLen`)
	/// Storage: `Game::Game` (r:1 w:0)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::StoredPhaseDurations` (r:1 w:0)
	/// Proof: `Game::StoredPhaseDurations` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 12]`.
	fn schedule_games(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `324`
		//  Estimated: `123334`
		// Minimum execution time: 32_166_000 picoseconds.
		Weight::from_parts(25_437_257, 0)
			.saturating_add(Weight::from_parts(0, 123334))
			// Standard Error: 12_963
			.saturating_add(Weight::from_parts(8_511_288, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Game::GameSchedules` (r:1 w:1)
	/// Proof: `Game::GameSchedules` (`max_values`: Some(1), `max_size`: Some(121849), added: 122344, mode: `MaxEncodedLen`)
	fn remove_scheduled_game() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `8705`
		//  Estimated: `123334`
		// Minimum execution time: 181_013_000 picoseconds.
		Weight::from_parts(185_872_000, 0)
			.saturating_add(Weight::from_parts(0, 123334))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Game::PlayDepositAmount` (r:0 w:1)
	/// Proof: `Game::PlayDepositAmount` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	fn set_play_deposit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_544_000 picoseconds.
		Weight::from_parts(6_013_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Game::Players` (r:1 w:0)
	/// Proof: `Game::Players` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::Game` (r:1 w:0)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Game::StmtAccountToAlias` (r:1 w:0)
	/// Proof: `Game::StmtAccountToAlias` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `Game::ArchivedPlayers` (r:1 w:0)
	/// Proof: `Game::ArchivedPlayers` (`max_values`: None, `max_size`: Some(58), added: 2533, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:0)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Game::PendingInvites` (r:1 w:1)
	/// Proof: `Game::PendingInvites` (`max_values`: None, `max_size`: Some(96), added: 2571, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:0)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Registrations` (r:16 w:0)
	/// Proof: `Airdrop::Registrations` (`max_values`: None, `max_size`: Some(105), added: 2580, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn as_invited_tx_ext(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `677 + n * (178 ±0)`
		//  Estimated: `3593 + n * (3275 ±0)`
		// Minimum execution time: 92_930_000 picoseconds.
		Weight::from_parts(163_166_561, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			// Standard Error: 274_654
			.saturating_add(Weight::from_parts(656_812_743, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn process_reporting() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `264`
		//  Estimated: `1559`
		// Minimum execution time: 8_712_000 picoseconds.
		Weight::from_parts(9_389_000, 0)
			.saturating_add(Weight::from_parts(0, 1559))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Game::PlayerAttendanceHistory` (r:1 w:1)
	/// Proof: `Game::PlayerAttendanceHistory` (`max_values`: None, `max_size`: Some(98), added: 2573, mode: `MaxEncodedLen`)
	/// Storage: `Game::GameParticipantCount` (r:1 w:1)
	/// Proof: `Game::GameParticipantCount` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	fn insert_attendance_history() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `235`
		//  Estimated: `3563`
		// Minimum execution time: 9_954_000 picoseconds.
		Weight::from_parts(10_788_000, 0)
			.saturating_add(Weight::from_parts(0, 3563))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Game::Game` (r:1 w:1)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:32)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	fn cancel_game() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3939`
		//  Estimated: `53390`
		// Minimum execution time: 1_162_103_000 picoseconds.
		Weight::from_parts(1_211_290_000, 0)
			.saturating_add(Weight::from_parts(0, 53390))
			.saturating_add(T::DbWeight::get().reads(22))
			.saturating_add(T::DbWeight::get().writes(54))
	}
	/// Storage: `Game::Game` (r:1 w:0)
	/// Proof: `Game::Game` (`max_values`: Some(1), `max_size`: Some(74), added: 569, mode: `MaxEncodedLen`)
	/// Storage: `Game::StoredPhaseDurations` (r:0 w:1)
	/// Proof: `Game::StoredPhaseDurations` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	fn set_game_phases() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `230`
		//  Estimated: `1559`
		// Minimum execution time: 12_573_000 picoseconds.
		Weight::from_parts(13_696_000, 0)
			.saturating_add(Weight::from_parts(0, 1559))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Airdrop::Events` (r:16 w:16)
	/// Proof: `Airdrop::Events` (`max_values`: None, `max_size`: Some(800), added: 3275, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(847), added: 3322, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `Airdrop::ActionSchedule` (r:0 w:16)
	/// Proof: `Airdrop::ActionSchedule` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 16]`.
	fn on_game_cancelled(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `851 + n * (174 ±0)`
		//  Estimated: `7404 + n * (3275 ±0)`
		// Minimum execution time: 3_004_000 picoseconds.
		Weight::from_parts(23_900_724, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			// Standard Error: 103_231
			.saturating_add(Weight::from_parts(70_252_861, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 3275).saturating_mul(n.into()))
	}
	/// Storage: `Score::Participants` (r:1 w:0)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
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
	fn claim_airdrop() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1504`
		//  Estimated: `7404`
		// Minimum execution time: 129_327_000 picoseconds.
		Weight::from_parts(136_274_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(11))
			.saturating_add(T::DbWeight::get().writes(8))
	}
}
