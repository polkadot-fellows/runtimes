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

//! Autogenerated weights for `frame_election_provider_support`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-08-14, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./polkadot-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=./polkadot-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=frame_election_provider_support
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./polkadot-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `frame_election_provider_support`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> frame_election_provider_support::WeightInfo for WeightInfo<T> {
	/// The range of component `v` is `[1000, 2000]`.
	/// The range of component `t` is `[500, 1000]`.
	/// The range of component `d` is `[5, 16]`.
	fn phragmen(v: u32, _t: u32, d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 8_562_318_000 picoseconds.
		Weight::from_parts(8_587_018_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 198_113
			.saturating_add(Weight::from_parts(8_218_845, 0).saturating_mul(v.into()))
			// Standard Error: 20_254_434
			.saturating_add(Weight::from_parts(2_152_051_772, 0).saturating_mul(d.into()))
	}
	/// The range of component `v` is `[1000, 2000]`.
	/// The range of component `t` is `[500, 1000]`.
	/// The range of component `d` is `[5, 16]`.
	fn phragmms(v: u32, _t: u32, d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_953_357_000 picoseconds.
		Weight::from_parts(5_971_487_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 155_998
			.saturating_add(Weight::from_parts(6_301_190, 0).saturating_mul(v.into()))
			// Standard Error: 15_948_746
			.saturating_add(Weight::from_parts(1_817_448_899, 0).saturating_mul(d.into()))
	}
}
