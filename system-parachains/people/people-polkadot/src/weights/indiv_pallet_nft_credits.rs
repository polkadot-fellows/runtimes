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

//! TMP weights for `indiv_pallet_nft_credits`, to be replaced by a benchmark on reference hardware.
//!
//! Until this runtime is benchmarked, these are the weights of upstream's `next-people-paseo`
//! runtime at individuality-community `24033977`
//! (`runtimes/next-people-paseo/src/weights/indiv_pallet_nft_credits.rs`, STEPS 50, REPEAT 20, on
//! `parity-weights` hardware).

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `indiv_pallet_nft_credits`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> indiv_pallet_nft_credits::WeightInfo for WeightInfo<T> {
	/// Storage: `NftCredits::CreditBuffers` (r:1 w:1)
	/// Proof: `NftCredits::CreditBuffers` (`max_values`: None, `max_size`: Some(24), added: 2499, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NftClaimCreditAwards` (r:64 w:0)
	/// Proof: `NftCredits::NftClaimCreditAwards` (`max_values`: None, `max_size`: Some(2105), added: 4580, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NextCreditTreeSequence` (r:1 w:1)
	/// Proof: `NftCredits::NextCreditTreeSequence` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::CreditTreeDeliveryQueue` (r:1 w:1)
	/// Proof: `NftCredits::CreditTreeDeliveryQueue` (`max_values`: Some(1), `max_size`: Some(3074), added: 3569, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NftClaimCreditRoots` (r:0 w:1)
	/// Proof: `NftCredits::NftClaimCreditRoots` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NftClaimCreditAwardExpiries` (r:0 w:1)
	/// Proof: `NftCredits::NftClaimCreditAwardExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::RootExpiries` (r:0 w:1)
	/// Proof: `NftCredits::RootExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 2048]`.
	fn build_credit_tree(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `204 + n * (65 ±0)`
		//  Estimated: `4559 + n * (143 ±0)`
		// Minimum execution time: 21_587_000 picoseconds.
		Weight::from_parts(17_345_402, 0)
			.saturating_add(Weight::from_parts(0, 4559))
			// Standard Error: 799
			.saturating_add(Weight::from_parts(1_208_410, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(6))
			.saturating_add(Weight::from_parts(0, 143).saturating_mul(n.into()))
	}
	/// Storage: `NftCredits::CreditBuffers` (r:1 w:0)
	/// Proof: `NftCredits::CreditBuffers` (`max_values`: None, `max_size`: Some(24), added: 2499, mode: `MaxEncodedLen`)
	fn build_credit_tree_empty() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `114`
		//  Estimated: `3489`
		// Minimum execution time: 4_077_000 picoseconds.
		Weight::from_parts(4_428_000, 0)
			.saturating_add(Weight::from_parts(0, 3489))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `NftCredits::CreditTreeDeliveryQueue` (r:1 w:1)
	/// Proof: `NftCredits::CreditTreeDeliveryQueue` (`max_values`: Some(1), `max_size`: Some(3074), added: 3569, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::RelevantMessagingState` (r:1 w:0)
	/// Proof: `ParachainSystem::RelevantMessagingState` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `NftCredits::NftClaimCreditRoots` (r:32 w:0)
	/// Proof: `NftCredits::NftClaimCreditRoots` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::DeliveryFeeFactor` (r:1 w:0)
	/// Proof: `XcmpQueue::DeliveryFeeFactor` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `PolkadotXcm::SupportedVersion` (r:1 w:0)
	/// Proof: `PolkadotXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(2306), added: 2801, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::OutboundXcmpMessages` (r:0 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpMessages` (`max_values`: None, `max_size`: Some(105506), added: 107981, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 32]`.
	fn send_credit_trees(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `5237 + n * (188 ±0)`
		//  Estimated: `8592 + n * (2531 ±6)`
		// Minimum execution time: 74_082_000 picoseconds.
		Weight::from_parts(75_993_853, 0)
			.saturating_add(Weight::from_parts(0, 8592))
			// Standard Error: 20_916
			.saturating_add(Weight::from_parts(5_219_174, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 2531).saturating_mul(n.into()))
	}
	/// Storage: `ParachainSystem::RelevantMessagingState` (r:1 w:0)
	/// Proof: `ParachainSystem::RelevantMessagingState` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `NftCredits::NftClaimCreditRoots` (r:32 w:0)
	/// Proof: `NftCredits::NftClaimCreditRoots` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::LastReplayTime` (r:1 w:1)
	/// Proof: `NftCredits::LastReplayTime` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::DeliveryFeeFactor` (r:1 w:0)
	/// Proof: `XcmpQueue::DeliveryFeeFactor` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `PolkadotXcm::SupportedVersion` (r:1 w:0)
	/// Proof: `PolkadotXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(2306), added: 2801, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::OutboundXcmpMessages` (r:0 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpMessages` (`max_values`: None, `max_size`: Some(105506), added: 107981, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 32]`.
	fn replay_credit_trees(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `434 + n * (63 ±0)`
		//  Estimated: `3900 + n * (2531 ±0)`
		// Minimum execution time: 47_976_000 picoseconds.
		Weight::from_parts(46_452_996, 0)
			.saturating_add(Weight::from_parts(0, 3900))
			// Standard Error: 6_912
			.saturating_add(Weight::from_parts(3_993_484, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 2531).saturating_mul(n.into()))
	}
	/// Storage: `NftCredits::CreditTreeDeliveryQueue` (r:1 w:0)
	/// Proof: `NftCredits::CreditTreeDeliveryQueue` (`max_values`: Some(1), `max_size`: Some(3074), added: 3569, mode: `MaxEncodedLen`)
	fn authorize_send_credit_trees() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `3276`
		//  Estimated: `4559`
		// Minimum execution time: 10_316_000 picoseconds.
		Weight::from_parts(10_998_000, 0)
			.saturating_add(Weight::from_parts(0, 4559))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `NftCredits::NftClaimCreditRoots` (r:64 w:64)
	/// Proof: `NftCredits::NftClaimCreditRoots` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::RootExpiries` (r:0 w:64)
	/// Proof: `NftCredits::RootExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 64]`.
	fn receive_tree_deletions(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `171 + n * (62 ±0)`
		//  Estimated: `990 + n * (2531 ±0)`
		// Minimum execution time: 14_291_000 picoseconds.
		Weight::from_parts(6_556_634, 0)
			.saturating_add(Weight::from_parts(0, 990))
			// Standard Error: 6_403
			.saturating_add(Weight::from_parts(5_887_985, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2531).saturating_mul(n.into()))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::RootExpiries` (r:64 w:64)
	/// Proof: `NftCredits::RootExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NftClaimCreditRoots` (r:0 w:64)
	/// Proof: `NftCredits::NftClaimCreditRoots` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 64]`.
	fn sweep_expired_roots(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `264 + n * (15 ±0)`
		//  Estimated: `3575 + n * (2486 ±1)`
		// Minimum execution time: 9_898_000 picoseconds.
		Weight::from_parts(14_978_425, 0)
			.saturating_add(Weight::from_parts(0, 3575))
			// Standard Error: 3_920
			.saturating_add(Weight::from_parts(3_569_578, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2486).saturating_mul(n.into()))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::RootExpiries` (r:1 w:0)
	/// Proof: `NftCredits::RootExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	fn authorize_sweep_expired_roots() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `612`
		//  Estimated: `3481`
		// Minimum execution time: 11_132_000 picoseconds.
		Weight::from_parts(12_209_000, 0)
			.saturating_add(Weight::from_parts(0, 3481))
			.saturating_add(T::DbWeight::get().reads(2))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NftClaimCreditAwardExpiries` (r:8 w:8)
	/// Proof: `NftCredits::NftClaimCreditAwardExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NftClaimCreditAwards` (r:512 w:512)
	/// Proof: `NftCredits::NftClaimCreditAwards` (`max_values`: None, `max_size`: Some(2105), added: 4580, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 8]`.
	fn sweep_expired_awards(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `284 + n * (3082 ±0)`
		//  Estimated: `3604 + n * (293120 ±29)`
		// Minimum execution time: 12_258_000 picoseconds.
		Weight::from_parts(10_570_732, 0)
			.saturating_add(Weight::from_parts(0, 3604))
			// Standard Error: 128_762
			.saturating_add(Weight::from_parts(90_435_262, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((65_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes((65_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 293120).saturating_mul(n.into()))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftCredits::NftClaimCreditAwardExpiries` (r:1 w:0)
	/// Proof: `NftCredits::NftClaimCreditAwardExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	fn authorize_sweep_expired_awards() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `386`
		//  Estimated: `3481`
		// Minimum execution time: 16_463_000 picoseconds.
		Weight::from_parts(18_274_000, 0)
			.saturating_add(Weight::from_parts(0, 3481))
			.saturating_add(T::DbWeight::get().reads(2))
	}
}
