// Copyright 2017-2022 Parity Technologies (UK) Ltd.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.
//! Autogenerated weights for `runtime_common::claims`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-01-11, STEPS: `50`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `bm4`, CPU: `Intel(R) Core(TM) i7-7700K CPU @ 4.20GHz`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("rococo-dev"), DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=rococo-dev
// --steps=50
// --repeat=20
// --pallet=runtime_common::claims
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --header=./file_header.txt
// --output=./runtime/rococo/src/weights/runtime_common_claims.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `runtime_common::claims`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> runtime_common::claims::WeightInfo for WeightInfo<T> {
	// Storage: Claims Claims (r:1 w:1)
	// Storage: Claims Signing (r:1 w:1)
	// Storage: Claims Total (r:1 w:1)
	// Storage: Claims Vesting (r:1 w:1)
	// Storage: Vesting Vesting (r:1 w:1)
	// Storage: System Account (r:1 w:0)
	// Storage: Balances Locks (r:1 w:1)
	fn claim() -> Weight {
		// Minimum execution time: 142_660 nanoseconds.
		Weight::from_ref_time(144_589_000)
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	// Storage: Claims Total (r:1 w:1)
	// Storage: Claims Vesting (r:0 w:1)
	// Storage: Claims Claims (r:0 w:1)
	// Storage: Claims Signing (r:0 w:1)
	fn mint_claim() -> Weight {
		// Minimum execution time: 12_173 nanoseconds.
		Weight::from_ref_time(12_656_000)
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	// Storage: Claims Claims (r:1 w:1)
	// Storage: Claims Signing (r:1 w:1)
	// Storage: Claims Total (r:1 w:1)
	// Storage: Claims Vesting (r:1 w:1)
	// Storage: Vesting Vesting (r:1 w:1)
	// Storage: System Account (r:1 w:0)
	// Storage: Balances Locks (r:1 w:1)
	fn claim_attest() -> Weight {
		// Minimum execution time: 146_534 nanoseconds.
		Weight::from_ref_time(148_392_000)
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	// Storage: Claims Preclaims (r:1 w:1)
	// Storage: Claims Signing (r:1 w:1)
	// Storage: Claims Claims (r:1 w:1)
	// Storage: Claims Total (r:1 w:1)
	// Storage: Claims Vesting (r:1 w:1)
	// Storage: Vesting Vesting (r:1 w:1)
	// Storage: System Account (r:1 w:0)
	// Storage: Balances Locks (r:1 w:1)
	fn attest() -> Weight {
		// Minimum execution time: 67_366 nanoseconds.
		Weight::from_ref_time(69_286_000)
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	// Storage: Claims Claims (r:1 w:2)
	// Storage: Claims Vesting (r:1 w:2)
	// Storage: Claims Signing (r:1 w:2)
	// Storage: Claims Preclaims (r:1 w:1)
	fn move_claim() -> Weight {
		// Minimum execution time: 22_528 nanoseconds.
		Weight::from_ref_time(22_908_000)
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(7))
	}
}
