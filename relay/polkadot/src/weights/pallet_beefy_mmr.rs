#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_balances`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_beefy_mmr::WeightInfo for WeightInfo<T> {
	fn extract_validation_context() -> Weight {
	  Weight::from_parts(0, 0)
		.saturating_add(Weight::from_parts(0, 0))
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1))
	}

	fn read_peak() -> Weight {
	  Weight::from_parts(0, 0)
		.saturating_add(Weight::from_parts(0, 0))
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1))
	}

	fn n_items_proof_is_non_canonical(_: u32, ) -> Weight {
	  Weight::from_parts(0, 0)
		.saturating_add(Weight::from_parts(0, 0))
		.saturating_add(T::DbWeight::get().reads(1))
		.saturating_add(T::DbWeight::get().writes(1))
	}
}
