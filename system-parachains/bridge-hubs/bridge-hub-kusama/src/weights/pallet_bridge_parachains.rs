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

//! Autogenerated weights for `pallet_bridge_parachains`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-08-14, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./bridge-hub-kusama-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot-parachain
// benchmark
// pallet
// --chain=./bridge-hub-kusama-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_bridge_parachains
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./bridge-hub-kusama-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_bridge_parachains`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_bridge_parachains::WeightInfo for WeightInfo<T> {
	/// Storage: `BridgePolkadotParachains::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotParachains::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotGrandpa::ImportedHeaders` (`max_values`: Some(1200), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ParasInfo` (r:1 w:1)
	/// Proof: `BridgePolkadotParachains::ParasInfo` (`max_values`: Some(1), `max_size`: Some(60), added: 555, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ImportedParaHashes` (r:1 w:1)
	/// Proof: `BridgePolkadotParachains::ImportedParaHashes` (`max_values`: Some(600), `max_size`: Some(64), added: 1549, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotGrandpa::FreeHeadersRemaining` (r:1 w:1)
	/// Proof: `BridgePolkadotGrandpa::FreeHeadersRemaining` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ImportedParaHeads` (r:0 w:1)
	/// Proof: `BridgePolkadotParachains::ImportedParaHeads` (`max_values`: Some(600), `max_size`: Some(196), added: 1681, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[1, 2]`.
	fn submit_parachain_heads_with_n_parachains(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `317`
		//  Estimated: `2543`
		// Minimum execution time: 42_430_000 picoseconds.
		Weight::from_parts(43_429_246, 0)
			.saturating_add(Weight::from_parts(0, 2543))
			// Standard Error: 49_229
			.saturating_add(Weight::from_parts(7_326, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `BridgePolkadotParachains::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotParachains::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotGrandpa::ImportedHeaders` (`max_values`: Some(1200), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ParasInfo` (r:1 w:1)
	/// Proof: `BridgePolkadotParachains::ParasInfo` (`max_values`: Some(1), `max_size`: Some(60), added: 555, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ImportedParaHashes` (r:1 w:1)
	/// Proof: `BridgePolkadotParachains::ImportedParaHashes` (`max_values`: Some(600), `max_size`: Some(64), added: 1549, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotGrandpa::FreeHeadersRemaining` (r:1 w:1)
	/// Proof: `BridgePolkadotGrandpa::FreeHeadersRemaining` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ImportedParaHeads` (r:0 w:1)
	/// Proof: `BridgePolkadotParachains::ImportedParaHeads` (`max_values`: Some(600), `max_size`: Some(196), added: 1681, mode: `MaxEncodedLen`)
	fn submit_parachain_heads_with_1kb_proof() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `317`
		//  Estimated: `2543`
		// Minimum execution time: 43_951_000 picoseconds.
		Weight::from_parts(44_350_000, 0)
			.saturating_add(Weight::from_parts(0, 2543))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `BridgePolkadotParachains::PalletOperatingMode` (r:1 w:0)
	/// Proof: `BridgePolkadotParachains::PalletOperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotGrandpa::ImportedHeaders` (r:1 w:0)
	/// Proof: `BridgePolkadotGrandpa::ImportedHeaders` (`max_values`: Some(1200), `max_size`: Some(68), added: 1553, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ParasInfo` (r:1 w:1)
	/// Proof: `BridgePolkadotParachains::ParasInfo` (`max_values`: Some(1), `max_size`: Some(60), added: 555, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ImportedParaHashes` (r:1 w:1)
	/// Proof: `BridgePolkadotParachains::ImportedParaHashes` (`max_values`: Some(600), `max_size`: Some(64), added: 1549, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotGrandpa::FreeHeadersRemaining` (r:1 w:1)
	/// Proof: `BridgePolkadotGrandpa::FreeHeadersRemaining` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `BridgePolkadotParachains::ImportedParaHeads` (r:0 w:1)
	/// Proof: `BridgePolkadotParachains::ImportedParaHeads` (`max_values`: Some(600), `max_size`: Some(196), added: 1681, mode: `MaxEncodedLen`)
	fn submit_parachain_heads_with_16kb_proof() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `317`
		//  Estimated: `2543`
		// Minimum execution time: 74_271_000 picoseconds.
		Weight::from_parts(74_950_000, 0)
			.saturating_add(Weight::from_parts(0, 2543))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
}
