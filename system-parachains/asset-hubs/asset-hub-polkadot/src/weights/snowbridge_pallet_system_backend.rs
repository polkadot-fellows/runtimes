// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `snowbridge_pallet_system_frontend::BackendWeightInfo`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> snowbridge_pallet_system_frontend::BackendWeightInfo for WeightInfo<T> {
	/// Copy the weight generated for `fn register_token() -> Weight` from ../../../../bridge-hubs/bridge-hub-polkadot/src/weights/snowbridge_pallet_system_v2.rs
	fn transact_register_token() -> Weight {
		Weight::from_parts(54_520_000, 0)
			.saturating_add(Weight::from_parts(0, 4115))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	/// Copy the weight generated for `fn add_tip() -> Weight` from ../../../../bridge-hubs/bridge-hub-polkadot/src/weights/snowbridge_pallet_system_v2.rs
	fn transact_add_tip() -> Weight {
		Weight::from_parts(13_270_000, 0)
			.saturating_add(Weight::from_parts(0, 3505))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}

	/// Copy the weight generated for `fn do_process_message() -> Weight` from ../../../../bridge-hubs/bridge-hub-polkadot/src/weights/snowbridge_pallet_outbound_queue_v2.rs
	fn do_process_message() -> Weight {
		Weight::from_parts(32_300_000, 0)
			.saturating_add(Weight::from_parts(0, 1527))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(4))
	}

	/// Copy the weight generated for `fn commit_single() -> Weight` from ../../../../bridge-hubs/bridge-hub-polkadot/src/weights/snowbridge_pallet_outbound_queue_v2.rs
	fn commit_single() -> Weight {
		Weight::from_parts(13_500_000, 0)
			.saturating_add(Weight::from_parts(0, 1620))
			.saturating_add(T::DbWeight::get().reads(1))
	}

	/// Copy the weight generated for `fn submit_delivery_receipt() -> Weight` from ../../../../bridge-hubs/bridge-hub-polkadot/src/weights/snowbridge_pallet_outbound_queue_v2.rs
	fn submit_delivery_receipt() -> Weight {
		Weight::from_parts(96_539_000, 0)
			.saturating_add(Weight::from_parts(0, 3762))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
