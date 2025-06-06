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

//! Autogenerated weights for `pallet_migrations`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 47.0.0
//! DATE: 2025-04-08, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `e502a20f0dc0`, CPU: `QEMU Virtual CPU version 2.5+`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --extrinsic=*
// --runtime=target/production/wbuild/people-kusama-runtime/people_kusama_runtime.wasm
// --pallet=pallet_migrations
// --header=/_work/fellowship-001/runtimes/runtimes/.github/scripts/cmd/file_header.txt
// --output=./system-parachains/people/people-kusama/src/weights
// --wasm-execution=compiled
// --steps=50
// --repeat=20
// --heap-pages=4096

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_migrations`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_migrations::WeightInfo for WeightInfo<T> {
	/// Storage: `MultiBlockMigrations::Cursor` (r:1 w:1)
	/// Proof: `MultiBlockMigrations::Cursor` (`max_values`: Some(1), `max_size`: Some(65550), added: 66045, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	fn onboard_new_mbms() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `71`
		//  Estimated: `67035`
		// Minimum execution time: 8_190_000 picoseconds.
		Weight::from_parts(9_520_000, 0)
			.saturating_add(Weight::from_parts(0, 67035))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `MultiBlockMigrations::Cursor` (r:1 w:0)
	/// Proof: `MultiBlockMigrations::Cursor` (`max_values`: Some(1), `max_size`: Some(65550), added: 66045, mode: `MaxEncodedLen`)
	fn progress_mbms_none() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `67035`
		// Minimum execution time: 2_240_000 picoseconds.
		Weight::from_parts(2_380_000, 0)
			.saturating_add(Weight::from_parts(0, 67035))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Storage: `MultiBlockMigrations::Cursor` (r:0 w:1)
	/// Proof: `MultiBlockMigrations::Cursor` (`max_values`: Some(1), `max_size`: Some(65550), added: 66045, mode: `MaxEncodedLen`)
	fn exec_migration_completed() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `29`
		//  Estimated: `3494`
		// Minimum execution time: 6_070_000 picoseconds.
		Weight::from_parts(7_240_000, 0)
			.saturating_add(Weight::from_parts(0, 3494))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Storage: `MultiBlockMigrations::Historic` (r:1 w:0)
	/// Proof: `MultiBlockMigrations::Historic` (`max_values`: None, `max_size`: Some(266), added: 2741, mode: `MaxEncodedLen`)
	fn exec_migration_skipped_historic() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `125`
		//  Estimated: `3731`
		// Minimum execution time: 11_190_000 picoseconds.
		Weight::from_parts(13_550_000, 0)
			.saturating_add(Weight::from_parts(0, 3731))
			.saturating_add(T::DbWeight::get().reads(2))
	}
	/// Storage: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Storage: `MultiBlockMigrations::Historic` (r:1 w:0)
	/// Proof: `MultiBlockMigrations::Historic` (`max_values`: None, `max_size`: Some(266), added: 2741, mode: `MaxEncodedLen`)
	fn exec_migration_advance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `71`
		//  Estimated: `3731`
		// Minimum execution time: 10_480_000 picoseconds.
		Weight::from_parts(11_680_000, 0)
			.saturating_add(Weight::from_parts(0, 3731))
			.saturating_add(T::DbWeight::get().reads(2))
	}
	/// Storage: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Storage: `MultiBlockMigrations::Historic` (r:1 w:1)
	/// Proof: `MultiBlockMigrations::Historic` (`max_values`: None, `max_size`: Some(266), added: 2741, mode: `MaxEncodedLen`)
	fn exec_migration_complete() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `71`
		//  Estimated: `3731`
		// Minimum execution time: 12_421_000 picoseconds.
		Weight::from_parts(14_410_000, 0)
			.saturating_add(Weight::from_parts(0, 3731))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Storage: `MultiBlockMigrations::Historic` (r:1 w:0)
	/// Proof: `MultiBlockMigrations::Historic` (`max_values`: None, `max_size`: Some(266), added: 2741, mode: `MaxEncodedLen`)
	/// Storage: `MultiBlockMigrations::Cursor` (r:0 w:1)
	/// Proof: `MultiBlockMigrations::Cursor` (`max_values`: Some(1), `max_size`: Some(65550), added: 66045, mode: `MaxEncodedLen`)
	fn exec_migration_fail() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `71`
		//  Estimated: `3731`
		// Minimum execution time: 13_050_000 picoseconds.
		Weight::from_parts(15_000_000, 0)
			.saturating_add(Weight::from_parts(0, 3731))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	fn on_init_loop() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 200_000 picoseconds.
		Weight::from_parts(250_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// Storage: `MultiBlockMigrations::Cursor` (r:0 w:1)
	/// Proof: `MultiBlockMigrations::Cursor` (`max_values`: Some(1), `max_size`: Some(65550), added: 66045, mode: `MaxEncodedLen`)
	fn force_set_cursor() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 3_110_000 picoseconds.
		Weight::from_parts(3_570_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `MultiBlockMigrations::Cursor` (r:0 w:1)
	/// Proof: `MultiBlockMigrations::Cursor` (`max_values`: Some(1), `max_size`: Some(65550), added: 66045, mode: `MaxEncodedLen`)
	fn force_set_active_cursor() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 3_460_000 picoseconds.
		Weight::from_parts(4_261_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `MultiBlockMigrations::Cursor` (r:1 w:0)
	/// Proof: `MultiBlockMigrations::Cursor` (`max_values`: Some(1), `max_size`: Some(65550), added: 66045, mode: `MaxEncodedLen`)
	/// Storage: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x583359fe0e84d953a9dd84e8addb08a5` (r:1 w:0)
	fn force_onboard_mbms() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `85`
		//  Estimated: `67035`
		// Minimum execution time: 6_690_000 picoseconds.
		Weight::from_parts(7_100_000, 0)
			.saturating_add(Weight::from_parts(0, 67035))
			.saturating_add(T::DbWeight::get().reads(2))
	}
	/// Storage: `MultiBlockMigrations::Historic` (r:256 w:256)
	/// Proof: `MultiBlockMigrations::Historic` (`max_values`: None, `max_size`: Some(266), added: 2741, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 256]`.
	fn clear_historic(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1022 + n * (271 ±0)`
		//  Estimated: `3834 + n * (2740 ±0)`
		// Minimum execution time: 19_940_000 picoseconds.
		Weight::from_parts(14_507_666, 0)
			.saturating_add(Weight::from_parts(0, 3834))
			// Standard Error: 3_687
			.saturating_add(Weight::from_parts(1_554_578, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2740).saturating_mul(n.into()))
	}
	/// Storage: `Skipped::Metadata` (r:0 w:0)
	/// Proof: `Skipped::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `n` is `[0, 2048]`.
	fn reset_pallet_migration(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1642 + n * (38 ±0)`
		//  Estimated: `720 + n * (39 ±0)`
		// Minimum execution time: 1_919_000 picoseconds.
		Weight::from_parts(5_740_246, 0)
			.saturating_add(Weight::from_parts(0, 720))
			// Standard Error: 1_463
			.saturating_add(Weight::from_parts(836_551, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 39).saturating_mul(n.into()))
	}
}
