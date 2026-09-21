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
	traits::{
		fungible::{Inspect, Mutate, MutateHold, Unbalanced, UnbalancedHold},
		tokens::{Fortitude, Precision, Preservation},
	},
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
		///
		/// The `From<PortableHoldReason>` bound is where the runtime declares what each migrated
		/// Relay Chain hold becomes locally.
		type RuntimeHoldReason: From<HoldReason> + From<PortableHoldReason>;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		/// Registrar or HRMP deposit that was reserved on the Relay Chain.
		///
		/// Held under this reason until the owning pallet receives its state and takes its own
		/// deposit out of it.
		#[codec(index = 0)]
		RcMigratedReserve,
		/// A Relay Chain proxy deposit whose definitions travel here. Released when they arrive:
		/// the recreated entry is re-reserved at this chain's rates and the rest becomes free.
		#[codec(index = 1)]
		ProxyDeposit,
		/// Relay Chain reserve that no pallet's deposit records accounted for. Parked here for
		/// investigation. Nothing was allowed to stay behind on the Relay Chain.
		#[codec(index = 2)]
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

// TODO(ahm-v2): the helpers below have no caller and no test until the accounts stage lands.
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

	/// Move `amount` of `who`'s free balance under `reason`.
	///
	/// Not `MutateHold::hold`: that reduces the free balance first and books the hold second,
	/// and pallet-balances reaps an account that is below the ED with nothing on hold, burning
	/// the remainder. Deposit holders arrive here with sub-ED free balance next to their
	/// deposit, so the hold is booked first. Total issuance is unchanged, as with `hold`.
	pub fn place_hold(
		reason: &T::RuntimeHoldReason,
		who: &T::AccountId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		T::Currency::increase_balance_on_hold(reason, who, amount, Precision::Exact)?;
		T::Currency::decrease_balance(
			who,
			amount,
			Precision::Exact,
			Preservation::Expendable,
			Fortitude::Force,
		)?;
		Ok(())
	}

	/// Move `amount` from `who`'s hold under `reason` back to free balance.
	///
	/// Not `MutateHold::release`, for the mirror reason of [`Self::place_hold`]: it reduces the
	/// hold first, and the account would be reaped while below the ED with nothing on hold. The
	/// free balance is credited first. Total issuance is unchanged, as with `release`.
	pub fn release_hold(
		reason: &T::RuntimeHoldReason,
		who: &T::AccountId,
		amount: BalanceOf<T>,
	) -> Result<(), DispatchError> {
		T::Currency::increase_balance(who, amount, Precision::Exact)?;
		T::Currency::decrease_balance_on_hold(reason, who, amount, Precision::Exact)?;
		Ok(())
	}
}
