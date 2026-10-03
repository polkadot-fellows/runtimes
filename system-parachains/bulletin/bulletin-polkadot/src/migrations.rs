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

#![allow(deprecated, missing_docs)]

use super::*;
use frame_support::traits::{
	fungible::{Inspect, Mutate},
	tokens::{Fortitude, Preservation},
	OnRuntimeUpgrade,
};
use frame_system::Pallet as System;
use parachains_common::TREASURY_PALLET_ID;
use sp_runtime::traits::AccountIdConversion;

/// Unreleased migrations. Add new ones here:
pub type Unreleased = (
	// xcmp-queue storage v6 -> v7 (SDK stable2606-1).
	cumulus_pallet_xcmp_queue::migration::v7::MigrateV6ToV7<Runtime>,
	cumulus_pallet_parachain_system::migration::Migration<Runtime>,
	DrainLegacyTreasuryToAccumulation,
);

/// Sweeps the legacy `py/trsry` fee account, unspendable on this chain, into the accumulation
/// account. Idempotent: once it is gone, a rerun only reads.
pub struct DrainLegacyTreasuryToAccumulation;

impl DrainLegacyTreasuryToAccumulation {
	pub fn legacy_treasury_account() -> AccountId {
		TREASURY_PALLET_ID.into_account_truncating()
	}
}

impl OnRuntimeUpgrade for DrainLegacyTreasuryToAccumulation {
	fn on_runtime_upgrade() -> Weight {
		let legacy_account = Self::legacy_treasury_account();
		if !System::<Runtime>::account_exists(&legacy_account) {
			return <Runtime as frame_system::Config>::DbWeight::get().reads(1);
		}
		let residual = Balances::reducible_balance(
			&legacy_account,
			Preservation::Expendable,
			Fortitude::Polite,
		);
		if residual > 0 {
			if let Err(e) = Balances::transfer(
				&legacy_account,
				&AccumulateForward::accumulation_account(),
				residual,
				Preservation::Expendable,
			) {
				log::error!(
					target: "runtime::bulletin",
					"Failed to sweep {residual} from the legacy treasury account: {e:?}"
				);
			}
		}
		<Runtime as frame_system::Config>::DbWeight::get().reads_writes(6, 3)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;
		let residual = Balances::reducible_balance(
			&Self::legacy_treasury_account(),
			Preservation::Expendable,
			Fortitude::Polite,
		);
		let accumulated = Balances::total_balance(&AccumulateForward::accumulation_account());
		Ok((residual, accumulated).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;
		let (residual, accumulated_before): (Balance, Balance) =
			Decode::decode(&mut &state[..]).map_err(|_| "invalid pre_upgrade state")?;
		let legacy_account = Self::legacy_treasury_account();
		frame_support::ensure!(
			Balances::total_balance(&legacy_account) == 0,
			"legacy treasury account not swept"
		);
		frame_support::ensure!(
			!System::<Runtime>::account_exists(&legacy_account),
			"legacy treasury account not reaped"
		);
		frame_support::ensure!(
			Balances::total_balance(&AccumulateForward::accumulation_account()) ==
				accumulated_before + residual,
			"residual not moved to the accumulation account"
		);
		Ok(())
	}
}

/// Migrations/checks that do not need to be versioned and can run on every update.
pub type Permanent = (
	// Idempotent: initializes `RetentionPeriod` when zero, a no-op once set.
	pallet_bulletin_transaction_storage::migrations::SetRetentionPeriodIfZero<
		Runtime,
		pallet_bulletin_transaction_storage::DefaultRetentionPeriod,
	>,
);

/// All single block migrations that will run on the next runtime upgrade.
pub type SingleBlockMigrations = (Unreleased, Permanent);

/// MBM migrations to apply on runtime upgrade.
pub type MbmMigrations = ();
