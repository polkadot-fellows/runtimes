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

//! Autogenerated weights for `pallet_scheduler`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-01-05, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./collectives-polkadot-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot-parachain
// benchmark
// pallet
// --chain=./collectives-polkadot-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_scheduler
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./collectives-polkadot-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_scheduler`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_scheduler::WeightInfo for WeightInfo<T> {
	/// Storage: `Scheduler::IncompleteSince` (r:1 w:1)
	/// Proof: `Scheduler::IncompleteSince` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn service_agendas_base() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `31`
		//  Estimated: `1489`
		// Minimum execution time: 4_340_000 picoseconds.
		Weight::from_parts(4_450_000, 0)
			.saturating_add(Weight::from_parts(0, 1489))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Scheduler::Agenda` (r:1 w:1)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[0, 200]`.
	fn service_agenda_base(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `77 + s * (177 ±0)`
		//  Estimated: `159279`
		// Minimum execution time: 4_510_000 picoseconds.
		Weight::from_parts(9_038_594, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			// Standard Error: 3_005
			.saturating_add(Weight::from_parts(437_294, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	fn service_task_base() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_380_000 picoseconds.
		Weight::from_parts(4_560_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// Storage: `Preimage::PreimageFor` (r:1 w:1)
	/// Proof: `Preimage::PreimageFor` (`max_values`: None, `max_size`: Some(4194344), added: 4196819, mode: `Measured`)
	/// Storage: `Preimage::StatusFor` (r:1 w:0)
	/// Proof: `Preimage::StatusFor` (`max_values`: None, `max_size`: Some(91), added: 2566, mode: `MaxEncodedLen`)
	/// Storage: `Preimage::RequestStatusFor` (r:1 w:1)
	/// Proof: `Preimage::RequestStatusFor` (`max_values`: None, `max_size`: Some(91), added: 2566, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[128, 4194304]`.
	fn service_task_fetched(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66 + s * (1 ±0)`
		//  Estimated: `3556 + s * (1 ±0)`
		// Minimum execution time: 16_320_000 picoseconds.
		Weight::from_parts(16_792_000, 3556)
			// Standard Error: 263
			.saturating_add(Weight::from_parts(23_402, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
			.saturating_add(Weight::from_parts(0, 1).saturating_mul(s.into()))
	}
	/// Storage: `Scheduler::Lookup` (r:0 w:1)
	/// Proof: `Scheduler::Lookup` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	fn service_task_named() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_250_000 picoseconds.
		Weight::from_parts(6_420_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	fn service_task_periodic() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_330_000 picoseconds.
		Weight::from_parts(4_460_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	fn execute_dispatch_signed() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 2_840_000 picoseconds.
		Weight::from_parts(2_980_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	fn execute_dispatch_unsigned() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 2_880_000 picoseconds.
		Weight::from_parts(2_970_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// Storage: `Scheduler::Agenda` (r:1 w:1)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[0, 199]`.
	fn schedule(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `77 + s * (177 ±0)`
		//  Estimated: `159279`
		// Minimum execution time: 13_361_000 picoseconds.
		Weight::from_parts(17_556_848, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			// Standard Error: 3_179
			.saturating_add(Weight::from_parts(451_359, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Scheduler::Agenda` (r:1 w:1)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Retries` (r:0 w:1)
	/// Proof: `Scheduler::Retries` (`max_values`: None, `max_size`: Some(30), added: 2505, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Lookup` (r:0 w:1)
	/// Proof: `Scheduler::Lookup` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[1, 200]`.
	fn cancel(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `77 + s * (177 ±0)`
		//  Estimated: `159279`
		// Minimum execution time: 22_030_000 picoseconds.
		Weight::from_parts(20_183_995, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			// Standard Error: 3_096
			.saturating_add(Weight::from_parts(704_936, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Scheduler::Lookup` (r:1 w:1)
	/// Proof: `Scheduler::Lookup` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Agenda` (r:1 w:1)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[0, 199]`.
	fn schedule_named(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `468 + s * (179 ±0)`
		//  Estimated: `159279`
		// Minimum execution time: 18_370_000 picoseconds.
		Weight::from_parts(25_840_443, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			// Standard Error: 3_228
			.saturating_add(Weight::from_parts(467_768, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Scheduler::Lookup` (r:1 w:1)
	/// Proof: `Scheduler::Lookup` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Agenda` (r:1 w:1)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Retries` (r:0 w:1)
	/// Proof: `Scheduler::Retries` (`max_values`: None, `max_size`: Some(30), added: 2505, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[1, 200]`.
	fn cancel_named(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `509 + s * (179 ±0)`
		//  Estimated: `159279`
		// Minimum execution time: 26_200_000 picoseconds.
		Weight::from_parts(26_569_424, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			// Standard Error: 3_194
			.saturating_add(Weight::from_parts(716_769, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Scheduler::Agenda` (r:1 w:1)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Retries` (r:0 w:1)
	/// Proof: `Scheduler::Retries` (`max_values`: None, `max_size`: Some(30), added: 2505, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[1, 200]`.
	fn schedule_retry(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `118`
		//  Estimated: `159279`
		// Minimum execution time: 13_971_000 picoseconds.
		Weight::from_parts(15_125_943, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			// Standard Error: 350
			.saturating_add(Weight::from_parts(14_541, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Scheduler::Agenda` (r:1 w:0)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Retries` (r:0 w:1)
	/// Proof: `Scheduler::Retries` (`max_values`: None, `max_size`: Some(30), added: 2505, mode: `MaxEncodedLen`)
	fn set_retry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `35481`
		//  Estimated: `159279`
		// Minimum execution time: 79_511_000 picoseconds.
		Weight::from_parts(80_930_000, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Scheduler::Lookup` (r:1 w:0)
	/// Proof: `Scheduler::Lookup` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Agenda` (r:1 w:0)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Retries` (r:0 w:1)
	/// Proof: `Scheduler::Retries` (`max_values`: None, `max_size`: Some(30), added: 2505, mode: `MaxEncodedLen`)
	fn set_retry_named() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `36253`
		//  Estimated: `159279`
		// Minimum execution time: 88_061_000 picoseconds.
		Weight::from_parts(92_131_000, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Scheduler::Agenda` (r:1 w:0)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Retries` (r:0 w:1)
	/// Proof: `Scheduler::Retries` (`max_values`: None, `max_size`: Some(30), added: 2505, mode: `MaxEncodedLen`)
	fn cancel_retry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `35493`
		//  Estimated: `159279`
		// Minimum execution time: 77_201_000 picoseconds.
		Weight::from_parts(78_730_000, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Scheduler::Lookup` (r:1 w:0)
	/// Proof: `Scheduler::Lookup` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Agenda` (r:1 w:0)
	/// Proof: `Scheduler::Agenda` (`max_values`: None, `max_size`: Some(155814), added: 158289, mode: `MaxEncodedLen`)
	/// Storage: `Scheduler::Retries` (r:0 w:1)
	/// Proof: `Scheduler::Retries` (`max_values`: None, `max_size`: Some(30), added: 2505, mode: `MaxEncodedLen`)
	fn cancel_retry_named() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `36265`
		//  Estimated: `159279`
		// Minimum execution time: 86_640_000 picoseconds.
		Weight::from_parts(88_381_000, 0)
			.saturating_add(Weight::from_parts(0, 159279))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
