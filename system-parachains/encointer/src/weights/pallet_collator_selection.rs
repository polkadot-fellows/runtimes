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

//! Autogenerated weights for `pallet_collator_selection`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 47.2.0
//! DATE: 2025-06-16, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --runtime=target/production/wbuild/encointer-kusama-runtime/encointer_kusama_runtime.compact.compressed.wasm
// --header=.github/scripts/cmd/file_header.txt
// --output=./system-parachains/encointer/src/weights/
// --all
// --quiet

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_collator_selection`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_collator_selection::WeightInfo for WeightInfo<T> {
	/// Storage: `Session::NextKeys` (r:20 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::Invulnerables` (r:0 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 20]`.
	fn set_invulnerables(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `197 + b * (79 ±0)`
		//  Estimated: `1188 + b * (2555 ±0)`
		// Minimum execution time: 15_780_000 picoseconds.
		Weight::from_parts(13_084_476, 0)
			.saturating_add(Weight::from_parts(0, 1188))
			// Standard Error: 9_548
			.saturating_add(Weight::from_parts(4_479_641, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 2555).saturating_mul(b.into()))
	}
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[1, 19]`.
	/// The range of component `c` is `[1, 99]`.
	fn add_invulnerable(b: u32, c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `862 + b * (32 ±0) + c * (53 ±0)`
		//  Estimated: `6287 + b * (37 ±0) + c * (53 ±0)`
		// Minimum execution time: 51_070_000 picoseconds.
		Weight::from_parts(50_917_275, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 6_106
			.saturating_add(Weight::from_parts(80_690, 0).saturating_mul(b.into()))
			// Standard Error: 1_157
			.saturating_add(Weight::from_parts(95_185, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 37).saturating_mul(b.into()))
			.saturating_add(Weight::from_parts(0, 53).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:1)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[5, 20]`.
	fn remove_invulnerable(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `186 + b * (32 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 15_300_000 picoseconds.
		Weight::from_parts(15_544_882, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 1_217
			.saturating_add(Weight::from_parts(67_671, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::DesiredCandidates` (r:0 w:1)
	/// Proof: `CollatorSelection::DesiredCandidates` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn set_desired_candidates() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 6_280_000 picoseconds.
		Weight::from_parts(6_521_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:100 w:100)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:100)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[0, 100]`.
	/// The range of component `k` is `[0, 100]`.
	fn set_candidacy_bond(c: u32, k: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + c * (182 ±0) + k * (115 ±0)`
		//  Estimated: `6287 + c * (901 ±29) + k * (901 ±29)`
		// Minimum execution time: 13_500_000 picoseconds.
		Weight::from_parts(13_591_000, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 199_538
			.saturating_add(Weight::from_parts(6_637_734, 0).saturating_mul(c.into()))
			// Standard Error: 199_538
			.saturating_add(Weight::from_parts(6_348_619, 0).saturating_mul(k.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(k.into())))
			.saturating_add(Weight::from_parts(0, 901).saturating_mul(c.into()))
			.saturating_add(Weight::from_parts(0, 901).saturating_mul(k.into()))
	}
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[3, 100]`.
	fn update_bond(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `386 + c * (49 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 35_830_000 picoseconds.
		Weight::from_parts(36_818_091, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 1_119
			.saturating_add(Weight::from_parts(79_015, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[1, 99]`.
	fn register_as_candidate(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `832 + c * (52 ±0)`
		//  Estimated: `6287 + c * (54 ±0)`
		// Minimum execution time: 47_550_000 picoseconds.
		Weight::from_parts(49_786_570, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 1_043
			.saturating_add(Weight::from_parts(102_911, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(Weight::from_parts(0, 54).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::CandidacyBond` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidacyBond` (`max_values`: Some(1), `max_size`: Some(16), added: 511, mode: `MaxEncodedLen`)
	/// Storage: `Session::NextKeys` (r:1 w:0)
	/// Proof: `Session::NextKeys` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:2)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[3, 100]`.
	fn take_candidate_slot(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `972 + c * (53 ±0)`
		//  Estimated: `6287 + c * (54 ±0)`
		// Minimum execution time: 66_330_000 picoseconds.
		Weight::from_parts(68_548_147, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 1_992
			.saturating_add(Weight::from_parts(110_502, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 54).saturating_mul(c.into()))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:1)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// The range of component `c` is `[3, 100]`.
	fn leave_intent(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `414 + c * (48 ±0)`
		//  Estimated: `6287`
		// Minimum execution time: 38_540_000 picoseconds.
		Weight::from_parts(39_826_932, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 1_166
			.saturating_add(Weight::from_parts(87_656, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:0 w:1)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	fn note_author() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `155`
		//  Estimated: `6196`
		// Minimum execution time: 50_180_000 picoseconds.
		Weight::from_parts(50_970_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `CollatorSelection::CandidateList` (r:1 w:0)
	/// Proof: `CollatorSelection::CandidateList` (`max_values`: Some(1), `max_size`: Some(4802), added: 5297, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::LastAuthoredBlock` (r:100 w:0)
	/// Proof: `CollatorSelection::LastAuthoredBlock` (`max_values`: None, `max_size`: Some(44), added: 2519, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::Invulnerables` (r:1 w:0)
	/// Proof: `CollatorSelection::Invulnerables` (`max_values`: Some(1), `max_size`: Some(641), added: 1136, mode: `MaxEncodedLen`)
	/// Storage: `CollatorSelection::DesiredCandidates` (r:1 w:0)
	/// Proof: `CollatorSelection::DesiredCandidates` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:97 w:97)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 100]`.
	/// The range of component `c` is `[1, 100]`.
	fn new_session(r: u32, c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2369 + c * (97 ±0) + r * (114 ±0)`
		//  Estimated: `6287 + c * (2519 ±0) + r * (2603 ±0)`
		// Minimum execution time: 35_321_000 picoseconds.
		Weight::from_parts(35_900_000, 0)
			.saturating_add(Weight::from_parts(0, 6287))
			// Standard Error: 349_107
			.saturating_add(Weight::from_parts(16_011_255, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(c.into())))
			.saturating_add(Weight::from_parts(0, 2519).saturating_mul(c.into()))
			.saturating_add(Weight::from_parts(0, 2603).saturating_mul(r.into()))
	}
}
