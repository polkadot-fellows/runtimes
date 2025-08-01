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

//! Autogenerated weights for `pallet_utility`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 47.1.0
//! DATE: 2025-06-16, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `4e7e37be56c6`, CPU: `QEMU Virtual CPU version 2.5+`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --extrinsic=*
// --runtime=target/production/wbuild/staging-kusama-runtime/staging_kusama_runtime.wasm
// --pallet=pallet_utility
// --header=/_work/fellowship-001/runtimes/runtimes/.github/scripts/cmd/file_header.txt
// --output=./relay/kusama/src/weights
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

/// Weight functions for `pallet_utility`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_utility::WeightInfo for WeightInfo<T> {
	/// The range of component `c` is `[0, 1000]`.
	fn batch(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_989_000 picoseconds.
		Weight::from_parts(7_240_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 4_977
			.saturating_add(Weight::from_parts(4_299_874, 0).saturating_mul(c.into()))
	}
	fn as_derivative() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_390_000 picoseconds.
		Weight::from_parts(6_940_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// The range of component `c` is `[0, 1000]`.
	fn batch_all(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_010_000 picoseconds.
		Weight::from_parts(25_003_753, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 5_216
			.saturating_add(Weight::from_parts(4_570_156, 0).saturating_mul(c.into()))
	}
	fn dispatch_as() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_310_000 picoseconds.
		Weight::from_parts(10_019_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	/// The range of component `c` is `[0, 1000]`.
	fn force_batch(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_640_000 picoseconds.
		Weight::from_parts(1_982_971, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 6_840
			.saturating_add(Weight::from_parts(4_296_491, 0).saturating_mul(c.into()))
	}
	fn dispatch_as_fallible() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 9_110_000 picoseconds.
		Weight::from_parts(9_600_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
	fn if_else() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 11_290_000 picoseconds.
		Weight::from_parts(12_610_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
}
