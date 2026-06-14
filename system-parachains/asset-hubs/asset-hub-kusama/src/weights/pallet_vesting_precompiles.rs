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

//! Weights for `pallet_vesting_precompiles` on Asset Hub Kusama.
//!
//! NOTE: These weights were not benchmarked on Asset Hub Kusama; they are
//! reused from the Asset Hub Polkadot benchmark run below as a stop-gap
//! until a Kusama-specific benchmark run is available.
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 54.0.0
//! DATE: 2026-05-05, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `bm1-3`, CPU: `Intel(R) Xeon(R) E-2388G CPU @ 3.20GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// frame-omni-bencher
// v1
// benchmark
// pallet
// --extrinsic=*
// --runtime=target/production/wbuild/asset-hub-polkadot-runtime/asset_hub_polkadot_runtime.wasm
// --pallet=pallet_vesting_precompiles
// --header=/opt/actions-runner/_work/runtimes/runtimes/.github/scripts/cmd/file_header.txt
// --output=./system-parachains/asset-hubs/asset-hub-polkadot/src/weights
// --wasm-execution=compiled
// --steps=50
// --repeat=20
// --heap-pages=4096
// --min-duration
// 1
// --quiet

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_vesting_precompiles`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_vesting_precompiles::weights::WeightInfo for WeightInfo<T> {
	/// Storage: `Vesting::Vesting` (r:1 w:0)
	/// Proof: `Vesting::Vesting` (`max_values`: None, `max_size`: Some(1057), added: 3532, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::ValidationData` (r:1 w:0)
	/// Proof: `ParachainSystem::ValidationData` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::LastRelayChainBlockNumber` (r:1 w:0)
	/// Proof: `ParachainSystem::LastRelayChainBlockNumber` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn vesting_balance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1554`
		//  Estimated: `4522`
		// Minimum execution time: 14_838_000 picoseconds.
		Weight::from_parts(15_740_000, 0)
			.saturating_add(Weight::from_parts(0, 4522))
			.saturating_add(T::DbWeight::get().reads(3))
	}
	/// Storage: `Revive::OriginalAccount` (r:1 w:0)
	/// Proof: `Revive::OriginalAccount` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Vesting::Vesting` (r:1 w:0)
	/// Proof: `Vesting::Vesting` (`max_values`: None, `max_size`: Some(1057), added: 3532, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::ValidationData` (r:1 w:0)
	/// Proof: `ParachainSystem::ValidationData` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `ParachainSystem::LastRelayChainBlockNumber` (r:1 w:0)
	/// Proof: `ParachainSystem::LastRelayChainBlockNumber` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:0)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn vesting_balance_of() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `2352`
		//  Estimated: `4522`
		// Minimum execution time: 20_172_000 picoseconds.
		Weight::from_parts(21_237_000, 0)
			.saturating_add(Weight::from_parts(0, 4522))
			.saturating_add(T::DbWeight::get().reads(5))
	}
}
