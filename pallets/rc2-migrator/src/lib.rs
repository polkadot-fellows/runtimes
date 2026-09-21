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

//! Relay-chain side of the registrar + HRMP migration to the Coretime chain.
//!
//! Drives the migration stage machine: drains legacy `paras_registrar` and `hrmp` state together
//! with their deposits and sends everything to the counterpart `pallet-ct-migrator` over XCM.
//! Temporary pallet; removed once the migration is complete.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::BlockNumberFor;
use polkadot_parachain_primitives::primitives::{HrmpChannelId, Id as ParaId};
use sp_runtime::AccountId32;

pub type MigrationStageOf<T> = MigrationStage<BlockNumberFor<T>>;

/// Total balance kept on the relay chain and total migrated, by destination.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Clone,
	Copy,
	Default,
	PartialEq,
	Eq,
	Debug,
	TypeInfo,
	MaxEncodedLen,
)]
pub struct MigratedBalances {
	/// Balance that remains on the relay chain.
	pub kept: u128,
	/// Deposits burned here and re-established as holds on the Coretime chain.
	pub ct_reserved: u128,
	/// Free working buffer burned here and minted liquid on the Coretime chain.
	pub ct_free: u128,
	/// Free balance burned here and teleported to Asset Hub.
	pub ah_free: u128,
	/// Phantom issuance burned by the `TiCorrection` stage (issuance no account held).
	pub ti_corrected: u128,
}

/// Progress of the migration. Advanced by `on_initialize`.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Default, PartialEq, Eq, Debug, TypeInfo)]
pub enum MigrationStage<BlockNumber> {
	#[default]
	Pending,
	Scheduled {
		start: BlockNumber,
	},
	Paused,
	/// Waiting for the Coretime chain to confirm that it is ready to receive data.
	WaitingForCt,
	RegistrarInit,
	RegistrarOngoing {
		last_key: Option<ParaId>,
	},
	RegistrarDone,
	HrmpInit,
	HrmpOngoing {
		last_key: Option<HrmpChannelId>,
	},
	HrmpDone,
	/// All data sent; waiting for manual verification before finishing.
	CoolOff {
		end_at: BlockNumber,
	},
	MigrationDone,
}

impl<BlockNumber> MigrationStage<BlockNumber> {
	pub fn is_finished(&self) -> bool {
		matches!(self, Self::MigrationDone)
	}

	pub fn is_ongoing(&self) -> bool {
		!matches!(self, Self::Pending | Self::Scheduled { .. } | Self::MigrationDone)
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// Bound to `pallet_balances` rather than the fungible traits because the accounts stage
	/// edits `frame_system::Account` and `pallet_balances::TotalIssuance` directly: a migrating
	/// account is burned whole, including one that other pallets still reference, and no
	/// fungible API allows that.
	#[pallet::config]
	pub trait Config:
		frame_system::Config<
			AccountId = AccountId32,
			AccountData = pallet_balances::AccountData<u128>,
		> + pallet_balances::Config<Balance = u128>
	{
		/// The overarching event type.
		#[allow(deprecated)]
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::unbounded]
	pub type RcMigrationStage<T: Config> = StorageValue<_, MigrationStageOf<T>, ValueQuery>;

	/// Balance kept on the relay chain versus migrated away. Set up by the accounts stage.
	#[pallet::storage]
	pub type RcMigratedBalance<T: Config> = StorageValue<_, MigratedBalances, ValueQuery>;

	#[pallet::event]
	pub enum Event<T: Config> {
		StageTransition { old: MigrationStageOf<T>, new: MigrationStageOf<T> },
	}
}
