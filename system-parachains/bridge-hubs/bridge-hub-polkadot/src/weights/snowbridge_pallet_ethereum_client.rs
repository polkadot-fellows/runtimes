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

//! Autogenerated weights for `snowbridge_pallet_ethereum_client`
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
// --pallet=snowbridge_pallet_ethereum_client
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

/// Weight functions for `snowbridge_pallet_ethereum_client`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> snowbridge_pallet_ethereum_client::WeightInfo for WeightInfo<T> {
	/// Storage: `EthereumBeaconClient::FinalizedBeaconStateIndex` (r:1 w:1)
	/// Proof: `EthereumBeaconClient::FinalizedBeaconStateIndex` (`max_values`: Some(1), `max_size`: Some(4), added: 499, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::FinalizedBeaconStateMapping` (r:1 w:1)
	/// Proof: `EthereumBeaconClient::FinalizedBeaconStateMapping` (`max_values`: None, `max_size`: Some(36), added: 2511, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::NextSyncCommittee` (r:0 w:1)
	/// Proof: `EthereumBeaconClient::NextSyncCommittee` (`max_values`: Some(1), `max_size`: Some(92372), added: 92867, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::InitialCheckpointRoot` (r:0 w:1)
	/// Proof: `EthereumBeaconClient::InitialCheckpointRoot` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::ValidatorsRoot` (r:0 w:1)
	/// Proof: `EthereumBeaconClient::ValidatorsRoot` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::LatestFinalizedBlockRoot` (r:0 w:1)
	/// Proof: `EthereumBeaconClient::LatestFinalizedBlockRoot` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::CurrentSyncCommittee` (r:0 w:1)
	/// Proof: `EthereumBeaconClient::CurrentSyncCommittee` (`max_values`: Some(1), `max_size`: Some(92372), added: 92867, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::FinalizedBeaconState` (r:0 w:1)
	/// Proof: `EthereumBeaconClient::FinalizedBeaconState` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	fn force_checkpoint() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `4`
		//  Estimated: `3501`
		// Minimum execution time: 123_696_664_000 picoseconds.
		Weight::from_parts(124_154_578_000, 0)
			.saturating_add(Weight::from_parts(0, 3501))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(8))
	}
	/// Storage: `EthereumBeaconClient::OperatingMode` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::OperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::LatestFinalizedBlockRoot` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::LatestFinalizedBlockRoot` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::FinalizedBeaconState` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::FinalizedBeaconState` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::NextSyncCommittee` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::NextSyncCommittee` (`max_values`: Some(1), `max_size`: Some(92372), added: 92867, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::CurrentSyncCommittee` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::CurrentSyncCommittee` (`max_values`: Some(1), `max_size`: Some(92372), added: 92867, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::ValidatorsRoot` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::ValidatorsRoot` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::LatestSyncCommitteeUpdatePeriod` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::LatestSyncCommitteeUpdatePeriod` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn submit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `92700`
		//  Estimated: `93857`
		// Minimum execution time: 32_452_194_000 picoseconds.
		Weight::from_parts(32_576_714_000, 0)
			.saturating_add(Weight::from_parts(0, 93857))
			.saturating_add(T::DbWeight::get().reads(7))
	}
	/// Storage: `EthereumBeaconClient::OperatingMode` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::OperatingMode` (`max_values`: Some(1), `max_size`: Some(1), added: 496, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::LatestFinalizedBlockRoot` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::LatestFinalizedBlockRoot` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::FinalizedBeaconState` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::FinalizedBeaconState` (`max_values`: None, `max_size`: Some(72), added: 2547, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::NextSyncCommittee` (r:1 w:1)
	/// Proof: `EthereumBeaconClient::NextSyncCommittee` (`max_values`: Some(1), `max_size`: Some(92372), added: 92867, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::CurrentSyncCommittee` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::CurrentSyncCommittee` (`max_values`: Some(1), `max_size`: Some(92372), added: 92867, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::ValidatorsRoot` (r:1 w:0)
	/// Proof: `EthereumBeaconClient::ValidatorsRoot` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	/// Storage: `EthereumBeaconClient::LatestSyncCommitteeUpdatePeriod` (r:1 w:1)
	/// Proof: `EthereumBeaconClient::LatestSyncCommitteeUpdatePeriod` (`max_values`: Some(1), `max_size`: Some(8), added: 503, mode: `MaxEncodedLen`)
	fn submit_with_sync_committee() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `92700`
		//  Estimated: `93857`
		// Minimum execution time: 156_547_111_000 picoseconds.
		Weight::from_parts(156_887_733_000, 0)
			.saturating_add(Weight::from_parts(0, 93857))
			.saturating_add(T::DbWeight::get().reads(7))
			.saturating_add(T::DbWeight::get().writes(2))
	}
}
