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

//! Autogenerated weights for `pallet_xcm_bridge_hub_router`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-08-14, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./asset-hub-kusama-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot-parachain
// benchmark
// pallet
// --chain=./asset-hub-kusama-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_xcm_bridge_hub_router
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./asset-hub-kusama-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_xcm_bridge_hub_router`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_xcm_bridge_hub_router::WeightInfo for WeightInfo<T> {
	/// Storage: `XcmpQueue::InboundXcmpSuspended` (r:1 w:0)
	/// Proof: `XcmpQueue::InboundXcmpSuspended` (`max_values`: Some(1), `max_size`: Some(4002), added: 4497, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:0)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(1282), added: 1777, mode: `MaxEncodedLen`)
	/// Storage: `ToPolkadotXcmRouter::Bridge` (r:1 w:1)
	/// Proof: `ToPolkadotXcmRouter::Bridge` (`max_values`: Some(1), `max_size`: Some(17), added: 512, mode: `MaxEncodedLen`)
	fn on_initialize_when_non_congested() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `159`
		//  Estimated: `5487`
		// Minimum execution time: 11_410_000 picoseconds.
		Weight::from_parts(11_870_000, 0)
			.saturating_add(Weight::from_parts(0, 5487))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `XcmpQueue::InboundXcmpSuspended` (r:1 w:0)
	/// Proof: `XcmpQueue::InboundXcmpSuspended` (`max_values`: Some(1), `max_size`: Some(4002), added: 4497, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:0)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(1282), added: 1777, mode: `MaxEncodedLen`)
	fn on_initialize_when_congested() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `111`
		//  Estimated: `5487`
		// Minimum execution time: 5_300_000 picoseconds.
		Weight::from_parts(5_510_000, 0)
			.saturating_add(Weight::from_parts(0, 5487))
			.saturating_add(T::DbWeight::get().reads(2))
	}
	/// Storage: `ToPolkadotXcmRouter::Bridge` (r:1 w:1)
	/// Proof: `ToPolkadotXcmRouter::Bridge` (`max_values`: Some(1), `max_size`: Some(17), added: 512, mode: `MaxEncodedLen`)
	fn report_bridge_status() -> Weight {
		// TODO: fresh weights
		// Proof Size summary in bytes:
		//  Measured:  `150`
		//  Estimated: `1502`
		// Minimum execution time: 12_394_000 picoseconds.
		Weight::from_parts(12_883_000, 0)
			.saturating_add(Weight::from_parts(0, 1502))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
