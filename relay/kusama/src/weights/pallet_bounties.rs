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
//! Autogenerated weights for `pallet_bounties`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2024-03-10, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./kusama-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=./kusama-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_bounties
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./kusama-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_bounties`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_bounties::WeightInfo for WeightInfo<T> {
	/// Storage: `Bounties::BountyCount` (r:1 w:1)
	/// Proof: `Bounties::BountyCount` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Bounties::BountyDescriptions` (r:0 w:1)
	/// Proof: `Bounties::BountyDescriptions` (`max_values`: None, `max_size`: Some(16400), added: 18875, mode: `MaxEncodedLen`)
	/// Storage: `Bounties::Bounties` (r:0 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// The range of component `d` is `[0, 16384]`.
	fn propose_bounty(d: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `210`
		//  Estimated: `3593`
		// Minimum execution time: 21_969_000 picoseconds.
		Weight::from_parts(23_348_914, 0)
			.saturating_add(Weight::from_parts(0, 3593))
			// Standard Error: 7
			.saturating_add(Weight::from_parts(678, 0).saturating_mul(d.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `Bounties::BountyApprovals` (r:1 w:1)
	/// Proof: `Bounties::BountyApprovals` (`max_values`: Some(1), `max_size`: Some(402), added: 897, mode: `MaxEncodedLen`)
	fn approve_bounty() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `302`
		//  Estimated: `3642`
		// Minimum execution time: 11_335_000 picoseconds.
		Weight::from_parts(11_904_000, 0)
			.saturating_add(Weight::from_parts(0, 3642))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	fn propose_curator() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `322`
		//  Estimated: `3642`
		// Minimum execution time: 10_862_000 picoseconds.
		Weight::from_parts(11_375_000, 0)
			.saturating_add(Weight::from_parts(0, 3642))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn unassign_curator() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `498`
		//  Estimated: `3642`
		// Minimum execution time: 37_076_000 picoseconds.
		Weight::from_parts(38_064_000, 0)
			.saturating_add(Weight::from_parts(0, 3642))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn accept_curator() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `494`
		//  Estimated: `3642`
		// Minimum execution time: 27_482_000 picoseconds.
		Weight::from_parts(28_170_000, 0)
			.saturating_add(Weight::from_parts(0, 3642))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `ChildBounties::ParentChildBounties` (r:1 w:0)
	/// Proof: `ChildBounties::ParentChildBounties` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	fn award_bounty() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `438`
		//  Estimated: `3642`
		// Minimum execution time: 17_172_000 picoseconds.
		Weight::from_parts(17_751_000, 0)
			.saturating_add(Weight::from_parts(0, 3642))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:3 w:3)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `ChildBounties::ChildrenCuratorFees` (r:1 w:1)
	/// Proof: `ChildBounties::ChildrenCuratorFees` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `Bounties::BountyDescriptions` (r:0 w:1)
	/// Proof: `Bounties::BountyDescriptions` (`max_values`: None, `max_size`: Some(16400), added: 18875, mode: `MaxEncodedLen`)
	fn claim_bounty() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `802`
		//  Estimated: `8799`
		// Minimum execution time: 101_888_000 picoseconds.
		Weight::from_parts(103_593_000, 0)
			.saturating_add(Weight::from_parts(0, 8799))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `ChildBounties::ParentChildBounties` (r:1 w:0)
	/// Proof: `ChildBounties::ParentChildBounties` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Bounties::BountyDescriptions` (r:0 w:1)
	/// Proof: `Bounties::BountyDescriptions` (`max_values`: None, `max_size`: Some(16400), added: 18875, mode: `MaxEncodedLen`)
	fn close_bounty_proposed() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `482`
		//  Estimated: `3642`
		// Minimum execution time: 39_111_000 picoseconds.
		Weight::from_parts(40_170_000, 0)
			.saturating_add(Weight::from_parts(0, 3642))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `ChildBounties::ParentChildBounties` (r:1 w:0)
	/// Proof: `ChildBounties::ParentChildBounties` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Bounties::BountyDescriptions` (r:0 w:1)
	/// Proof: `Bounties::BountyDescriptions` (`max_values`: None, `max_size`: Some(16400), added: 18875, mode: `MaxEncodedLen`)
	fn close_bounty_active() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `718`
		//  Estimated: `6196`
		// Minimum execution time: 68_360_000 picoseconds.
		Weight::from_parts(70_054_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Bounties::Bounties` (r:1 w:1)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	fn extend_bounty_expiry() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `358`
		//  Estimated: `3642`
		// Minimum execution time: 11_532_000 picoseconds.
		Weight::from_parts(11_858_000, 0)
			.saturating_add(Weight::from_parts(0, 3642))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Bounties::BountyApprovals` (r:1 w:1)
	/// Proof: `Bounties::BountyApprovals` (`max_values`: Some(1), `max_size`: Some(402), added: 897, mode: `MaxEncodedLen`)
	/// Storage: `Bounties::Bounties` (r:100 w:100)
	/// Proof: `Bounties::Bounties` (`max_values`: None, `max_size`: Some(177), added: 2652, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:200 w:200)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `b` is `[0, 100]`.
	fn spend_funds(b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + b * (297 ±0)`
		//  Estimated: `1887 + b * (5206 ±0)`
		// Minimum execution time: 2_929_000 picoseconds.
		Weight::from_parts(2_990_000, 0)
			.saturating_add(Weight::from_parts(0, 1887))
			// Standard Error: 9_763
			.saturating_add(Weight::from_parts(32_333_372, 0).saturating_mul(b.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(b.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(b.into())))
			.saturating_add(Weight::from_parts(0, 5206).saturating_mul(b.into()))
	}
}
