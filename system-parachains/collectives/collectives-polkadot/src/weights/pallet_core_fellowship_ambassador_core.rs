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

//! Autogenerated weights for `pallet_core_fellowship`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-08-15, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
// --pallet=pallet_core_fellowship
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

/// Weight functions for `pallet_core_fellowship`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_core_fellowship::WeightInfo for WeightInfo<T> {
	/// Storage: `AmbassadorCore::Params` (r:0 w:1)
	/// Proof: `AmbassadorCore::Params` (`max_values`: Some(1), `max_size`: Some(368), added: 863, mode: `MaxEncodedLen`)
	fn set_params() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_060_000 picoseconds.
		Weight::from_parts(7_240_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `AmbassadorCore::Params` (r:1 w:1)
	/// Proof: `AmbassadorCore::Params` (`max_values`: Some(1), `max_size`: Some(368), added: 863, mode: `MaxEncodedLen`)
	fn set_partial_params() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `471`
		//  Estimated: `1853`
		// Minimum execution time: 14_270_000 picoseconds.
		Weight::from_parts(14_610_000, 0)
			.saturating_add(Weight::from_parts(0, 1853))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::Members` (r:1 w:1)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Params` (r:1 w:0)
	/// Proof: `AmbassadorCore::Params` (`max_values`: Some(1), `max_size`: Some(368), added: 863, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::MemberCount` (r:1 w:1)
	/// Proof: `AmbassadorCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IdToIndex` (r:1 w:1)
	/// Proof: `AmbassadorCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::MemberEvidence` (r:1 w:1)
	/// Proof: `AmbassadorCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IndexToId` (r:0 w:1)
	/// Proof: `AmbassadorCollective::IndexToId` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	fn bump_offboard() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66402`
		//  Estimated: `69046`
		// Minimum execution time: 118_621_000 picoseconds.
		Weight::from_parts(120_321_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::Members` (r:1 w:1)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Params` (r:1 w:0)
	/// Proof: `AmbassadorCore::Params` (`max_values`: Some(1), `max_size`: Some(368), added: 863, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::MemberCount` (r:1 w:1)
	/// Proof: `AmbassadorCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IdToIndex` (r:1 w:1)
	/// Proof: `AmbassadorCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::MemberEvidence` (r:1 w:1)
	/// Proof: `AmbassadorCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IndexToId` (r:0 w:1)
	/// Proof: `AmbassadorCollective::IndexToId` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	fn bump_demote() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66512`
		//  Estimated: `69046`
		// Minimum execution time: 122_370_000 picoseconds.
		Weight::from_parts(123_591_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `AmbassadorCollective::Members` (r:1 w:0)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	fn set_active() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `360`
		//  Estimated: `3514`
		// Minimum execution time: 19_210_000 picoseconds.
		Weight::from_parts(19_810_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::Members` (r:1 w:1)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::MemberCount` (r:1 w:1)
	/// Proof: `AmbassadorCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IndexToId` (r:0 w:1)
	/// Proof: `AmbassadorCollective::IndexToId` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IdToIndex` (r:0 w:1)
	/// Proof: `AmbassadorCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	fn induct() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `118`
		//  Estimated: `3514`
		// Minimum execution time: 27_870_000 picoseconds.
		Weight::from_parts(28_080_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `AmbassadorCollective::Members` (r:1 w:1)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Params` (r:1 w:0)
	/// Proof: `AmbassadorCore::Params` (`max_values`: Some(1), `max_size`: Some(368), added: 863, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::MemberCount` (r:1 w:1)
	/// Proof: `AmbassadorCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::MemberEvidence` (r:1 w:1)
	/// Proof: `AmbassadorCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IndexToId` (r:0 w:1)
	/// Proof: `AmbassadorCollective::IndexToId` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IdToIndex` (r:0 w:1)
	/// Proof: `AmbassadorCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	fn promote() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `66055`
		//  Estimated: `69046`
		// Minimum execution time: 114_711_000 picoseconds.
		Weight::from_parts(115_631_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `AmbassadorCollective::Members` (r:1 w:1)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::MemberCount` (r:9 w:9)
	/// Proof: `AmbassadorCollective::MemberCount` (`max_values`: None, `max_size`: Some(14), added: 2489, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::MemberEvidence` (r:1 w:1)
	/// Proof: `AmbassadorCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IndexToId` (r:0 w:9)
	/// Proof: `AmbassadorCollective::IndexToId` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::IdToIndex` (r:0 w:9)
	/// Proof: `AmbassadorCollective::IdToIndex` (`max_values`: None, `max_size`: Some(54), added: 2529, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 9]`.
	/// The range of component `r` is `[1, 9]`.
	fn promote_fast(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `65968`
		//  Estimated: `69046 + r * (2489 ±0)`
		// Minimum execution time: 108_991_000 picoseconds.
		Weight::from_parts(95_565_683, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			// Standard Error: 12_243
			.saturating_add(Weight::from_parts(16_291_417, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(r.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(r.into())))
			.saturating_add(Weight::from_parts(0, 2489).saturating_mul(r.into()))
	}
	/// Storage: `AmbassadorCollective::Members` (r:1 w:0)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::MemberEvidence` (r:0 w:1)
	/// Proof: `AmbassadorCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn offboard() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `265`
		//  Estimated: `3514`
		// Minimum execution time: 19_880_000 picoseconds.
		Weight::from_parts(20_340_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCollective::Members` (r:1 w:0)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	fn import() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `285`
		//  Estimated: `3514`
		// Minimum execution time: 18_020_000 picoseconds.
		Weight::from_parts(18_410_000, 0)
			.saturating_add(Weight::from_parts(0, 3514))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `AmbassadorCollective::Members` (r:1 w:0)
	/// Proof: `AmbassadorCollective::Members` (`max_values`: None, `max_size`: Some(42), added: 2517, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::Member` (r:1 w:1)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::MemberEvidence` (r:1 w:1)
	/// Proof: `AmbassadorCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn approve() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `65967`
		//  Estimated: `69046`
		// Minimum execution time: 95_150_000 picoseconds.
		Weight::from_parts(96_150_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `AmbassadorCore::Member` (r:1 w:0)
	/// Proof: `AmbassadorCore::Member` (`max_values`: None, `max_size`: Some(49), added: 2524, mode: `MaxEncodedLen`)
	/// Storage: `AmbassadorCore::MemberEvidence` (r:1 w:1)
	/// Proof: `AmbassadorCore::MemberEvidence` (`max_values`: None, `max_size`: Some(65581), added: 68056, mode: `MaxEncodedLen`)
	fn submit_evidence() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `151`
		//  Estimated: `69046`
		// Minimum execution time: 81_530_000 picoseconds.
		Weight::from_parts(82_231_000, 0)
			.saturating_add(Weight::from_parts(0, 69046))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
