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

//! Placeholder weights for `pallet_assets` (pool instance), pending a real benchmark run.

#![allow(missing_docs)]

use core::marker::PhantomData;
use frame_support::{traits::Get, weights::Weight};

/// Weight functions for `pallet_assets` (pool instance).
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_assets::WeightInfo for WeightInfo<T> {
	// TODO: placeholder until benchmarked
	fn create() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn force_create() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn start_destroy() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn destroy_accounts(_c: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn destroy_approvals(_a: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn finish_destroy() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn mint() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn burn() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn transfer() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn transfer_keep_alive() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn force_transfer() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn freeze() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn thaw() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn freeze_asset() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn thaw_asset() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn transfer_ownership() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn set_team() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn set_reserves(_n: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn set_metadata(_n: u32, _s: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn clear_metadata() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn force_set_metadata(_n: u32, _s: u32) -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn force_clear_metadata() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn force_asset_status() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn approve_transfer() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn transfer_approved() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn cancel_approval() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn force_cancel_approval() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn set_min_balance() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn touch() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn touch_other() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn refund() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn refund_other() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn block() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn transfer_all() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn total_issuance() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn balance() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn allowance() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn migration_v2_foreign_asset_set_reserve_weight() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
	// TODO: placeholder until benchmarked
	fn get_metadata() -> Weight {
		Weight::from_parts(1_000_000_000, 10_000)
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(10))
	}
}
