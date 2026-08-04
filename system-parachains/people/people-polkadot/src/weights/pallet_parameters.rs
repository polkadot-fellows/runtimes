// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

//! Conservative fallback weights for `pallet_parameters`.
//!
//! These must be regenerated on the reference benchmark hardware before production deployment.

use core::marker::PhantomData;
use frame_support::{traits::Get, weights::Weight};

/// Weight functions for `pallet_parameters`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_parameters::WeightInfo for WeightInfo<T> {
	fn set_parameter() -> Weight {
		Weight::from_parts(6_204_000, 14_787)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
