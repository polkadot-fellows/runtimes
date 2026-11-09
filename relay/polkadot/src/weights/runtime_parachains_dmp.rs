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

//! Weights for `runtime_parachains::dmp`
//!
//! TODO 2609-bench: not generated for this runtime. Copied from the default `SubstrateWeight` in
//! `polkadot-runtime-parachains` (polkadot-sdk stable2609-1, polkadot-sdk#11909). Regenerate with
//! the bench bot.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `runtime_parachains::dmp`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> runtime_parachains::dmp::WeightInfo for WeightInfo<T> {
	/// Storage: `Dmp::DownwardMessageQueueLazyDelete` (r:2 w:1)
	/// Proof: `Dmp::DownwardMessageQueueLazyDelete` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueuePages` (r:0 w:3)
	/// Proof: `Dmp::DownwardMessageQueuePages` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn lazy_delete_some() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `206`
		//  Estimated: `6146`
		// Minimum execution time: 8_000_000 picoseconds.
		Weight::from_parts(10_000_000, 6146)
			.saturating_add(T::DbWeight::get().reads(2_u64))
			.saturating_add(T::DbWeight::get().writes(4_u64))
	}
	/// Storage: UNKNOWN KEY `0x63f78c98723ddc9073523ef3beefda0ca95dac46c07a40d91506e7637ec4ba57` (r:1 w:0)
	/// Proof: UNKNOWN KEY `0x63f78c98723ddc9073523ef3beefda0ca95dac46c07a40d91506e7637ec4ba57` (r:1 w:0)
	fn migrate_v0_to_v1_step_base() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `175`
		//  Estimated: `3640`
		// Minimum execution time: 2_000_000 picoseconds.
		Weight::from_parts(3_000_000, 3640)
			.saturating_add(T::DbWeight::get().reads(1_u64))
	}
	/// Storage: UNKNOWN KEY `0x63f78c98723ddc9073523ef3beefda0ca95dac46c07a40d91506e7637ec4ba57` (r:2 w:1)
	/// Proof: UNKNOWN KEY `0x63f78c98723ddc9073523ef3beefda0ca95dac46c07a40d91506e7637ec4ba57` (r:2 w:1)
	/// Storage: `Dmp::DownwardMessageQueueMeta` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueueMeta` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueuePages` (r:0 w:1)
	/// Proof: `Dmp::DownwardMessageQueuePages` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn migrate_v0_to_v1_step_iter() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1048801`
		//  Estimated: `1054741`
		// Minimum execution time: 413_000_000 picoseconds.
		Weight::from_parts(471_000_000, 1054741)
			.saturating_add(T::DbWeight::get().reads(3_u64))
			.saturating_add(T::DbWeight::get().writes(3_u64))
	}
	/// Storage: `Dmp::DownwardMessageQueuePages` (r:0 w:1)
	/// Proof: `Dmp::DownwardMessageQueuePages` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn migrate_v0_to_v1_step_msg() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 87_000_000 picoseconds.
		Weight::from_parts(98_000_000, 0)
			.saturating_add(T::DbWeight::get().writes(1_u64))
	}
}

// For backwards compatibility and tests.
