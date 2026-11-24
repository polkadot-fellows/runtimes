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

/// Weight functions for `indiv_pallet_score`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> indiv_pallet_score::WeightInfo for WeightInfo<T> {
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundSchedules` (r:1 w:1)
	/// Proof: `Score::RoundSchedules` (`max_values`: Some(1), `max_size`: Some(241), added: 736, mode: `MaxEncodedLen`)
	fn schedule_payout_rounds() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `832`
		//  Estimated: `4348`
		// Minimum execution time: 62_654_000 picoseconds.
		Weight::from_parts(66_940_000, 0)
			.saturating_add(Weight::from_parts(0, 4348))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Score::RoundSchedules` (r:1 w:1)
	/// Proof: `Score::RoundSchedules` (`max_values`: Some(1), `max_size`: Some(241), added: 736, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	fn remove_payout_schedule() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1091`
		//  Estimated: `4348`
		// Minimum execution time: 56_250_000 picoseconds.
		Weight::from_parts(58_543_000, 0)
			.saturating_add(Weight::from_parts(0, 4348))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Score::CurrentRoundIndex` (r:1 w:1)
	/// Proof: `Score::CurrentRoundIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundPlanning` (r:1 w:1)
	/// Proof: `Score::RoundPlanning` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundSchedules` (r:1 w:1)
	/// Proof: `Score::RoundSchedules` (`max_values`: Some(1), `max_size`: Some(241), added: 736, mode: `MaxEncodedLen`)
	/// Storage: `Score::CurrentRoundPoints` (r:1 w:1)
	/// Proof: `Score::CurrentRoundPoints` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundPayouts` (r:0 w:1)
	/// Proof: `Score::RoundPayouts` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	fn transition_round() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `474`
		//  Estimated: `1726`
		// Minimum execution time: 20_192_000 picoseconds.
		Weight::from_parts(21_352_000, 0)
			.saturating_add(Weight::from_parts(0, 1726))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Score::CurrentRoundIndex` (r:1 w:0)
	/// Proof: `Score::CurrentRoundIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundPlanning` (r:1 w:0)
	/// Proof: `Score::RoundPlanning` (`max_values`: Some(1), `max_size`: Some(20), added: 515, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundSchedules` (r:1 w:0)
	/// Proof: `Score::RoundSchedules` (`max_values`: Some(1), `max_size`: Some(241), added: 736, mode: `MaxEncodedLen`)
	fn authorize_transition_round() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `209`
		//  Estimated: `1726`
		// Minimum execution time: 6_942_000 picoseconds.
		Weight::from_parts(7_411_000, 0)
			.saturating_add(Weight::from_parts(0, 1726))
			.saturating_add(T::DbWeight::get().reads(3))
	}
	/// Storage: `Score::RoundPayouts` (r:1 w:1)
	/// Proof: `Score::RoundPayouts` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundsPointsForParticipant` (r:10001 w:10000)
	/// Proof: `Score::RoundsPointsForParticipant` (`max_values`: None, `max_size`: Some(73), added: 2548, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:10000 w:10000)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[1, 10000]`.
	fn operate_payout_round(l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2274 + l * (137 ±0)`
		//  Estimated: `4348 + l * (2561 ±0)`
		// Minimum execution time: 120_240_000 picoseconds.
		Weight::from_parts(121_164_000, 0)
			.saturating_add(Weight::from_parts(0, 4348))
			// Standard Error: 29_872
			.saturating_add(Weight::from_parts(16_923_161, 0).saturating_mul(l.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(l.into())))
			.saturating_add(T::DbWeight::get().writes(5))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(l.into())))
			.saturating_add(Weight::from_parts(0, 2561).saturating_mul(l.into()))
	}
	/// Storage: `Score::RoundPayouts` (r:1 w:0)
	/// Proof: `Score::RoundPayouts` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// The range of component `l` is `[1, 10000]`.
	fn authorize_operate_payout_round(l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `308`
		//  Estimated: `3537`
		// Minimum execution time: 8_726_000 picoseconds.
		Weight::from_parts(17_371_428, 0)
			.saturating_add(Weight::from_parts(0, 3537))
			// Standard Error: 27
			.saturating_add(Weight::from_parts(910, 0).saturating_mul(l.into()))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Score::CurrentRoundIndex` (r:1 w:0)
	/// Proof: `Score::CurrentRoundIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Score::CurrentRoundPoints` (r:1 w:1)
	/// Proof: `Score::CurrentRoundPoints` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Score::RoundsPointsForParticipant` (r:1 w:1)
	/// Proof: `Score::RoundsPointsForParticipant` (`max_values`: None, `max_size`: Some(73), added: 2548, mode: `MaxEncodedLen`)
	fn cash_out() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `242`
		//  Estimated: `3551`
		// Minimum execution time: 20_633_000 picoseconds.
		Weight::from_parts(21_998_000, 0)
			.saturating_add(Weight::from_parts(0, 3551))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `NetworkSuffix::NetworkSuffix` (r:1 w:0)
	/// Proof: `NetworkSuffix::NetworkSuffix` (`max_values`: Some(1), `max_size`: Some(17), added: 512, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(808), added: 3283, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:2 w:2)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(732), added: 3207, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::Holds` (r:1 w:1)
	/// Proof: `AssetsHolder::Holds` (`max_values`: None, `max_size`: Some(883), added: 3358, mode: `MaxEncodedLen`)
	/// Storage: `AssetsHolder::BalancesOnHold` (r:1 w:1)
	/// Proof: `AssetsHolder::BalancesOnHold` (`max_values`: None, `max_size`: Some(682), added: 3157, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn redeem_credit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `957`
		//  Estimated: `7404`
		// Minimum execution time: 106_833_000 picoseconds.
		Weight::from_parts(113_933_000, 0)
			.saturating_add(Weight::from_parts(0, 7404))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `People::NextPersonalId` (r:1 w:1)
	/// Proof: `People::NextPersonalId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `People::Keys` (r:1 w:1)
	/// Proof: `People::Keys` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
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
	/// Storage: `People::CounterForKeys` (r:1 w:1)
	/// Proof: `People::CounterForKeys` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `People::People` (r:0 w:1)
	/// Proof: `People::People` (`max_values`: None, `max_size`: Some(89), added: 2564, mode: `MaxEncodedLen`)
	/// Storage: `People::ReservedPersonalId` (r:0 w:1)
	/// Proof: `People::ReservedPersonalId` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	fn register() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `528`
		//  Estimated: `11671`
		// Minimum execution time: 2_577_579_000 picoseconds.
		Weight::from_parts(2_608_289_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Score::AbsenceGraceSchedule` (r:0 w:1)
	/// Proof: `Score::AbsenceGraceSchedule` (`max_values`: Some(1), `max_size`: Some(49), added: 544, mode: `MaxEncodedLen`)
	fn set_absence_grace_schedule() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_133_000 picoseconds.
		Weight::from_parts(6_652_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Score::PersonhoodThresholdSchedule` (r:0 w:1)
	/// Proof: `Score::PersonhoodThresholdSchedule` (`max_values`: Some(1), `max_size`: Some(81), added: 576, mode: `MaxEncodedLen`)
	fn set_personhood_threshold_schedule() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_226_000 picoseconds.
		Weight::from_parts(6_723_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Members::ActiveMembers` (r:1 w:0)
	/// Proof: `Members::ActiveMembers` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThresholdSchedule` (r:1 w:0)
	/// Proof: `Score::PersonhoodThresholdSchedule` (`max_values`: Some(1), `max_size`: Some(81), added: 576, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceSchedule` (r:1 w:0)
	/// Proof: `Score::AbsenceGraceSchedule` (`max_values`: Some(1), `max_size`: Some(49), added: 544, mode: `MaxEncodedLen`)
	/// Storage: `Members::Collections` (r:1 w:0)
	/// Proof: `Members::Collections` (`max_values`: None, `max_size`: Some(646), added: 3121, mode: `MaxEncodedLen`)
	/// Storage: `Members::RingsState` (r:1 w:1)
	/// Proof: `Members::RingsState` (`max_values`: None, `max_size`: Some(34), added: 2509, mode: `MaxEncodedLen`)
	/// Storage: `Score::Participants` (r:1 w:1)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	/// Storage: `People::People` (r:1 w:1)
	/// Proof: `People::People` (`max_values`: None, `max_size`: Some(89), added: 2564, mode: `MaxEncodedLen`)
	/// Storage: `Members::Members` (r:1 w:1)
	/// Proof: `Members::Members` (`max_values`: None, `max_size`: Some(93), added: 2568, mode: `MaxEncodedLen`)
	/// Storage: `Members::OnboardingQueue` (r:1 w:1)
	/// Proof: `Members::OnboardingQueue` (`max_values`: None, `max_size`: Some(8206), added: 10681, mode: `MaxEncodedLen`)
	/// Storage: `Score::AbsenceGraceRatio` (r:0 w:1)
	/// Proof: `Score::AbsenceGraceRatio` (`max_values`: Some(1), `max_size`: Some(2), added: 497, mode: `MaxEncodedLen`)
	/// Storage: `Score::PersonhoodThreshold` (r:0 w:1)
	/// Proof: `Score::PersonhoodThreshold` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	fn force_set_attendance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `989`
		//  Estimated: `11671`
		// Minimum execution time: 69_294_000 picoseconds.
		Weight::from_parts(72_618_000, 0)
			.saturating_add(Weight::from_parts(0, 11671))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Score::Participants` (r:1 w:0)
	/// Proof: `Score::Participants` (`max_values`: None, `max_size`: Some(86), added: 2561, mode: `MaxEncodedLen`)
	fn as_participant_tx_ext() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `242`
		//  Estimated: `3551`
		// Minimum execution time: 12_988_000 picoseconds.
		Weight::from_parts(13_744_000, 0)
			.saturating_add(Weight::from_parts(0, 3551))
			.saturating_add(T::DbWeight::get().reads(1))
	}
}
