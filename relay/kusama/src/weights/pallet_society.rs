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

//! Autogenerated weights for `pallet_society`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-22, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `a3dce7bd4066`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("spec-kusama.json")`, DB CACHE: 1024

// Executed Command:
// /builds/polkadot-sdk/target/production/polkadot
// benchmark
// pallet
// --chain=spec-kusama.json
// --pallet=pallet_society
// --extrinsic=
// --output=/builds/runtimes/relay/kusama/src/weights
// --header=/builds/bench/header.txt
// --no-median-slopes
// --no-min-squares

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_society`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_society::WeightInfo for WeightInfo<T> {
	/// Storage: `Society::Bids` (r:1 w:1)
	/// Proof: `Society::Bids` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Candidates` (r:1 w:0)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:1 w:0)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::SuspendedMembers` (r:1 w:0)
	/// Proof: `Society::SuspendedMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Parameters` (r:1 w:0)
	/// Proof: `Society::Parameters` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn bid() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `416`
		//  Estimated: `3881`
		// Minimum execution time: 34_174_000 picoseconds.
		Weight::from_parts(34_886_000, 0)
			.saturating_add(Weight::from_parts(0, 3881))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Society::Bids` (r:1 w:1)
	/// Proof: `Society::Bids` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn unbid() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `433`
		//  Estimated: `1918`
		// Minimum execution time: 27_645_000 picoseconds.
		Weight::from_parts(29_116_000, 0)
			.saturating_add(Weight::from_parts(0, 1918))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Society::Bids` (r:1 w:1)
	/// Proof: `Society::Bids` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Candidates` (r:1 w:0)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:2 w:1)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::SuspendedMembers` (r:1 w:0)
	/// Proof: `Society::SuspendedMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn vouch() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `453`
		//  Estimated: `6393`
		// Minimum execution time: 24_738_000 picoseconds.
		Weight::from_parts(25_463_000, 0)
			.saturating_add(Weight::from_parts(0, 6393))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Bids` (r:1 w:1)
	/// Proof: `Society::Bids` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:1 w:1)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn unvouch() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `507`
		//  Estimated: `3972`
		// Minimum execution time: 19_262_000 picoseconds.
		Weight::from_parts(19_850_000, 0)
			.saturating_add(Weight::from_parts(0, 3972))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Candidates` (r:1 w:1)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:1 w:0)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Votes` (r:1 w:1)
	/// Proof: `Society::Votes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn vote() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `541`
		//  Estimated: `4006`
		// Minimum execution time: 23_895_000 picoseconds.
		Weight::from_parts(24_823_000, 0)
			.saturating_add(Weight::from_parts(0, 4006))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Defending` (r:1 w:1)
	/// Proof: `Society::Defending` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:1 w:0)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::ChallengeRoundCount` (r:1 w:0)
	/// Proof: `Society::ChallengeRoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::DefenderVotes` (r:1 w:1)
	/// Proof: `Society::DefenderVotes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn defender_vote() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `533`
		//  Estimated: `3998`
		// Minimum execution time: 22_835_000 picoseconds.
		Weight::from_parts(23_891_000, 0)
			.saturating_add(Weight::from_parts(0, 3998))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Members` (r:1 w:0)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Payouts` (r:1 w:1)
	/// Proof: `Society::Payouts` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn payout() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `622`
		//  Estimated: `4087`
		// Minimum execution time: 48_427_000 picoseconds.
		Weight::from_parts(50_771_000, 0)
			.saturating_add(Weight::from_parts(0, 4087))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Members` (r:1 w:1)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Payouts` (r:1 w:1)
	/// Proof: `Society::Payouts` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn waive_repay() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `519`
		//  Estimated: `3984`
		// Minimum execution time: 21_876_000 picoseconds.
		Weight::from_parts(22_565_000, 0)
			.saturating_add(Weight::from_parts(0, 3984))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Head` (r:1 w:1)
	/// Proof: `Society::Head` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::MemberCount` (r:1 w:1)
	/// Proof: `Society::MemberCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::MemberByIndex` (r:0 w:1)
	/// Proof: `Society::MemberByIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Founder` (r:0 w:1)
	/// Proof: `Society::Founder` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Rules` (r:0 w:1)
	/// Proof: `Society::Rules` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:0 w:1)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Parameters` (r:0 w:1)
	/// Proof: `Society::Parameters` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn found_society() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `114`
		//  Estimated: `1599`
		// Minimum execution time: 18_364_000 picoseconds.
		Weight::from_parts(18_760_000, 0)
			.saturating_add(Weight::from_parts(0, 1599))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `Society::Founder` (r:1 w:1)
	/// Proof: `Society::Founder` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::MemberCount` (r:1 w:1)
	/// Proof: `Society::MemberCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:5 w:5)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::MemberByIndex` (r:5 w:5)
	/// Proof: `Society::MemberByIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Votes` (r:4 w:4)
	/// Proof: `Society::Votes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Candidates` (r:4 w:4)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Head` (r:0 w:1)
	/// Proof: `Society::Head` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Defending` (r:0 w:1)
	/// Proof: `Society::Defending` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::ChallengeRoundCount` (r:0 w:1)
	/// Proof: `Society::ChallengeRoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Skeptic` (r:0 w:1)
	/// Proof: `Society::Skeptic` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Pot` (r:0 w:1)
	/// Proof: `Society::Pot` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Rules` (r:0 w:1)
	/// Proof: `Society::Rules` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::RoundCount` (r:0 w:1)
	/// Proof: `Society::RoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Bids` (r:0 w:1)
	/// Proof: `Society::Bids` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Parameters` (r:0 w:1)
	/// Proof: `Society::Parameters` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::NextHead` (r:0 w:1)
	/// Proof: `Society::NextHead` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn dissolve() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1626`
		//  Estimated: `14991`
		// Minimum execution time: 61_128_000 picoseconds.
		Weight::from_parts(63_743_000, 0)
			.saturating_add(Weight::from_parts(0, 14991))
			.saturating_add(T::DbWeight::get().reads(20))
			.saturating_add(T::DbWeight::get().writes(30))
	}
	/// Storage: `Society::Founder` (r:1 w:0)
	/// Proof: `Society::Founder` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::SuspendedMembers` (r:1 w:1)
	/// Proof: `Society::SuspendedMembers` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Payouts` (r:1 w:0)
	/// Proof: `Society::Payouts` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Pot` (r:1 w:1)
	/// Proof: `Society::Pot` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn judge_suspended_member() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `456`
		//  Estimated: `3921`
		// Minimum execution time: 22_445_000 picoseconds.
		Weight::from_parts(23_432_000, 0)
			.saturating_add(Weight::from_parts(0, 3921))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Founder` (r:1 w:0)
	/// Proof: `Society::Founder` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::MemberCount` (r:1 w:0)
	/// Proof: `Society::MemberCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Parameters` (r:0 w:1)
	/// Proof: `Society::Parameters` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn set_parameters() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `359`
		//  Estimated: `1844`
		// Minimum execution time: 13_473_000 picoseconds.
		Weight::from_parts(13_898_000, 0)
			.saturating_add(Weight::from_parts(0, 1844))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Society::Candidates` (r:1 w:1)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::RoundCount` (r:1 w:0)
	/// Proof: `Society::RoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Skeptic` (r:1 w:0)
	/// Proof: `Society::Skeptic` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Votes` (r:1 w:0)
	/// Proof: `Society::Votes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:1 w:1)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Parameters` (r:1 w:0)
	/// Proof: `Society::Parameters` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	fn punish_skeptic() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `608`
		//  Estimated: `4073`
		// Minimum execution time: 25_485_000 picoseconds.
		Weight::from_parts(26_279_000, 0)
			.saturating_add(Weight::from_parts(0, 4073))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::Candidates` (r:1 w:1)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::RoundCount` (r:1 w:0)
	/// Proof: `Society::RoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Parameters` (r:1 w:0)
	/// Proof: `Society::Parameters` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::MemberCount` (r:1 w:1)
	/// Proof: `Society::MemberCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::NextHead` (r:1 w:1)
	/// Proof: `Society::NextHead` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Society::MemberByIndex` (r:0 w:1)
	/// Proof: `Society::MemberByIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:0 w:1)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn claim_membership() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `604`
		//  Estimated: `4069`
		// Minimum execution time: 40_010_000 picoseconds.
		Weight::from_parts(41_904_000, 0)
			.saturating_add(Weight::from_parts(0, 4069))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `Society::Founder` (r:1 w:0)
	/// Proof: `Society::Founder` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Candidates` (r:1 w:1)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::RoundCount` (r:1 w:0)
	/// Proof: `Society::RoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Parameters` (r:1 w:0)
	/// Proof: `Society::Parameters` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::MemberCount` (r:1 w:1)
	/// Proof: `Society::MemberCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::NextHead` (r:1 w:1)
	/// Proof: `Society::NextHead` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Society::MemberByIndex` (r:0 w:1)
	/// Proof: `Society::MemberByIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Members` (r:0 w:1)
	/// Proof: `Society::Members` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn bestow_membership() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `622`
		//  Estimated: `4087`
		// Minimum execution time: 42_167_000 picoseconds.
		Weight::from_parts(43_387_000, 0)
			.saturating_add(Weight::from_parts(0, 4087))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `Society::Founder` (r:1 w:0)
	/// Proof: `Society::Founder` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Candidates` (r:1 w:1)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::RoundCount` (r:1 w:0)
	/// Proof: `Society::RoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn kick_candidate() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `748`
		//  Estimated: `6196`
		// Minimum execution time: 39_265_000 picoseconds.
		Weight::from_parts(40_839_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Society::Candidates` (r:1 w:1)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::RoundCount` (r:1 w:0)
	/// Proof: `Society::RoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn resign_candidacy() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `718`
		//  Estimated: `6196`
		// Minimum execution time: 36_476_000 picoseconds.
		Weight::from_parts(37_762_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Society::Candidates` (r:1 w:1)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::RoundCount` (r:1 w:0)
	/// Proof: `Society::RoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	fn drop_candidate() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `730`
		//  Estimated: `6196`
		// Minimum execution time: 36_674_000 picoseconds.
		Weight::from_parts(37_917_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Society::Candidates` (r:1 w:0)
	/// Proof: `Society::Candidates` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::VoteClearCursor` (r:1 w:0)
	/// Proof: `Society::VoteClearCursor` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Society::Votes` (r:2 w:2)
	/// Proof: `Society::Votes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn cleanup_candidacy() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `524`
		//  Estimated: `6464`
		// Minimum execution time: 16_583_000 picoseconds.
		Weight::from_parts(17_185_000, 0)
			.saturating_add(Weight::from_parts(0, 6464))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Society::ChallengeRoundCount` (r:1 w:0)
	/// Proof: `Society::ChallengeRoundCount` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Society::DefenderVotes` (r:1 w:1)
	/// Proof: `Society::DefenderVotes` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn cleanup_challenge() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `482`
		//  Estimated: `3947`
		// Minimum execution time: 11_942_000 picoseconds.
		Weight::from_parts(12_421_000, 0)
			.saturating_add(Weight::from_parts(0, 3947))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
