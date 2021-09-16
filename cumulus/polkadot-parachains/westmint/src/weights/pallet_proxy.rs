//! Autogenerated weights for pallet_proxy
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-05-31, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("statemint-dev"), DB CACHE: 128

// Executed Command:
// ./target/release/statemint
// benchmark
// --chain=statemint-dev
// --execution=wasm
// --wasm-execution=compiled
// --pallet=pallet_proxy
// --extrinsic=*
// --steps=50
// --repeat=20
// --raw
// --output=./runtime/statemint/src/weights/

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_proxy.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_proxy::WeightInfo for WeightInfo<T> {
	fn proxy(p: u32) -> Weight {
		(27_585_000 as Weight)
			// Standard Error: 1_000
			.saturating_add((203_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
	}
	fn proxy_announced(a: u32, p: u32) -> Weight {
		(61_093_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((680_000 as Weight).saturating_mul(a as Weight))
			// Standard Error: 2_000
			.saturating_add((201_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn remove_announcement(a: u32, p: u32) -> Weight {
		(39_494_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((686_000 as Weight).saturating_mul(a as Weight))
			// Standard Error: 2_000
			.saturating_add((1_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn reject_announcement(a: u32, p: u32) -> Weight {
		(39_817_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((685_000 as Weight).saturating_mul(a as Weight))
			// Standard Error: 2_000
			.saturating_add((1_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn announce(a: u32, p: u32) -> Weight {
		(54_835_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((684_000 as Weight).saturating_mul(a as Weight))
			// Standard Error: 2_000
			.saturating_add((205_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	fn add_proxy(p: u32) -> Weight {
		(37_625_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((300_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn remove_proxy(p: u32) -> Weight {
		(36_945_000 as Weight)
			// Standard Error: 3_000
			.saturating_add((325_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn remove_proxies(p: u32) -> Weight {
		(35_128_000 as Weight)
			// Standard Error: 1_000
			.saturating_add((209_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn anonymous(p: u32) -> Weight {
		(51_624_000 as Weight)
			// Standard Error: 1_000
			.saturating_add((41_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	fn kill_anonymous(p: u32) -> Weight {
		(37_469_000 as Weight)
			// Standard Error: 1_000
			.saturating_add((204_000 as Weight).saturating_mul(p as Weight))
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}
