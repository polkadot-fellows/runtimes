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

//! Module with configuration which reflects BridgeHubPolkadot runtime setup
//! (AccountId, Headers, Hashes...)

#![cfg_attr(not(feature = "std"), no_std)]

use bp_messages::*;
use bp_runtime::{
	decl_bridge_finality_runtime_apis, decl_bridge_messages_runtime_apis, Chain, Parachain,
};
pub use bridge_hub_common::*;
use frame_support::dispatch::DispatchClass;
use sp_runtime::RuntimeDebug;

/// BridgeHubPolkadot parachain.
#[derive(RuntimeDebug)]
pub struct BridgeHubPolkadot;

impl Chain for BridgeHubPolkadot {
	type BlockNumber = BlockNumber;
	type Hash = Hash;
	type Hasher = Hasher;
	type Header = Header;

	type AccountId = AccountId;
	type Balance = Balance;
	type Nonce = Nonce;
	type Signature = Signature;

	fn max_extrinsic_size() -> u32 {
		*BlockLength::get().max.get(DispatchClass::Normal)
	}

	fn max_extrinsic_weight() -> Weight {
		BlockWeights::get()
			.get(DispatchClass::Normal)
			.max_extrinsic
			.unwrap_or(Weight::MAX)
	}
}

impl Parachain for BridgeHubPolkadot {
	const PARACHAIN_ID: u32 = BRIDGE_HUB_POLKADOT_PARACHAIN_ID;
}

/// Identifier of BridgeHubPolkadot in the Polkadot relay chain.
pub const BRIDGE_HUB_POLKADOT_PARACHAIN_ID: u32 = 1002;

/// Name of the With-BridgeHubPolkadot messages pallet instance that is deployed at bridged chains.
pub const WITH_BRIDGE_HUB_POLKADOT_MESSAGES_PALLET_NAME: &str = "BridgePolkadotMessages";

/// Name of the With-BridgeHubPolkadot bridge-relayers pallet instance that is deployed at bridged
/// chains.
pub const WITH_BRIDGE_HUB_POLKADOT_RELAYERS_PALLET_NAME: &str = "BridgeRelayers";

/// Pallet index of `BridgeKusamaMessages: pallet_bridge_messages::<Instance1>`.
pub const WITH_BRIDGE_POLKADOT_TO_KUSAMA_MESSAGES_PALLET_INDEX: u8 = 53;

decl_bridge_finality_runtime_apis!(bridge_hub_polkadot);
decl_bridge_messages_runtime_apis!(bridge_hub_polkadot);
