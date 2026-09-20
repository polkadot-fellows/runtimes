// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Snowbridge constants for the Ethereum Main network. The SDK's helpers carry the Sepolia
//! addresses instead.

use bp_bridge_hub_polkadot::snowbridge::EthereumNetwork;
use frame_support::parameter_types;
use hex_literal::hex;
use xcm::latest::prelude::*;

/// Weth (Wrapped Ether) contract address on Ethereum mainnet.
pub const WETH: [u8; 20] = hex!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
/// The minimum Ether balance required for an account to exist. Matches value on Polkadot mainnet.
pub const MIN_ETHER_BALANCE: u128 = 15_000_000_000_000;

parameter_types! {
	pub WethLocation: Location = Location::new(2, [GlobalConsensus(EthereumNetwork::get()), AccountKey20 { network: None, key: WETH }]);
}
