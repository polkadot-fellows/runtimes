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

//! Weights for `pallet_kill_switch`.
//!
//! Placeholders until the pallet is benchmarked in a runtime. Each runtime that uses the pallet
//! has to generate its own.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::{Weight, constants::RocksDbWeight}};
use core::marker::PhantomData;

pub trait WeightInfo {
	fn pause() -> Weight;
	fn extend() -> Weight;
	fn unpause(d: u32, ) -> Weight;
	fn force_extend() -> Weight;
	fn force_pause() -> Weight;
	fn expire() -> Weight;
	fn release_deposit() -> Weight;
	fn slash_deposit() -> Weight;
	fn on_poll_base() -> Weight;
	fn on_poll_step() -> Weight;
	fn on_poll_expire() -> Weight;
}

impl WeightInfo for () {
	fn pause() -> Weight {
		Weight::from_parts(60_000_000, 6_000)
			.saturating_add(RocksDbWeight::get().reads(8_u64))
			.saturating_add(RocksDbWeight::get().writes(8_u64))
	}
	fn extend() -> Weight {
		Weight::from_parts(60_000_000, 6_000)
			.saturating_add(RocksDbWeight::get().reads(6_u64))
			.saturating_add(RocksDbWeight::get().writes(7_u64))
	}
	fn unpause(d: u32, ) -> Weight {
		Weight::from_parts(40_000_000, 4_000)
			.saturating_add(Weight::from_parts(40_000_000, 3_000).saturating_mul(d.into()))
			.saturating_add(RocksDbWeight::get().reads(5_u64))
			.saturating_add(RocksDbWeight::get().reads((3_u64).saturating_mul(d.into())))
			.saturating_add(RocksDbWeight::get().writes(6_u64))
			.saturating_add(RocksDbWeight::get().writes((3_u64).saturating_mul(d.into())))
	}
	fn force_extend() -> Weight {
		Weight::from_parts(30_000_000, 4_000)
			.saturating_add(RocksDbWeight::get().reads(3_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
	fn force_pause() -> Weight {
		Weight::from_parts(30_000_000, 4_000)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
	fn expire() -> Weight {
		Weight::from_parts(30_000_000, 4_000)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
	fn release_deposit() -> Weight {
		Weight::from_parts(50_000_000, 4_000)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(3_u64))
	}
	fn slash_deposit() -> Weight {
		Weight::from_parts(60_000_000, 4_000)
			.saturating_add(RocksDbWeight::get().reads(5_u64))
			.saturating_add(RocksDbWeight::get().writes(4_u64))
	}
	fn on_poll_base() -> Weight {
		Weight::from_parts(5_000_000, 1_500)
			.saturating_add(RocksDbWeight::get().reads(2_u64))
			.saturating_add(RocksDbWeight::get().writes(1_u64))
	}
	fn on_poll_step() -> Weight {
		Weight::from_parts(5_000_000, 3_000)
			.saturating_add(RocksDbWeight::get().reads(1_u64))
	}
	fn on_poll_expire() -> Weight {
		Weight::from_parts(30_000_000, 4_000)
			.saturating_add(RocksDbWeight::get().reads(4_u64))
			.saturating_add(RocksDbWeight::get().writes(5_u64))
	}
}
