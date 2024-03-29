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
//! Autogenerated weights for `pallet_timestamp`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-03-10, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./encointer-kusama-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=./encointer-kusama-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_timestamp
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./encointer-kusama-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_timestamp`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_timestamp::WeightInfo for WeightInfo<T> {
	/// Storage: `Timestamp::Now` (r:1 w:1)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `EncointerScheduler::NextPhaseTimestamp` (r:1 w:0)
	/// Proof: `EncointerScheduler::NextPhaseTimestamp` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `EncointerScheduler::CurrentCeremonyIndex` (r:1 w:1)
	/// Proof: `EncointerScheduler::CurrentCeremonyIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `EncointerScheduler::PhaseDurations` (r:3 w:0)
	/// Proof: `EncointerScheduler::PhaseDurations` (`max_values`: None, `max_size`: Some(25), added: 2500, mode: `MaxEncodedLen`)
	fn set() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `95`
		//  Estimated: `8490`
		// Minimum execution time: 16_108_000 picoseconds.
		Weight::from_parts(16_764_000, 0)
			.saturating_add(Weight::from_parts(0, 8490))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	fn on_finalize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `94`
		//  Estimated: `0`
		// Minimum execution time: 3_257_000 picoseconds.
		Weight::from_parts(3_395_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
	}
}
