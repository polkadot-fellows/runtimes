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

//! Weights for `pallet_accumulate_and_forward`.
//!
//! PROVISIONAL: adapted from Coretime Polkadot's #1282 figures. Regenerate before release; they
//! only gate an `on_idle` hook.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_accumulate_and_forward`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_accumulate_and_forward::WeightInfo for WeightInfo<T> {
	/// Storage: `XcmPallet::ShouldRecordXcm` (r:1 w:0)
	/// Proof: `XcmPallet::ShouldRecordXcm` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `XcmPallet::SupportedVersion` (r:1 w:0)
	/// Proof: `XcmPallet::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DeliveryFeeFactor` (r:1 w:0)
	/// Proof: `Dmp::DeliveryFeeFactor` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueues` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueues` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `XcmPallet::AssetTraps` (r:1 w:0)
	/// Proof: `XcmPallet::AssetTraps` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn send_native() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `248`
		//  Estimated: `3791`
		// Minimum execution time: 47_732_000 picoseconds.
		Weight::from_parts(49_924_000, 0)
			.saturating_add(Weight::from_parts(0, 3791))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
