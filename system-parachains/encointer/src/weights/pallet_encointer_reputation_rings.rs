
//! Weights for `pallet_encointer_reputation_rings`
//!
//! Note: These weights were benchmarked on the encointer-node-notee solo chain
//! and should be re-benchmarked for the parachain runtime.
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 53.0.0
//! DATE: 2026-02-14, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `17e996ac557c`, CPU: `12th Gen Intel(R) Core(TM) i7-1260P`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("dev")`, DB CACHE: 1024

// Executed Command:
// target/release/encointer-node-notee
// benchmark
// pallet
// --chain=dev
// --steps=50
// --repeat=20
// --pallet=pallet_encointer_reputation_rings
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=runtime/src/weights/pallet_encointer_reputation_rings.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_encointer_reputation_rings`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_encointer_reputation_rings::WeightInfo for WeightInfo<T> {
	/// Storage: `EncointerReputationRings::BandersnatchKeys` (r:0 w:1)
	/// Proof: `EncointerReputationRings::BandersnatchKeys` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	fn register_bandersnatch_key() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 7_031_000 picoseconds.
		Weight::from_parts(7_359_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `EncointerReputationRings::PendingRingComputation` (r:1 w:1)
	/// Proof: `EncointerReputationRings::PendingRingComputation` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `EncointerCommunities::CommunityIdentifiers` (r:1 w:0)
	/// Proof: `EncointerCommunities::CommunityIdentifiers` (`max_values`: Some(1), `max_size`: Some(90002), added: 90497, mode: `MaxEncodedLen`)
	/// Storage: `EncointerScheduler::CurrentCeremonyIndex` (r:1 w:0)
	/// Proof: `EncointerScheduler::CurrentCeremonyIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	fn initiate_rings() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `214`
		//  Estimated: `91487`
		// Minimum execution time: 28_620_000 picoseconds.
		Weight::from_parts(37_336_000, 0)
			.saturating_add(Weight::from_parts(0, 91487))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `EncointerReputationRings::PendingRingComputation` (r:1 w:1)
	/// Proof: `EncointerReputationRings::PendingRingComputation` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `EncointerReputationRings::BandersnatchKeys` (r:501 w:0)
	/// Proof: `EncointerReputationRings::BandersnatchKeys` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `EncointerCeremonies::ParticipantReputation` (r:500 w:0)
	/// Proof: `EncointerCeremonies::ParticipantReputation` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `n` is `[10, 500]`.
	fn continue_ring_computation_collect(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `300 + n * (145 ±0)`
		//  Estimated: `3545 + n * (2621 ±0)`
		// Minimum execution time: 111_383_000 picoseconds.
		Weight::from_parts(114_708_000, 0)
			.saturating_add(Weight::from_parts(0, 3545))
			// Standard Error: 31_393
			.saturating_add(Weight::from_parts(9_814_657, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 2621).saturating_mul(n.into()))
	}
	/// Storage: `EncointerReputationRings::PendingRingComputation` (r:1 w:1)
	/// Proof: `EncointerReputationRings::PendingRingComputation` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `EncointerReputationRings::BandersnatchKeys` (r:500 w:0)
	/// Proof: `EncointerReputationRings::BandersnatchKeys` (`max_values`: None, `max_size`: Some(80), added: 2555, mode: `MaxEncodedLen`)
	/// Storage: `EncointerReputationRings::SubRingCount` (r:0 w:1)
	/// Proof: `EncointerReputationRings::SubRingCount` (`max_values`: None, `max_size`: Some(66), added: 2541, mode: `MaxEncodedLen`)
	/// Storage: `EncointerReputationRings::RingMembers` (r:0 w:2)
	/// Proof: `EncointerReputationRings::RingMembers` (`max_values`: None, `max_size`: Some(8244), added: 10719, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[10, 500]`.
	fn continue_ring_computation_build(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `50 + n * (119 ±0)`
		//  Estimated: `1531 + n * (2555 ±0)`
		// Minimum execution time: 68_544_000 picoseconds.
		Weight::from_parts(21_161_709, 0)
			.saturating_add(Weight::from_parts(0, 1531))
			// Standard Error: 26_154
			.saturating_add(Weight::from_parts(4_455_534, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 2555).saturating_mul(n.into()))
	}
}
