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

//! Emulated integration test helpers that have no upstream equivalent.
//!
//! This crate must never re-export items from `emulated-integration-tests-common`: test crates
//! import those straight from upstream. Everything kept here is expected to be upstreamed
//! eventually and then dropped.

// The re-exports below exist only so that `test_accumulated_funds_are_burnt_on_asset_hub!` can
// name them through `$crate` at its expansion sites.
pub use paste;

// Substrate
pub use frame_support;
pub use frame_system;
pub use pallet_accumulate_and_forward;
pub use pallet_balances;
pub use pallet_collator_selection;
pub use pallet_message_queue;

// Polkadot
pub use pallet_xcm;
pub use xcm::prelude::{Assets, Junction, Location, Weight, WeightLimit};

// Cumulus
pub use xcm_emulator::Chain;

pub mod common;
