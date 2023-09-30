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

//! Autogenerated weights for `runtime_parachains::hrmp`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-09-21, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `a3dce7bd4066`, CPU: `Intel(R) Xeon(R) CPU @ 2.60GHz`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("spec-kusama.json")`, DB CACHE: 1024

// Executed Command:
// /builds/polkadot-sdk/target/production/polkadot
// benchmark
// pallet
// --chain=spec-kusama.json
// --pallet=runtime_parachains::hrmp
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

/// Weight functions for `runtime_parachains::hrmp`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> runtime_parachains::hrmp::WeightInfo for WeightInfo<T> {
	/// Storage: `Paras::ParaLifecycles` (r:1 w:0)
	/// Proof: `Paras::ParaLifecycles` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequests` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpChannels` (r:1 w:0)
	/// Proof: `Hrmp::HrmpChannels` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpEgressChannelsIndex` (r:1 w:0)
	/// Proof: `Hrmp::HrmpEgressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequestCount` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequestsList` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestsList` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueues` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueues` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueueHeads` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueueHeads` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn hrmp_init_open_channel() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `417`
		//  Estimated: `3882`
		// Minimum execution time: 36_394_000 picoseconds.
		Weight::from_parts(37_935_000, 0)
			.saturating_add(Weight::from_parts(0, 3882))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `Hrmp::HrmpOpenChannelRequests` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpIngressChannelsIndex` (r:1 w:0)
	/// Proof: `Hrmp::HrmpIngressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpAcceptedChannelRequestCount` (r:1 w:1)
	/// Proof: `Hrmp::HrmpAcceptedChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueues` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueues` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueueHeads` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueueHeads` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn hrmp_accept_open_channel() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `407`
		//  Estimated: `3872`
		// Minimum execution time: 32_220_000 picoseconds.
		Weight::from_parts(33_205_000, 0)
			.saturating_add(Weight::from_parts(0, 3872))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Hrmp::HrmpChannels` (r:1 w:0)
	/// Proof: `Hrmp::HrmpChannels` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpCloseChannelRequests` (r:1 w:1)
	/// Proof: `Hrmp::HrmpCloseChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpCloseChannelRequestsList` (r:1 w:1)
	/// Proof: `Hrmp::HrmpCloseChannelRequestsList` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueues` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueues` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueueHeads` (r:1 w:1)
	/// Proof: `Dmp::DownwardMessageQueueHeads` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn hrmp_close_channel() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `520`
		//  Estimated: `3985`
		// Minimum execution time: 33_344_000 picoseconds.
		Weight::from_parts(34_695_000, 0)
			.saturating_add(Weight::from_parts(0, 3985))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `Hrmp::HrmpIngressChannelsIndex` (r:128 w:128)
	/// Proof: `Hrmp::HrmpIngressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpEgressChannelsIndex` (r:128 w:128)
	/// Proof: `Hrmp::HrmpEgressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpChannels` (r:254 w:254)
	/// Proof: `Hrmp::HrmpChannels` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpAcceptedChannelRequestCount` (r:0 w:1)
	/// Proof: `Hrmp::HrmpAcceptedChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpChannelContents` (r:0 w:254)
	/// Proof: `Hrmp::HrmpChannelContents` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequestCount` (r:0 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `i` is `[0, 127]`.
	/// The range of component `e` is `[0, 127]`.
	fn force_clean_hrmp(i: u32, e: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `264 + e * (100 ±0) + i * (100 ±0)`
		//  Estimated: `3726 + e * (2575 ±0) + i * (2575 ±0)`
		// Minimum execution time: 1_189_688_000 picoseconds.
		Weight::from_parts(1_200_536_000, 0)
			.saturating_add(Weight::from_parts(0, 3726))
			// Standard Error: 114_016
			.saturating_add(Weight::from_parts(3_767_693, 0).saturating_mul(i.into()))
			// Standard Error: 114_016
			.saturating_add(Weight::from_parts(3_629_627, 0).saturating_mul(e.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(i.into())))
			.saturating_add(T::DbWeight::get().reads((2_u64).saturating_mul(e.into())))
			.saturating_add(T::DbWeight::get().writes(4))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(i.into())))
			.saturating_add(T::DbWeight::get().writes((3_u64).saturating_mul(e.into())))
			.saturating_add(Weight::from_parts(0, 2575).saturating_mul(e.into()))
			.saturating_add(Weight::from_parts(0, 2575).saturating_mul(i.into()))
	}
	/// Storage: `Hrmp::HrmpOpenChannelRequestsList` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestsList` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequests` (r:128 w:128)
	/// Proof: `Hrmp::HrmpOpenChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Paras::ParaLifecycles` (r:256 w:0)
	/// Proof: `Paras::ParaLifecycles` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpIngressChannelsIndex` (r:128 w:128)
	/// Proof: `Hrmp::HrmpIngressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpEgressChannelsIndex` (r:128 w:128)
	/// Proof: `Hrmp::HrmpEgressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequestCount` (r:128 w:128)
	/// Proof: `Hrmp::HrmpOpenChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpAcceptedChannelRequestCount` (r:128 w:128)
	/// Proof: `Hrmp::HrmpAcceptedChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpChannels` (r:0 w:128)
	/// Proof: `Hrmp::HrmpChannels` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `c` is `[0, 128]`.
	fn force_process_hrmp_open(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `492 + c * (136 ±0)`
		//  Estimated: `1947 + c * (5086 ±0)`
		// Minimum execution time: 6_513_000 picoseconds.
		Weight::from_parts(6_722_000, 0)
			.saturating_add(Weight::from_parts(0, 1947))
			// Standard Error: 20_890
			.saturating_add(Weight::from_parts(20_983_900, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((7_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((6_u64).saturating_mul(c.into())))
			.saturating_add(Weight::from_parts(0, 5086).saturating_mul(c.into()))
	}
	/// Storage: `Hrmp::HrmpCloseChannelRequestsList` (r:1 w:1)
	/// Proof: `Hrmp::HrmpCloseChannelRequestsList` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpChannels` (r:128 w:128)
	/// Proof: `Hrmp::HrmpChannels` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpEgressChannelsIndex` (r:128 w:128)
	/// Proof: `Hrmp::HrmpEgressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpIngressChannelsIndex` (r:128 w:128)
	/// Proof: `Hrmp::HrmpIngressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpCloseChannelRequests` (r:0 w:128)
	/// Proof: `Hrmp::HrmpCloseChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpChannelContents` (r:0 w:128)
	/// Proof: `Hrmp::HrmpChannelContents` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `c` is `[0, 128]`.
	fn force_process_hrmp_close(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `335 + c * (124 ±0)`
		//  Estimated: `1795 + c * (2600 ±0)`
		// Minimum execution time: 5_354_000 picoseconds.
		Weight::from_parts(5_471_000, 0)
			.saturating_add(Weight::from_parts(0, 1795))
			// Standard Error: 13_793
			.saturating_add(Weight::from_parts(13_044_103, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((3_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((5_u64).saturating_mul(c.into())))
			.saturating_add(Weight::from_parts(0, 2600).saturating_mul(c.into()))
	}
	/// Storage: `Hrmp::HrmpOpenChannelRequestsList` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestsList` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequests` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequestCount` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `c` is `[0, 128]`.
	fn hrmp_cancel_open_request(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1026 + c * (13 ±0)`
		//  Estimated: `4295 + c * (15 ±0)`
		// Minimum execution time: 22_502_000 picoseconds.
		Weight::from_parts(29_670_475, 0)
			.saturating_add(Weight::from_parts(0, 4295))
			// Standard Error: 2_335
			.saturating_add(Weight::from_parts(221_707, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
			.saturating_add(Weight::from_parts(0, 15).saturating_mul(c.into()))
	}
	/// Storage: `Hrmp::HrmpOpenChannelRequestsList` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestsList` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequests` (r:128 w:128)
	/// Proof: `Hrmp::HrmpOpenChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `c` is `[0, 128]`.
	fn clean_open_channel_requests(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `243 + c * (63 ±0)`
		//  Estimated: `1722 + c * (2538 ±0)`
		// Minimum execution time: 4_278_000 picoseconds.
		Weight::from_parts(429_668, 0)
			.saturating_add(Weight::from_parts(0, 1722))
			// Standard Error: 5_652
			.saturating_add(Weight::from_parts(3_529_802, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(c.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(c.into())))
			.saturating_add(Weight::from_parts(0, 2538).saturating_mul(c.into()))
	}
	/// Storage: `Hrmp::HrmpOpenChannelRequests` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequests` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequestsList` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestsList` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpOpenChannelRequestCount` (r:1 w:1)
	/// Proof: `Hrmp::HrmpOpenChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Paras::ParaLifecycles` (r:1 w:0)
	/// Proof: `Paras::ParaLifecycles` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpChannels` (r:1 w:0)
	/// Proof: `Hrmp::HrmpChannels` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpEgressChannelsIndex` (r:1 w:0)
	/// Proof: `Hrmp::HrmpEgressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueues` (r:2 w:2)
	/// Proof: `Dmp::DownwardMessageQueues` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueueHeads` (r:2 w:2)
	/// Proof: `Dmp::DownwardMessageQueueHeads` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpIngressChannelsIndex` (r:1 w:0)
	/// Proof: `Hrmp::HrmpIngressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpAcceptedChannelRequestCount` (r:1 w:1)
	/// Proof: `Hrmp::HrmpAcceptedChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn force_open_hrmp_channel(_c: u32) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `652`
		//  Estimated: `6592`
		// Minimum execution time: 60_528_000 picoseconds.
		Weight::from_parts(64_049_000, 0)
			.saturating_add(Weight::from_parts(0, 6592))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(8))
	}
}
