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

//! Autogenerated weights for `pallet_offences`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-01-05, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./kusama-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=./kusama-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_offences
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

/// Weight functions for `pallet_offences`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_offences::WeightInfo for WeightInfo<T> {
	/// Storage: `Offences::ConcurrentReportsIndex` (r:1 w:1)
	/// Proof: `Offences::ConcurrentReportsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Offences::Reports` (r:1 w:1)
	/// Proof: `Offences::Reports` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::SlashRewardFraction` (r:1 w:0)
	/// Proof: `Staking::SlashRewardFraction` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Staking::ActiveEra` (r:1 w:0)
	/// Proof: `Staking::ActiveEra` (`max_values`: Some(1), `max_size`: Some(13), added: 508, mode: `MaxEncodedLen`)
	/// Storage: `Staking::ErasStartSessionIndex` (r:1 w:0)
	/// Proof: `Staking::ErasStartSessionIndex` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `Staking::Invulnerables` (r:1 w:0)
	/// Proof: `Staking::Invulnerables` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::ValidatorSlashInEra` (r:1 w:1)
	/// Proof: `Staking::ValidatorSlashInEra` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `Staking::SlashingSpans` (r:25 w:25)
	/// Proof: `Staking::SlashingSpans` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::SpanSlash` (r:25 w:25)
	/// Proof: `Staking::SpanSlash` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `Staking::DisabledValidators` (r:1 w:1)
	/// Proof: `Staking::DisabledValidators` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Session::Validators` (r:1 w:0)
	/// Proof: `Session::Validators` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::NominatorSlashInEra` (r:24 w:24)
	/// Proof: `Staking::NominatorSlashInEra` (`max_values`: None, `max_size`: Some(68), added: 2543, mode: `MaxEncodedLen`)
	/// Storage: `Staking::UnappliedSlashes` (r:1 w:1)
	/// Proof: `Staking::UnappliedSlashes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `n` is `[0, 24]`.
	fn report_offence_grandpa(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1133`
		//  Estimated: `4586 + n * (2551 ±0)`
		// Minimum execution time: 79_591_000 picoseconds.
		Weight::from_parts(83_268_326, 0)
			.saturating_add(Weight::from_parts(0, 4586))
			// Standard Error: 9_989
			.saturating_add(Weight::from_parts(13_259_367, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(7))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2551).saturating_mul(n.into()))
	}
	/// Storage: `Offences::ConcurrentReportsIndex` (r:1 w:1)
	/// Proof: `Offences::ConcurrentReportsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Offences::Reports` (r:1 w:1)
	/// Proof: `Offences::Reports` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::SlashRewardFraction` (r:1 w:0)
	/// Proof: `Staking::SlashRewardFraction` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `Staking::ActiveEra` (r:1 w:0)
	/// Proof: `Staking::ActiveEra` (`max_values`: Some(1), `max_size`: Some(13), added: 508, mode: `MaxEncodedLen`)
	/// Storage: `Staking::ErasStartSessionIndex` (r:1 w:0)
	/// Proof: `Staking::ErasStartSessionIndex` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `Staking::Invulnerables` (r:1 w:0)
	/// Proof: `Staking::Invulnerables` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::ValidatorSlashInEra` (r:1 w:1)
	/// Proof: `Staking::ValidatorSlashInEra` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `Staking::SlashingSpans` (r:25 w:25)
	/// Proof: `Staking::SlashingSpans` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::SpanSlash` (r:25 w:25)
	/// Proof: `Staking::SpanSlash` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `Staking::DisabledValidators` (r:1 w:1)
	/// Proof: `Staking::DisabledValidators` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Session::Validators` (r:1 w:0)
	/// Proof: `Session::Validators` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Staking::NominatorSlashInEra` (r:24 w:24)
	/// Proof: `Staking::NominatorSlashInEra` (`max_values`: None, `max_size`: Some(68), added: 2543, mode: `MaxEncodedLen`)
	/// Storage: `Staking::UnappliedSlashes` (r:1 w:1)
	/// Proof: `Staking::UnappliedSlashes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `n` is `[0, 24]`.
	fn report_offence_babe(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1133`
		//  Estimated: `4586 + n * (2551 ±0)`
		// Minimum execution time: 78_390_000 picoseconds.
		Weight::from_parts(81_263_126, 0)
			.saturating_add(Weight::from_parts(0, 4586))
			// Standard Error: 8_555
			.saturating_add(Weight::from_parts(13_366_116, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(7))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2551).saturating_mul(n.into()))
	}
}
