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

//! Placeholder weights for `pallet_asset_conversion`, pending a real benchmark run.

#![allow(missing_docs)]

use core::marker::PhantomData;
use frame_support::{traits::Get, weights::Weight};

/// Weight functions for `pallet_asset_conversion`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_asset_conversion::WeightInfo for WeightInfo<T> {
	// TODO: placeholder until benchmarked
	fn create_pool() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn add_liquidity() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn remove_liquidity() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn swap_exact_tokens_for_tokens(_n: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn swap_tokens_for_exact_tokens(_n: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn touch(_n: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn get_reserves() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
}
