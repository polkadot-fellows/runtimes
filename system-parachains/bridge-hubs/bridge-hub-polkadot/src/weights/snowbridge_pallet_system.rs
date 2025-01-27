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

//! Autogenerated weights for `snowbridge_pallet_system`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 32.0.0
//! DATE: 2025-01-05, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ggwpez-ref-hw`, CPU: `AMD EPYC 7232P 8-Core Processor`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("./bridge-hub-polkadot-chain-spec.json")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polkadot-parachain
// benchmark
// pallet
// --chain=./bridge-hub-polkadot-chain-spec.json
// --steps=50
// --repeat=20
// --pallet=snowbridge_pallet_system
// --extrinsic=*
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./bridge-hub-polkadot-weights/
// --header=./file_header.txt

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `snowbridge_pallet_system`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> snowbridge_pallet_system::WeightInfo for WeightInfo<T> {
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:1)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn upgrade() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3601`
		// Minimum execution time: 46_281_000 picoseconds.
		Weight::from_parts(47_110_000, 0)
			.saturating_add(Weight::from_parts(0, 3601))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:1)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn set_operating_mode() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3601`
		// Minimum execution time: 36_331_000 picoseconds.
		Weight::from_parts(36_670_000, 0)
			.saturating_add(Weight::from_parts(0, 3601))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:1)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:0 w:1)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	fn set_pricing_parameters() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3601`
		// Minimum execution time: 42_511_000 picoseconds.
		Weight::from_parts(43_050_000, 0)
			.saturating_add(Weight::from_parts(0, 3601))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `EthereumSystem::Agents` (r:1 w:1)
	/// Proof: `EthereumSystem::Agents` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `EthereumOutboundQueue::OperatingMode` (r:1 w:0)
	/// Proof: `EthereumOutboundQueue::OperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:1)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn create_agent() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `484`
		//  Estimated: `6196`
		// Minimum execution time: 91_490_000 picoseconds.
		Weight::from_parts(92_381_000, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: `EthereumSystem::Agents` (r:1 w:0)
	/// Proof: `EthereumSystem::Agents` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::Channels` (r:2 w:1)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `EthereumOutboundQueue::OperatingMode` (r:1 w:0)
	/// Proof: `EthereumOutboundQueue::OperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:1 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn create_channel() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `929`
		//  Estimated: `69078`
		// Minimum execution time: 110_640_000 picoseconds.
		Weight::from_parts(111_820_000, 0)
			.saturating_add(Weight::from_parts(0, 69078))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `EthereumOutboundQueue::OperatingMode` (r:1 w:0)
	/// Proof: `EthereumOutboundQueue::OperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:2 w:2)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:0)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn update_channel() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `715`
		//  Estimated: `6212`
		// Minimum execution time: 104_901_000 picoseconds.
		Weight::from_parts(105_931_000, 0)
			.saturating_add(Weight::from_parts(0, 6212))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `EthereumSystem::Channels` (r:2 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:2 w:2)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:0)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn force_update_channel() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `547`
		//  Estimated: `6212`
		// Minimum execution time: 54_080_000 picoseconds.
		Weight::from_parts(54_550_000, 0)
			.saturating_add(Weight::from_parts(0, 6212))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `EthereumSystem::Agents` (r:1 w:0)
	/// Proof: `EthereumSystem::Agents` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `EthereumOutboundQueue::OperatingMode` (r:1 w:0)
	/// Proof: `EthereumOutboundQueue::OperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:2 w:2)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:0)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn transfer_native_from_agent() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `814`
		//  Estimated: `6212`
		// Minimum execution time: 110_881_000 picoseconds.
		Weight::from_parts(111_531_000, 0)
			.saturating_add(Weight::from_parts(0, 6212))
			.saturating_add(T::DbWeight::get().reads(9))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: `EthereumSystem::Agents` (r:1 w:0)
	/// Proof: `EthereumSystem::Agents` (`max_values`: None, `max_size`: Some(40), added: 2515, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:2 w:2)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:0)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn force_transfer_native_from_agent() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `569`
		//  Estimated: `6212`
		// Minimum execution time: 60_350_000 picoseconds.
		Weight::from_parts(61_010_000, 0)
			.saturating_add(Weight::from_parts(0, 6212))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:1)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	fn set_token_transfer_fees() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `218`
		//  Estimated: `3601`
		// Minimum execution time: 38_510_000 picoseconds.
		Weight::from_parts(39_180_000, 0)
			.saturating_add(Weight::from_parts(0, 3601))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `ParachainInfo::ParachainId` (r:1 w:0)
	/// Proof: `ParachainInfo::ParachainId` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::ForeignToNativeId` (r:1 w:1)
	/// Proof: `EthereumSystem::ForeignToNativeId` (`max_values`: None, `max_size`: Some(650), added: 3125, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::Channels` (r:1 w:0)
	/// Proof: `EthereumSystem::Channels` (`max_values`: None, `max_size`: Some(76), added: 2551, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::PricingParameters` (r:1 w:0)
	/// Proof: `EthereumSystem::PricingParameters` (`max_values`: Some(1), `max_size`: Some(112), added: 607, mode: `MaxEncodedLen`)
	/// Storage: `EthereumOutboundQueue::OperatingMode` (r:1 w:0)
	/// Proof: `EthereumOutboundQueue::OperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::BookStateFor` (r:1 w:1)
	/// Proof: `MessageQueue::BookStateFor` (`max_values`: None, `max_size`: Some(136), added: 2611, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::ServiceHead` (r:1 w:1)
	/// Proof: `MessageQueue::ServiceHead` (`max_values`: Some(1), `max_size`: Some(33), added: 528, mode: `MaxEncodedLen`)
	/// Storage: `MessageQueue::Pages` (r:0 w:1)
	/// Proof: `MessageQueue::Pages` (`max_values`: None, `max_size`: Some(65613), added: 68088, mode: `MaxEncodedLen`)
	/// Storage: `EthereumSystem::NativeToForeignId` (r:0 w:1)
	/// Proof: `EthereumSystem::NativeToForeignId` (`max_values`: None, `max_size`: Some(650), added: 3125, mode: `MaxEncodedLen`)
	fn register_token() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `256`
		//  Estimated: `4115`
		// Minimum execution time: 61_610_000 picoseconds.
		Weight::from_parts(62_461_000, 0)
			.saturating_add(Weight::from_parts(0, 4115))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(5))
	}
}
