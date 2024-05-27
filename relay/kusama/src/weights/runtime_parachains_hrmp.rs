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
//! Autogenerated weights for `runtime_parachains::hrmp`
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
// --pallet=runtime_parachains::hrmp
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
		//  Measured:  `421`
		//  Estimated: `3886`
		// Minimum execution time: 34_128_000 picoseconds.
		Weight::from_parts(35_256_000, 0)
			.saturating_add(Weight::from_parts(0, 3886))
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
		//  Measured:  `411`
		//  Estimated: `3876`
		// Minimum execution time: 30_651_000 picoseconds.
		Weight::from_parts(31_490_000, 0)
			.saturating_add(Weight::from_parts(0, 3876))
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
		//  Measured:  `524`
		//  Estimated: `3989`
		// Minimum execution time: 31_244_000 picoseconds.
		Weight::from_parts(32_392_000, 0)
			.saturating_add(Weight::from_parts(0, 3989))
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
		//  Measured:  `231 + e * (100 ±0) + i * (100 ±0)`
		//  Estimated: `3693 + e * (2575 ±0) + i * (2575 ±0)`
		// Minimum execution time: 1_177_153_000 picoseconds.
		Weight::from_parts(1_192_970_000, 0)
			.saturating_add(Weight::from_parts(0, 3693))
			// Standard Error: 106_710
			.saturating_add(Weight::from_parts(3_503_154, 0).saturating_mul(i.into()))
			// Standard Error: 106_710
			.saturating_add(Weight::from_parts(3_480_995, 0).saturating_mul(e.into()))
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
		//  Measured:  `459 + c * (136 ±0)`
		//  Estimated: `1914 + c * (5086 ±0)`
		// Minimum execution time: 5_963_000 picoseconds.
		Weight::from_parts(5_228_724, 0)
			.saturating_add(Weight::from_parts(0, 1914))
			// Standard Error: 15_470
			.saturating_add(Weight::from_parts(20_514_191, 0).saturating_mul(c.into()))
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
		//  Measured:  `302 + c * (124 ±0)`
		//  Estimated: `1762 + c * (2600 ±0)`
		// Minimum execution time: 4_969_000 picoseconds.
		Weight::from_parts(7_208_744, 0)
			.saturating_add(Weight::from_parts(0, 1762))
			// Standard Error: 10_032
			.saturating_add(Weight::from_parts(12_527_166, 0).saturating_mul(c.into()))
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
		//  Measured:  `993 + c * (13 ±0)`
		//  Estimated: `4262 + c * (15 ±0)`
		// Minimum execution time: 18_800_000 picoseconds.
		Weight::from_parts(27_965_586, 0)
			.saturating_add(Weight::from_parts(0, 4262))
			// Standard Error: 2_135
			.saturating_add(Weight::from_parts(84_530, 0).saturating_mul(c.into()))
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
		//  Measured:  `210 + c * (63 ±0)`
		//  Estimated: `1689 + c * (2538 ±0)`
		// Minimum execution time: 3_642_000 picoseconds.
		Weight::from_parts(6_869_380, 0)
			.saturating_add(Weight::from_parts(0, 1689))
			// Standard Error: 3_822
			.saturating_add(Weight::from_parts(3_165_608, 0).saturating_mul(c.into()))
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
	/// The range of component `c` is `[0, 1]`.
	fn force_open_hrmp_channel(c: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `421 + c * (235 ±0)`
		//  Estimated: `6361 + c * (235 ±0)`
		// Minimum execution time: 48_286_000 picoseconds.
		Weight::from_parts(50_472_132, 0)
			.saturating_add(Weight::from_parts(0, 6361))
			// Standard Error: 139_702
			.saturating_add(Weight::from_parts(10_844_067, 0).saturating_mul(c.into()))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(8))
			.saturating_add(Weight::from_parts(0, 235).saturating_mul(c.into()))
	}
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
	/// Storage: `Dmp::DownwardMessageQueues` (r:2 w:2)
	/// Proof: `Dmp::DownwardMessageQueues` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Dmp::DownwardMessageQueueHeads` (r:2 w:2)
	/// Proof: `Dmp::DownwardMessageQueueHeads` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpIngressChannelsIndex` (r:1 w:0)
	/// Proof: `Hrmp::HrmpIngressChannelsIndex` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Hrmp::HrmpAcceptedChannelRequestCount` (r:1 w:1)
	/// Proof: `Hrmp::HrmpAcceptedChannelRequestCount` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn establish_system_channel() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `421`
		//  Estimated: `6361`
		// Minimum execution time: 48_480_000 picoseconds.
		Weight::from_parts(49_938_000, 0)
			.saturating_add(Weight::from_parts(0, 6361))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `Hrmp::HrmpChannels` (r:1 w:1)
	/// Proof: `Hrmp::HrmpChannels` (`max_values`: None, `max_size`: None, mode: `Measured`)
	fn poke_channel_deposits() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `230`
		//  Estimated: `3695`
		// Minimum execution time: 11_525_000 picoseconds.
		Weight::from_parts(12_232_000, 0)
			.saturating_add(Weight::from_parts(0, 3695))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	fn establish_channel_with_system() -> Weight {
		// TODO: regenerate weights
		// Proof Size summary in bytes:
		//  Measured:  `417`
		//  Estimated: `6357`
		// Minimum execution time: 629_674_000 picoseconds.
		Weight::from_parts(640_174_000, 0)
			.saturating_add(Weight::from_parts(0, 6357))
			.saturating_add(T::DbWeight::get().reads(12))
			.saturating_add(T::DbWeight::get().writes(8))
	}
}
