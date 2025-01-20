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

//! Autogenerated weights for `runtime_parachains::on_demand`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-01-07, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./kusama-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=./kusama-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=runtime_parachains::on_demand
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./kusama-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `runtime_parachains::on_demand`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> runtime_parachains::on_demand::WeightInfo for WeightInfo<T> {
	/// Storage: `OnDemandAssignmentProvider::QueueStatus` (r:1 w:1)
	/// Proof: `OnDemandAssignmentProvider::QueueStatus` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `OnDemandAssignmentProvider::Revenue` (r:1 w:1)
	/// Proof: `OnDemandAssignmentProvider::Revenue` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `OnDemandAssignmentProvider::ParaIdAffinity` (r:1 w:0)
	/// Proof: `OnDemandAssignmentProvider::ParaIdAffinity` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `OnDemandAssignmentProvider::FreeEntries` (r:1 w:1)
	/// Proof: `OnDemandAssignmentProvider::FreeEntries` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `s` is `[1, 9999]`.
	fn place_order_keep_alive(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `237 + s * (8 ±0)`
		//  Estimated: `3700 + s * (8 ±0)`
		// Minimum execution time: 57_281_000 picoseconds.
		Weight::from_parts(52_107_948, 0)
			.saturating_add(Weight::from_parts(0, 3700))
			// Standard Error: 102
			.saturating_add(Weight::from_parts(18_089, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 8).saturating_mul(s.into()))
	}
	/// Storage: `OnDemandAssignmentProvider::QueueStatus` (r:1 w:1)
	/// Proof: `OnDemandAssignmentProvider::QueueStatus` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `OnDemandAssignmentProvider::Revenue` (r:1 w:1)
	/// Proof: `OnDemandAssignmentProvider::Revenue` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `OnDemandAssignmentProvider::ParaIdAffinity` (r:1 w:0)
	/// Proof: `OnDemandAssignmentProvider::ParaIdAffinity` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `OnDemandAssignmentProvider::FreeEntries` (r:1 w:1)
	/// Proof: `OnDemandAssignmentProvider::FreeEntries` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `s` is `[1, 9999]`.
	fn place_order_allow_death(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `237 + s * (8 ±0)`
		//  Estimated: `3700 + s * (8 ±0)`
		// Minimum execution time: 56_161_000 picoseconds.
		Weight::from_parts(52_133_515, 0)
			.saturating_add(Weight::from_parts(0, 3700))
			// Standard Error: 98
			.saturating_add(Weight::from_parts(17_651, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 8).saturating_mul(s.into()))
	}
}