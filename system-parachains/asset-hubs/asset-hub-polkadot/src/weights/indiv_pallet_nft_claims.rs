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

//! TMP weights for `indiv_pallet_nft_claims`, to be replaced by a benchmark on reference hardware.
//!
//! Until this runtime is benchmarked, these are the weights of upstream's `next-asset-hub-paseo`
//! runtime at individuality-community `24033977`
//! (`runtimes/next-asset-hub-paseo/src/weights/indiv_pallet_nft_claims.rs`, STEPS 50, REPEAT 20,
//! on `parity-weights` hardware).

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `indiv_pallet_nft_claims`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> indiv_pallet_nft_claims::WeightInfo for WeightInfo<T> {
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CreditTrees` (r:32 w:32)
	/// Proof: `NftClaims::CreditTrees` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::NextExpectedSequence` (r:1 w:1)
	/// Proof: `NftClaims::NextExpectedSequence` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::TreeExpiries` (r:0 w:32)
	/// Proof: `NftClaims::TreeExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 32]`.
	fn receive_credit_trees(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `78`
		//  Estimated: `1493 + n * (2531 ±0)`
		// Minimum execution time: 21_382_000 picoseconds.
		Weight::from_parts(16_066_179, 0)
			.saturating_add(Weight::from_parts(0, 1493))
			// Standard Error: 7_448
			.saturating_add(Weight::from_parts(4_819_985, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2531).saturating_mul(n.into()))
	}
	/// Storage: `NftClaims::CreditTrees` (r:1 w:0)
	/// Proof: `NftClaims::CreditTrees` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::ClaimedLeaves` (r:1 w:1)
	/// Proof: `NftClaims::ClaimedLeaves` (`max_values`: None, `max_size`: Some(270), added: 2745, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CollectionMinters` (r:1 w:0)
	/// Proof: `NftClaims::CollectionMinters` (`max_values`: None, `max_size`: Some(65), added: 2540, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NextInstanceId` (r:1 w:1)
	/// Proof: `Scarcity::NextInstanceId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadataCount` (r:0 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 11]`.
	fn claim_account(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `504`
		//  Estimated: `3735`
		// Minimum execution time: 63_764_000 picoseconds.
		Weight::from_parts(66_552_881, 0)
			.saturating_add(Weight::from_parts(0, 3735))
			// Standard Error: 16_432
			.saturating_add(Weight::from_parts(451_375, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `AliasAccounts::AccountToAlias` (r:1 w:0)
	/// Proof: `AliasAccounts::AccountToAlias` (`max_values`: None, `max_size`: Some(152), added: 2627, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CreditTrees` (r:1 w:0)
	/// Proof: `NftClaims::CreditTrees` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::ClaimedLeaves` (r:1 w:1)
	/// Proof: `NftClaims::ClaimedLeaves` (`max_values`: None, `max_size`: Some(270), added: 2745, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CollectionMinters` (r:1 w:0)
	/// Proof: `NftClaims::CollectionMinters` (`max_values`: None, `max_size`: Some(65), added: 2540, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NextInstanceId` (r:1 w:1)
	/// Proof: `Scarcity::NextInstanceId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadataCount` (r:0 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 11]`.
	fn claim_person(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `794`
		//  Estimated: `3735`
		// Minimum execution time: 71_918_000 picoseconds.
		Weight::from_parts(75_378_589, 0)
			.saturating_add(Weight::from_parts(0, 3735))
			// Standard Error: 23_198
			.saturating_add(Weight::from_parts(412_945, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `NftClaims::CreditTrees` (r:1 w:1)
	/// Proof: `NftClaims::CreditTrees` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::ClaimedLeaves` (r:1 w:1)
	/// Proof: `NftClaims::ClaimedLeaves` (`max_values`: None, `max_size`: Some(270), added: 2745, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CollectionMinters` (r:1 w:0)
	/// Proof: `NftClaims::CollectionMinters` (`max_values`: None, `max_size`: Some(65), added: 2540, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NextInstanceId` (r:1 w:1)
	/// Proof: `Scarcity::NextInstanceId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::PendingTreeDeletions` (r:1 w:1)
	/// Proof: `NftClaims::PendingTreeDeletions` (`max_values`: Some(1), `max_size`: Some(514), added: 1009, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadataCount` (r:0 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 11]`.
	fn claim_last_account(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `528 + n * (4 ±0)`
		//  Estimated: `3735`
		// Minimum execution time: 70_405_000 picoseconds.
		Weight::from_parts(74_909_938, 0)
			.saturating_add(Weight::from_parts(0, 3735))
			// Standard Error: 17_865
			.saturating_add(Weight::from_parts(525_686, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `AliasAccounts::AccountToAlias` (r:1 w:0)
	/// Proof: `AliasAccounts::AccountToAlias` (`max_values`: None, `max_size`: Some(152), added: 2627, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CreditTrees` (r:1 w:1)
	/// Proof: `NftClaims::CreditTrees` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::ClaimedLeaves` (r:1 w:1)
	/// Proof: `NftClaims::ClaimedLeaves` (`max_values`: None, `max_size`: Some(270), added: 2745, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CollectionMinters` (r:1 w:0)
	/// Proof: `NftClaims::CollectionMinters` (`max_values`: None, `max_size`: Some(65), added: 2540, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Collections` (r:1 w:1)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::ItemDefs` (r:1 w:1)
	/// Proof: `Scarcity::ItemDefs` (`max_values`: None, `max_size`: Some(53), added: 2528, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NftsByOwner` (r:1 w:1)
	/// Proof: `Scarcity::NftsByOwner` (`max_values`: None, `max_size`: Some(88), added: 2563, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::NextInstanceId` (r:1 w:1)
	/// Proof: `Scarcity::NextInstanceId` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::PendingTreeDeletions` (r:1 w:1)
	/// Proof: `NftClaims::PendingTreeDeletions` (`max_values`: Some(1), `max_size`: Some(514), added: 1009, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::InstanceMetadataCount` (r:0 w:1)
	/// Proof: `Scarcity::InstanceMetadataCount` (`max_values`: None, `max_size`: Some(20), added: 2495, mode: `MaxEncodedLen`)
	/// Storage: `Scarcity::Instances` (r:0 w:1)
	/// Proof: `Scarcity::Instances` (`max_values`: None, `max_size`: Some(48), added: 2523, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 11]`.
	fn claim_last_person(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `818 + n * (4 ±0)`
		//  Estimated: `3735`
		// Minimum execution time: 78_320_000 picoseconds.
		Weight::from_parts(82_482_299, 0)
			.saturating_add(Weight::from_parts(0, 3735))
			// Standard Error: 15_401
			.saturating_add(Weight::from_parts(463_814, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(10))
			.saturating_add(T::DbWeight::get().writes(9))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::TreeExpiries` (r:64 w:64)
	/// Proof: `NftClaims::TreeExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CreditTrees` (r:64 w:64)
	/// Proof: `NftClaims::CreditTrees` (`max_values`: None, `max_size`: Some(56), added: 2531, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::PendingTreeDeletions` (r:1 w:1)
	/// Proof: `NftClaims::PendingTreeDeletions` (`max_values`: Some(1), `max_size`: Some(514), added: 1009, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::ClaimedLeaves` (r:0 w:64)
	/// Proof: `NftClaims::ClaimedLeaves` (`max_values`: None, `max_size`: Some(270), added: 2745, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[0, 64]`.
	fn sweep_expired_trees(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `210 + n * (78 ±0)`
		//  Estimated: `3575 + n * (2531 ±2)`
		// Minimum execution time: 11_395_000 picoseconds.
		Weight::from_parts(21_326_983, 0)
			.saturating_add(Weight::from_parts(0, 3575))
			// Standard Error: 16_789
			.saturating_add(Weight::from_parts(8_338_544, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(n.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(n.into())))
			.saturating_add(Weight::from_parts(0, 2531).saturating_mul(n.into()))
	}
	/// Storage: `Timestamp::Now` (r:1 w:0)
	/// Proof: `Timestamp::Now` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::TreeExpiries` (r:1 w:0)
	/// Proof: `NftClaims::TreeExpiries` (`max_values`: None, `max_size`: Some(16), added: 2491, mode: `MaxEncodedLen`)
	fn authorize_sweep_expired_trees() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `535`
		//  Estimated: `3481`
		// Minimum execution time: 13_147_000 picoseconds.
		Weight::from_parts(14_334_000, 0)
			.saturating_add(Weight::from_parts(0, 3481))
			.saturating_add(T::DbWeight::get().reads(2))
	}
	/// Storage: `NftClaims::PendingTreeDeletions` (r:1 w:1)
	/// Proof: `NftClaims::PendingTreeDeletions` (`max_values`: Some(1), `max_size`: Some(514), added: 1009, mode: `MaxEncodedLen`)
	/// Storage: `XcmpQueue::DeliveryFeeFactor` (r:1 w:0)
	/// Proof: `XcmpQueue::DeliveryFeeFactor` (`max_values`: None, `max_size`: Some(28), added: 2503, mode: `MaxEncodedLen`)
	/// Storage: `PolkadotXcm::SupportedVersion` (r:1 w:0)
	/// Proof: `PolkadotXcm::SupportedVersion` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `XcmpQueue::OutboundXcmpStatus` (r:1 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpStatus` (`max_values`: Some(1), `max_size`: Some(2306), added: 2801, mode: `MaxEncodedLen`)
	/// Storage: `ParachainSystem::RelevantMessagingState` (r:1 w:0)
	/// Proof: `ParachainSystem::RelevantMessagingState` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `XcmpQueue::OutboundXcmpMessages` (r:0 w:1)
	/// Proof: `XcmpQueue::OutboundXcmpMessages` (`max_values`: None, `max_size`: Some(105506), added: 107981, mode: `MaxEncodedLen`)
	/// The range of component `n` is `[1, 64]`.
	fn send_tree_deletions(n: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `263 + n * (4 ±0)`
		//  Estimated: `3791 + n * (4 ±0)`
		// Minimum execution time: 38_800_000 picoseconds.
		Weight::from_parts(41_719_068, 0)
			.saturating_add(Weight::from_parts(0, 3791))
			// Standard Error: 1_236
			.saturating_add(Weight::from_parts(41_779, 0).saturating_mul(n.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 4).saturating_mul(n.into()))
	}
	/// Storage: `NftClaims::PendingTreeDeletions` (r:1 w:0)
	/// Proof: `NftClaims::PendingTreeDeletions` (`max_values`: Some(1), `max_size`: Some(514), added: 1009, mode: `MaxEncodedLen`)
	fn authorize_send_tree_deletions() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `545`
		//  Estimated: `1999`
		// Minimum execution time: 7_082_000 picoseconds.
		Weight::from_parts(7_630_000, 0)
			.saturating_add(Weight::from_parts(0, 1999))
			.saturating_add(T::DbWeight::get().reads(1))
	}
	/// Storage: `Scarcity::Collections` (r:1 w:0)
	/// Proof: `Scarcity::Collections` (`max_values`: None, `max_size`: Some(137), added: 2612, mode: `MaxEncodedLen`)
	/// Storage: `Revive::AccountInfoOf` (r:1 w:0)
	/// Proof: `Revive::AccountInfoOf` (`max_values`: None, `max_size`: Some(247), added: 2722, mode: `MaxEncodedLen`)
	/// Storage: `NftClaims::CollectionMinters` (r:0 w:1)
	/// Proof: `NftClaims::CollectionMinters` (`max_values`: None, `max_size`: Some(65), added: 2540, mode: `MaxEncodedLen`)
	fn set_collection_minter() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `769`
		//  Estimated: `3712`
		// Minimum execution time: 26_930_000 picoseconds.
		Weight::from_parts(28_611_000, 0)
			.saturating_add(Weight::from_parts(0, 3712))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
