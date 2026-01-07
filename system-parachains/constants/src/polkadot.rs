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

/// Universally recognized accounts.
pub mod account {
	use frame_support::PalletId;

	/// Polkadot treasury pallet id, used to convert into AccountId
	pub const POLKADOT_TREASURY_PALLET_ID: PalletId = PalletId(*b"py/trsry");
	/// Alliance pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const ALLIANCE_PALLET_ID: PalletId = PalletId(*b"py/allia");
	/// Referenda pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const REFERENDA_PALLET_ID: PalletId = PalletId(*b"py/refer");
	/// Ambassador Referenda pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const AMBASSADOR_REFERENDA_PALLET_ID: PalletId = PalletId(*b"py/amref");
	/// Identity pallet ID.
	/// Used as a temporary place to deposit a slashed imbalance before teleporting to the Treasury.
	pub const IDENTITY_PALLET_ID: PalletId = PalletId(*b"py/ident");
	/// Fellowship treasury pallet ID
	pub const FELLOWSHIP_TREASURY_PALLET_ID: PalletId = PalletId(*b"py/feltr");
	/// Ambassador treasury pallet ID
	pub const AMBASSADOR_TREASURY_PALLET_ID: PalletId = PalletId(*b"py/ambtr");
}

/// Consensus-related.
pub mod consensus {
	/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included
	/// into the relay chain.
	pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 1;
	/// How many parachain blocks are processed by the relay chain per parent. Limits the
	/// number of blocks authored per slot.
	pub const BLOCK_PROCESSING_VELOCITY: u32 = 1;
	/// Relay chain slot duration, in milliseconds.
	pub const RELAY_CHAIN_SLOT_DURATION_MILLIS: u32 = 6000;

	/// Parameters enabling async backing functionality.
	///
	/// Once all system chains have migrated to the new async backing mechanism, the parameters
	/// in this namespace will replace those currently defined in `super::*`.
	pub mod async_backing {
		/// Maximum number of blocks simultaneously accepted by the Runtime, not yet included into
		/// the relay chain.
		pub const UNINCLUDED_SEGMENT_CAPACITY: u32 = 3;
	}
}

/// Constants relating to DOT.
pub mod currency {
	use polkadot_core_primitives::Balance;

	/// The default existential deposit for system chains. 1/10th of the Relay Chain's existential
	/// deposit. Individual system parachains may modify this in special cases.
	pub const SYSTEM_PARA_EXISTENTIAL_DEPOSIT: Balance =
		polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT / 10;

	/// One "DOT" that a UI would show a user.
	pub const UNITS: Balance = 10_000_000_000;
	pub const DOLLARS: Balance = UNITS; // 10_000_000_000
	pub const GRAND: Balance = DOLLARS * 1_000; // 10_000_000_000_000
	pub const CENTS: Balance = DOLLARS / 100; // 100_000_000
	pub const MILLICENTS: Balance = CENTS / 1_000; // 100_000

	/// Deposit rate for stored data. 1/100th of the Relay Chain's deposit rate. `items` is the
	/// number of keys in storage and `bytes` is the size of the value.
	pub const fn system_para_deposit(items: u32, bytes: u32) -> Balance {
		polkadot_runtime_constants::currency::deposit(items, bytes) / 100
	}
}

/// Constants related to Polkadot fee payment.
pub mod fee {
	use frame_support::weights::constants::ExtrinsicBaseWeight;
	use polkadot_core_primitives::Balance;
	pub use sp_runtime::Perbill;

	/// The block saturation level. Fees will be updates based on this value.
	pub const TARGET_BLOCK_FULLNESS: Perbill = Perbill::from_percent(25);

	/// Cost of every transaction byte at Polkadot system parachains.
	///
	/// It is the Relay Chain (Polkadot) `TransactionByteFee` / 20.
	pub const TRANSACTION_BYTE_FEE: Balance = super::currency::MILLICENTS / 2;

	/// The two generic parameters of `BlockRatioFee` define a rational number that defines the
	/// ref_time to fee mapping. The numbers chosen here are exactly the same as the one from the
	/// `WeightToFeePolynomial` that was used before.
	pub type WeightToFee<Runtime> = pallet_revive::evm::fees::BlockRatioFee<
		{ super::currency::CENTS },
		{ (200 * ExtrinsicBaseWeight::get().ref_time()) as u128 },
		Runtime,
		Balance,
	>;
}

pub mod locations {
	use frame_support::{parameter_types, traits::Contains};
	use xcm::latest::prelude::{Junction::*, Location, NetworkId};

	parameter_types! {
		pub RelayChainLocation: Location = Location::parent();
		pub AssetHubLocation: Location =
			Location::new(1, Parachain(polkadot_runtime_constants::system_parachain::ASSET_HUB_ID));
		pub PeopleLocation: Location =
			Location::new(1, Parachain(polkadot_runtime_constants::system_parachain::PEOPLE_ID));

		pub GovernanceLocation: Location = Location::parent();

		pub EthereumNetwork: NetworkId = NetworkId::Ethereum { chain_id: 1 };
	}

	/// `Contains` implementation for the asset hub location pluralities.
	pub struct AssetHubPlurality;
	impl Contains<Location> for AssetHubPlurality {
		fn contains(loc: &Location) -> bool {
			matches!(
				loc.unpack(),
				(
					1,
					[
						Parachain(polkadot_runtime_constants::system_parachain::ASSET_HUB_ID),
						Plurality { .. }
					]
				)
			)
		}
	}
}
