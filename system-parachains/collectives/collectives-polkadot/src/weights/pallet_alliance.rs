// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Autogenerated weights for `pallet_alliance`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-12-19, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("../collectives-polkadot-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot
// benchmark
// pallet
// --chain=../collectives-polkadot-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=pallet_alliance
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./collectives-polkadot-weights
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_alliance`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_alliance::WeightInfo for WeightInfo<T> {
	/// Storage: `Alliance::Members` (r:1 w:0)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::ProposalOf` (r:1 w:1)
	/// Proof: `AllianceMotion::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:1)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::ProposalCount` (r:1 w:1)
	/// Proof: `AllianceMotion::ProposalCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Voting` (r:0 w:1)
	/// Proof: `AllianceMotion::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 1024]`.
	/// The range of component `m` is `[2, 100]`.
	/// The range of component `p` is `[1, 100]`.
	fn propose_proposed(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `439 + m * (32 ±0) + p * (36 ±0)`
		//  Estimated: `6676 + m * (32 ±0) + p * (36 ±0)`
		// Minimum execution time: 27_541_000 picoseconds.
		Weight::from_parts(26_501_076, 0)
			.saturating_add(Weight::from_parts(0, 6676))
			// Standard Error: 155
			.saturating_add(Weight::from_parts(1_699, 0).saturating_mul(b.into()))
			// Standard Error: 1_620
			.saturating_add(Weight::from_parts(41_100, 0).saturating_mul(m.into()))
			// Standard Error: 1_600
			.saturating_add(Weight::from_parts(165_984, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 32).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(p.into()))
	}
	/// Storage: `Alliance::Members` (r:1 w:0)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Voting` (r:1 w:1)
	/// Proof: `AllianceMotion::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[5, 100]`.
	fn vote(m: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `868 + m * (64 ±0)`
		//  Estimated: `6676 + m * (64 ±0)`
		// Minimum execution time: 25_747_000 picoseconds.
		Weight::from_parts(26_539_634, 0)
			.saturating_add(Weight::from_parts(0, 6676))
			// Standard Error: 1_503
			.saturating_add(Weight::from_parts(64_366, 0).saturating_mul(m.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(Weight::from_parts(0, 64).saturating_mul(m.into()))
	}
	/// Storage: `Alliance::Members` (r:1 w:0)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Voting` (r:1 w:1)
	/// Proof: `AllianceMotion::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:1 w:0)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:1)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::ProposalOf` (r:0 w:1)
	/// Proof: `AllianceMotion::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[4, 100]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_early_disapproved(m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `312 + m * (96 ±0) + p * (36 ±0)`
		//  Estimated: `6676 + m * (97 ±0) + p * (36 ±0)`
		// Minimum execution time: 33_675_000 picoseconds.
		Weight::from_parts(29_794_245, 0)
			.saturating_add(Weight::from_parts(0, 6676))
			// Standard Error: 1_663
			.saturating_add(Weight::from_parts(76_731, 0).saturating_mul(m.into()))
			// Standard Error: 1_622
			.saturating_add(Weight::from_parts(148_783, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 97).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(p.into()))
	}
	/// Storage: `Alliance::Members` (r:1 w:0)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Voting` (r:1 w:1)
	/// Proof: `AllianceMotion::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:1 w:0)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::ProposalOf` (r:1 w:1)
	/// Proof: `AllianceMotion::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:1)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 1024]`.
	/// The range of component `m` is `[4, 100]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_early_approved(b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `762 + m * (96 ±0) + p * (41 ±0)`
		//  Estimated: `6676 + m * (97 ±0) + p * (40 ±0)`
		// Minimum execution time: 45_057_000 picoseconds.
		Weight::from_parts(39_969_457, 0)
			.saturating_add(Weight::from_parts(0, 6676))
			// Standard Error: 289
			.saturating_add(Weight::from_parts(1_338, 0).saturating_mul(b.into()))
			// Standard Error: 3_059
			.saturating_add(Weight::from_parts(95_484, 0).saturating_mul(m.into()))
			// Standard Error: 2_982
			.saturating_add(Weight::from_parts(185_014, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 97).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 40).saturating_mul(p.into()))
	}
	/// Storage: `Alliance::Members` (r:1 w:0)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Voting` (r:1 w:1)
	/// Proof: `AllianceMotion::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:1 w:0)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Prime` (r:1 w:0)
	/// Proof: `AllianceMotion::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::ProposalOf` (r:1 w:1)
	/// Proof: `AllianceMotion::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:1)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Alliance::Rule` (r:0 w:1)
	/// Proof: `Alliance::Rule` (`max_values`: Some(1), `max_size`: Some(87), added: 582, mode: `MaxEncodedLen`)
	/// The range of component `m` is `[2, 100]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_disapproved(m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `518 + m * (96 ±0) + p * (41 ±0)`
		//  Estimated: `6676 + m * (109 ±0) + p * (43 ±0)`
		// Minimum execution time: 44_188_000 picoseconds.
		Weight::from_parts(38_418_500, 0)
			.saturating_add(Weight::from_parts(0, 6676))
			// Standard Error: 4_327
			.saturating_add(Weight::from_parts(163_142, 0).saturating_mul(m.into()))
			// Standard Error: 4_275
			.saturating_add(Weight::from_parts(206_104, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(Weight::from_parts(0, 109).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 43).saturating_mul(p.into()))
	}
	/// Storage: `Alliance::Members` (r:1 w:0)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Voting` (r:1 w:1)
	/// Proof: `AllianceMotion::Voting` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:1 w:0)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Prime` (r:1 w:0)
	/// Proof: `AllianceMotion::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:1)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::ProposalOf` (r:0 w:1)
	/// Proof: `AllianceMotion::ProposalOf` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `b` is `[1, 1024]`.
	/// The range of component `m` is `[5, 100]`.
	/// The range of component `p` is `[1, 100]`.
	fn close_approved(_b: u32, m: u32, p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `417 + m * (96 ±0) + p * (36 ±0)`
		//  Estimated: `6676 + m * (96 ±0) + p * (36 ±0)`
		// Minimum execution time: 34_952_000 picoseconds.
		Weight::from_parts(31_369_845, 0)
			.saturating_add(Weight::from_parts(0, 6676))
			// Standard Error: 2_026
			.saturating_add(Weight::from_parts(73_401, 0).saturating_mul(m.into()))
			// Standard Error: 1_953
			.saturating_add(Weight::from_parts(152_033, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 96).saturating_mul(m.into()))
			.saturating_add(Weight::from_parts(0, 36).saturating_mul(p.into()))
	}
	/// Storage: `Alliance::Members` (r:2 w:2)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Members` (r:1 w:1)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `m` is `[1, 100]`.
	/// The range of component `z` is `[0, 100]`.
	fn init_members(m: u32, z: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `12`
		//  Estimated: `12362`
		// Minimum execution time: 26_263_000 picoseconds.
		Weight::from_parts(14_666_996, 0)
			.saturating_add(Weight::from_parts(0, 12362))
			// Standard Error: 1_302
			.saturating_add(Weight::from_parts(154_678, 0).saturating_mul(m.into()))
			// Standard Error: 1_287
			.saturating_add(Weight::from_parts(127_234, 0).saturating_mul(z.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Alliance::Members` (r:2 w:2)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:0)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Alliance::DepositOf` (r:200 w:50)
	/// Proof: `Alliance::DepositOf` (`max_values`: None, `max_size`: Some(64), added: 2539, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:50 w:50)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Members` (r:0 w:1)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Prime` (r:0 w:1)
	/// Proof: `AllianceMotion::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// The range of component `x` is `[1, 100]`.
	/// The range of component `y` is `[0, 100]`.
	/// The range of component `z` is `[0, 50]`.
	fn disband(x: u32, y: u32, z: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + x * (51 ±0) + y * (53 ±0) + z * (252 ±0)`
		//  Estimated: `12362 + x * (2539 ±0) + y * (2539 ±0) + z * (2603 ±1)`
		// Minimum execution time: 297_271_000 picoseconds.
		Weight::from_parts(299_901_000, 0)
			.saturating_add(Weight::from_parts(0, 12362))
			// Standard Error: 24_181
			.saturating_add(Weight::from_parts(559_293, 0).saturating_mul(x.into()))
			// Standard Error: 24_064
			.saturating_add(Weight::from_parts(560_126, 0).saturating_mul(y.into()))
			// Standard Error: 48_085
			.saturating_add(Weight::from_parts(13_515_780, 0).saturating_mul(z.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(x.into())))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(y.into())))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(z.into())))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(z.into())))
			.saturating_add(Weight::from_parts(0, 2539).saturating_mul(x.into()))
			.saturating_add(Weight::from_parts(0, 2539).saturating_mul(y.into()))
			.saturating_add(Weight::from_parts(0, 2603).saturating_mul(z.into()))
	}
	/// Storage: `Alliance::Rule` (r:0 w:1)
	/// Proof: `Alliance::Rule` (`max_values`: Some(1), `max_size`: Some(87), added: 582, mode: `MaxEncodedLen`)
	fn set_rule() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 5_818_000 picoseconds.
		Weight::from_parts(6_067_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Alliance::Announcements` (r:1 w:1)
	/// Proof: `Alliance::Announcements` (`max_values`: Some(1), `max_size`: Some(8702), added: 9197, mode: `MaxEncodedLen`)
	fn announce() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `10187`
		// Minimum execution time: 7_927_000 picoseconds.
		Weight::from_parts(8_271_000, 0)
			.saturating_add(Weight::from_parts(0, 10187))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Alliance::Announcements` (r:1 w:1)
	/// Proof: `Alliance::Announcements` (`max_values`: Some(1), `max_size`: Some(8702), added: 9197, mode: `MaxEncodedLen`)
	fn remove_announcement() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `149`
		//  Estimated: `10187`
		// Minimum execution time: 8_919_000 picoseconds.
		Weight::from_parts(9_307_000, 0)
			.saturating_add(Weight::from_parts(0, 10187))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Alliance::Members` (r:3 w:1)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `Alliance::UnscrupulousAccounts` (r:1 w:0)
	/// Proof: `Alliance::UnscrupulousAccounts` (`max_values`: Some(1), `max_size`: Some(3202), added: 3697, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Alliance::DepositOf` (r:0 w:1)
	/// Proof: `Alliance::DepositOf` (`max_values`: None, `max_size`: Some(64), added: 2539, mode: `MaxEncodedLen`)
	fn join_alliance() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `294`
		//  Estimated: `18048`
		// Minimum execution time: 36_389_000 picoseconds.
		Weight::from_parts(37_687_000, 0)
			.saturating_add(Weight::from_parts(0, 18048))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Alliance::Members` (r:3 w:1)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `Alliance::UnscrupulousAccounts` (r:1 w:0)
	/// Proof: `Alliance::UnscrupulousAccounts` (`max_values`: Some(1), `max_size`: Some(3202), added: 3697, mode: `MaxEncodedLen`)
	fn nominate_ally() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `193`
		//  Estimated: `18048`
		// Minimum execution time: 22_348_000 picoseconds.
		Weight::from_parts(22_995_000, 0)
			.saturating_add(Weight::from_parts(0, 18048))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Alliance::Members` (r:2 w:2)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:0)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:0 w:1)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Prime` (r:0 w:1)
	/// Proof: `AllianceMotion::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn elevate_ally() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `236`
		//  Estimated: `12362`
		// Minimum execution time: 20_967_000 picoseconds.
		Weight::from_parts(21_594_000, 0)
			.saturating_add(Weight::from_parts(0, 12362))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Alliance::Members` (r:4 w:2)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:0)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:0 w:1)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Prime` (r:0 w:1)
	/// Proof: `AllianceMotion::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Alliance::RetiringMembers` (r:0 w:1)
	/// Proof: `Alliance::RetiringMembers` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	fn give_retirement_notice() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `236`
		//  Estimated: `23734`
		// Minimum execution time: 25_677_000 picoseconds.
		Weight::from_parts(27_070_000, 0)
			.saturating_add(Weight::from_parts(0, 23734))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Alliance::RetiringMembers` (r:1 w:1)
	/// Proof: `Alliance::RetiringMembers` (`max_values`: None, `max_size`: Some(52), added: 2527, mode: `MaxEncodedLen`)
	/// Storage: `Alliance::Members` (r:1 w:1)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `Alliance::DepositOf` (r:1 w:1)
	/// Proof: `Alliance::DepositOf` (`max_values`: None, `max_size`: Some(64), added: 2539, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn retire() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `517`
		//  Estimated: `6676`
		// Minimum execution time: 34_032_000 picoseconds.
		Weight::from_parts(34_935_000, 0)
			.saturating_add(Weight::from_parts(0, 6676))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Alliance::Members` (r:3 w:1)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:0)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Alliance::DepositOf` (r:1 w:1)
	/// Proof: `Alliance::DepositOf` (`max_values`: None, `max_size`: Some(64), added: 2539, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `ParachainInfo::ParachainId` (r:1 w:0)
	/// Proof: `ParachainInfo::ParachainId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::UpwardDeliveryFeeFactor` (r:1 w:0)
	/// Proof: `ParachainSystem::UpwardDeliveryFeeFactor` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `PolkadotXcm::SupportedVersion` (r:1 w:0)
	/// Proof: `PolkadotXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `PolkadotXcm::VersionDiscoveryQueue` (r:1 w:1)
	/// Proof: `PolkadotXcm::VersionDiscoveryQueue` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `PolkadotXcm::SafeXcmVersion` (r:1 w:0)
	/// Proof: `PolkadotXcm::SafeXcmVersion` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:0 w:1)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Prime` (r:0 w:1)
	/// Proof: `AllianceMotion::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn kick_member() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `643`
		//  Estimated: `18048`
		// Minimum execution time: 122_327_000 picoseconds.
		Weight::from_parts(126_204_000, 0)
			.saturating_add(Weight::from_parts(0, 18048))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Alliance::UnscrupulousAccounts` (r:1 w:1)
	/// Proof: `Alliance::UnscrupulousAccounts` (`max_values`: Some(1), `max_size`: Some(3202), added: 3697, mode: `MaxEncodedLen`)
	/// Storage: `Alliance::UnscrupulousWebsites` (r:1 w:1)
	/// Proof: `Alliance::UnscrupulousWebsites` (`max_values`: Some(1), `max_size`: Some(25702), added: 26197, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 100]`.
	/// The range of component `l` is `[0, 255]`.
	fn add_unscrupulous_items(n: u32, l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `76`
		//  Estimated: `27187`
		// Minimum execution time: 5_053_000 picoseconds.
		Weight::from_parts(5_227_000, 0)
			.saturating_add(Weight::from_parts(0, 27187))
			// Standard Error: 3_966
			.saturating_add(Weight::from_parts(1_007_511, 0).saturating_mul(n.into()))
			// Standard Error: 1_553
			.saturating_add(Weight::from_parts(71_070, 0).saturating_mul(l.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Alliance::UnscrupulousAccounts` (r:1 w:1)
	/// Proof: `Alliance::UnscrupulousAccounts` (`max_values`: Some(1), `max_size`: Some(3202), added: 3697, mode: `MaxEncodedLen`)
	/// Storage: `Alliance::UnscrupulousWebsites` (r:1 w:1)
	/// Proof: `Alliance::UnscrupulousWebsites` (`max_values`: Some(1), `max_size`: Some(25702), added: 26197, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 100]`.
	/// The range of component `l` is `[0, 255]`.
	fn remove_unscrupulous_items(n: u32, l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0 + l * (100 ±0) + n * (289 ±0)`
		//  Estimated: `27187`
		// Minimum execution time: 5_117_000 picoseconds.
		Weight::from_parts(5_276_000, 0)
			.saturating_add(Weight::from_parts(0, 27187))
			// Standard Error: 186_720
			.saturating_add(Weight::from_parts(17_319_715, 0).saturating_mul(n.into()))
			// Standard Error: 73_127
			.saturating_add(Weight::from_parts(268_546, 0).saturating_mul(l.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Alliance::Members` (r:3 w:2)
	/// Proof: `Alliance::Members` (`max_values`: None, `max_size`: Some(3211), added: 5686, mode: `MaxEncodedLen`)
	/// Storage: `AllianceMotion::Proposals` (r:1 w:0)
	/// Proof: `AllianceMotion::Proposals` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Members` (r:0 w:1)
	/// Proof: `AllianceMotion::Members` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `AllianceMotion::Prime` (r:0 w:1)
	/// Proof: `AllianceMotion::Prime` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn abdicate_fellow_status() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `236`
		//  Estimated: `18048`
		// Minimum execution time: 25_385_000 picoseconds.
		Weight::from_parts(26_058_000, 0)
			.saturating_add(Weight::from_parts(0, 18048))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(4))
	}
}
