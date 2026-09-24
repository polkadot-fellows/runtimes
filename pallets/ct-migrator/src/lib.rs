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

//! Coretime-chain side of the registrar + HRMP migration.
//!
//! Ingests state sent by `pallet-rc2-migrator`, writing through the same code path as fresh
//! registrations so that migrated and newly created state are identical. Temporary pallet;
//! removed once the migration is complete.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub use pallet::*;

use alloc::vec::Vec;
use frame_support::{
	pallet_prelude::*,
	storage::with_storage_layer,
	traits::fungible::{Inspect, Mutate, MutateHold},
};
use migrator_types::PortableHoldReason;
use sp_runtime::DispatchError;

pub type BalanceOf<T> =
	<<T as Config>::Currency as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

/// Progress of the migration. Advanced by messages from `pallet-rc2-migrator`.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Default,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub enum MigrationStage {
	#[default]
	Pending,
	DataMigrationOngoing,
	MigrationDone,
}

impl MigrationStage {
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	pub fn is_ongoing(&self) -> bool {
		matches!(self, Self::DataMigrationOngoing)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Native currency. Migrated balances are minted here; migrated reserves land as holds.
		type Currency: Mutate<Self::AccountId>
			+ MutateHold<Self::AccountId, Reason = Self::RuntimeHoldReason>;

		/// The overarching hold reason type.
		type RuntimeHoldReason: From<HoldReason>;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// A Relay Chain para registration deposit. Held until the para record arrives and the
		/// registrar pallet takes its own deposit out of it.
		#[codec(index = 0)]
		RegistrarDeposit,
		/// A Relay Chain HRMP channel deposit. Held until the channel record arrives and the
		/// HRMP pallet takes its own deposit out of it.
		#[codec(index = 1)]
		HrmpDeposit,
		/// A Relay Chain proxy deposit whose definitions travel here. Released when they arrive:
		/// the recreated entry is re-reserved at this chain's rates and the rest becomes free.
		#[codec(index = 2)]
		ProxyDeposit,
		/// Relay Chain reserve that no pallet's deposit records accounted for. Parked here for
		/// investigation. Nothing was allowed to stay behind on the Relay Chain.
		#[codec(index = 3)]
		UnattributedReserve,
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub type CtMigrationStage<T: Config> = StorageValue<_, MigrationStage, ValueQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {
		StageTransition { old: MigrationStage, new: MigrationStage },
	}
}

/// What each hold migrated from the Relay Chain becomes on this chain.
impl From<PortableHoldReason> for HoldReason {
	fn from(reason: PortableHoldReason) -> Self {
		match reason {
			PortableHoldReason::RegistrarDeposit => HoldReason::RegistrarDeposit,
			PortableHoldReason::HrmpDeposit => HoldReason::HrmpDeposit,
			PortableHoldReason::ProxyDeposit => HoldReason::ProxyDeposit,
			PortableHoldReason::UnattributedReserve => HoldReason::UnattributedReserve,
		}
	}
}

// TODO(ahm-v2): the helper below has no caller and no test until the accounts stage lands.
impl<T: Config> Pallet<T> {
	/// Run `integrate` over every item in its own storage transaction. A failing item is rolled
	/// back and handed to `park`; the other items are unaffected. Returns `(count_good,
	/// count_bad)`.
	pub fn receive_batch<I, R>(
		items: Vec<I>,
		integrate: impl Fn(&I) -> Result<R, DispatchError>,
		mut on_good: impl FnMut(R),
		park: impl Fn(I, DispatchError),
	) -> (u32, u32) {
		let (mut count_good, mut count_bad) = (0, 0);
		for item in items {
			match with_storage_layer(|| integrate(&item)) {
				Ok(r) => {
					count_good += 1;
					on_good(r);
				},
				Err(e) => {
					count_bad += 1;
					park(item, e);
				},
			}
		}
		(count_good, count_bad)
	}
}
