// This file is part of Substrate.

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

//! The operational pallet for the Relay Chain, designed to manage and facilitate the migration of
//! subsystems such as Governance, Staking, Balances from the Relay Chain to the Asset Hub. This
//! pallet works alongside its counterpart, `pallet_ah_migrator`, which handles migration
//! processes on the Asset Hub side.
//!
//! This pallet is responsible for controlling the initiation, progression, and completion of the
//! migration process, including managing its various stages and transferring the necessary data.
//! The pallet directly accesses the storage of other pallets for read/write operations while
//! maintaining compatibility with their existing APIs.
//!
//! To simplify development and avoid the need to edit the original pallets, this pallet may
//! duplicate private items such as storage entries from the original pallets. This ensures that the
//! migration logic can be implemented without altering the original implementations.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{InspectFreeze, Mutate, MutateFreeze, MutateHold},
		LockableCurrency, ReservableCurrency,
	},
};
use pallet_balances::AccountData;
use sp_runtime::AccountId32;
use sp_std::prelude::*;

/// The log target of this pallet.
pub const LOG_TARGET: &str = "runtime::rc-migrator";

/// Soft limit on the DMP message size.
///
/// The hard limit should be about 64KiB which means that we stay well below
/// that to avoid any trouble. We can raise this as final preparation for the migration once
/// everything is confirmed to work.
pub const MAX_XCM_SIZE: u32 = 50_000;

/// The maximum number of items that can be migrated in a single block.
///
/// This serves as an additional safety limit beyond the weight accounting of both the Relay Chain
/// and Asset Hub.
pub const MAX_ITEMS_PER_BLOCK: u32 = 1600;

/// The maximum number of XCM messages that can be sent in a single block.
pub const MAX_XCM_MSG_PER_BLOCK: u32 = 10;

/// The state for the Relay Chain accounts.
#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum AccountState<Balance> {
	/// The account should be migrated to AH and removed on RC.
	Migrate,

	/// The account must stay on RC with its balance.
	///
	/// E.g., RC system account.
	Preserve,

	// We might not need the `Part` variation since there are no many cases for `Part` we can just
	// keep the whole account balance on RC
	/// The part of the account must be preserved on RC.
	///
	/// Cases:
	/// - accounts placed deposit for parachain registration (paras_registrar pallet);
	/// - accounts placed deposit for hrmp channel registration (parachains_hrmp pallet);
	/// - accounts storing the keys within the session pallet with a consumer reference.
	Part {
		/// The free balance that must be preserved on RC.
		///
		/// Includes ED.
		free: Balance,
		/// The reserved balance that should be must be preserved on RC.
		///
		/// In practice reserved by old `Currency` api and has no associated reason.
		reserved: Balance,
		/// The number of consumers that must be preserved on RC.
		///
		/// Generally one consumer reference of reserved balance or/and consumer reference of the
		/// session pallet.
		consumers: u32,
	},
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Super config trait for all pallets that the migration depends on, providing convenient
	/// access to their items.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<AccountData = AccountData<u128>, AccountId = AccountId32, Nonce = u32>
	{
		/// The overall runtime origin type.
		type RuntimeOrigin: Into<Result<pallet_xcm::Origin, <Self as Config>::RuntimeOrigin>>
			+ IsType<<Self as frame_system::Config>::RuntimeOrigin>
			+ From<frame_system::RawOrigin<Self::AccountId>>;

		/// Native asset registry type.
		type Currency: Mutate<Self::AccountId, Balance = u128>
			+ MutateHold<Self::AccountId>
			+ InspectFreeze<Self::AccountId>
			+ MutateFreeze<Self::AccountId>
			+ ReservableCurrency<Self::AccountId, Balance = u128>
			+ LockableCurrency<Self::AccountId, Balance = u128>;
	}

	#[pallet::error]
	pub enum Error<T> {}

	/// Helper storage item to obtain and store the known accounts that should be kept partially or
	/// fully on Relay Chain.
	#[pallet::storage]
	pub type RcAccounts<T: Config> =
		CountedStorageMap<_, Twox64Concat, T::AccountId, AccountState<u128>, OptionQuery>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);
}
