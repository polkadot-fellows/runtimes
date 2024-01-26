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

//! Autogenerated weights for `snowbridge_outbound_queue`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-10-20, STEPS: `2`, REPEAT: `1`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `192.168.1.13`, CPU: `<UNKNOWN>`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("bridge-hub-polkadot-dev"), DB CACHE: 1024

// Executed Command:
// ../target/release/polkadot-parachain
// benchmark
// pallet
// --chain=bridge-hub-polkadot-dev
// --pallet=snowbridge_outbound_queue
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --output
// ./system-parachains/bridge-hubs/bridge-hub-polkadot/src/weights/snowbridge_outbound_queue.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `snowbridge_outbound_queue`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> snowbridge_outbound_queue::WeightInfo for WeightInfo<T> {
    /// Storage: EthereumOutboundQueue MessageLeaves (r:1 w:1)
    /// Proof Skipped: EthereumOutboundQueue MessageLeaves (max_values: Some(1), max_size: None, mode: Measured)
    /// Storage: EthereumOutboundQueue PendingHighPriorityMessageCount (r:1 w:1)
    /// Proof: EthereumOutboundQueue PendingHighPriorityMessageCount (max_values: Some(1), max_size: Some(4), added: 499, mode: MaxEncodedLen)
    /// Storage: EthereumOutboundQueue Nonce (r:1 w:1)
    /// Proof: EthereumOutboundQueue Nonce (max_values: None, max_size: Some(20), added: 2495, mode: MaxEncodedLen)
    /// Storage: EthereumOutboundQueue Messages (r:1 w:1)
    /// Proof Skipped: EthereumOutboundQueue Messages (max_values: Some(1), max_size: None, mode: Measured)
    fn do_process_message() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `42`
        //  Estimated: `3485`
        // Minimum execution time: 39_000_000 picoseconds.
        Weight::from_parts(39_000_000, 3485)
            .saturating_add(T::DbWeight::get().reads(4_u64))
            .saturating_add(T::DbWeight::get().writes(4_u64))
    }
    /// Storage: EthereumOutboundQueue MessageLeaves (r:1 w:0)
    /// Proof Skipped: EthereumOutboundQueue MessageLeaves (max_values: Some(1), max_size: None, mode: Measured)
    /// Storage: System Digest (r:1 w:1)
    /// Proof Skipped: System Digest (max_values: Some(1), max_size: None, mode: Measured)
    fn commit() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `1094`
        //  Estimated: `2579`
        // Minimum execution time: 28_000_000 picoseconds.
        Weight::from_parts(28_000_000, 2579)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }

    fn commit_single() -> Weight {
        // Proof Size summary in bytes:
        //  Measured:  `1094`
        //  Estimated: `2579`
        // Minimum execution time: 9_000_000 picoseconds.
        Weight::from_parts(9_000_000, 1586)
            .saturating_add(T::DbWeight::get().reads(2_u64))
            .saturating_add(T::DbWeight::get().writes(1_u64))
    }
}
