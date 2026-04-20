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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Shim for `pallet-dap-satellite` items not yet published on crates.io.
//!
//! ---- TODO: Dev scaffolding for the py/trsry wind-down PR. The PR is NOT merged (and this code
//! NOT shipped to chains) until we bump pallet-dap-satellite to SDK 2604 release.
//! (`pallet-dap-satellite >= 0.3.0`). At bump time, delete this module and re-export from
//! `pallet_dap_satellite` directly. Upstream provides:
//! - `pallet_dap_satellite::DapSatelliteLegacyAdapter` (SDK PR #11716)
//! - `pallet_dap_satellite::migrations::DrainLegacyTreasuryToDapSatellite` (SDK PR #11763)

extern crate alloc;

use core::marker::PhantomData;
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{Balanced, Inspect},
		tokens::{Fortitude, Precision, Preservation},
		Currency, OnRuntimeUpgrade, OnUnbalanced,
	},
	PalletId,
};
use sp_runtime::traits::AccountIdConversion;

const LOG_TARGET: &str = "runtime::dap-satellite-shim";

/// Legacy treasury `PalletId` (`py/trsry`).
const LEGACY_TREASURY_PALLET_ID: PalletId = PalletId(*b"py/trsry");

/// Adapter that redirects `NegativeImbalance` from the legacy `Currency` trait to the local
/// `pallet_dap_satellite` buffer.
///
/// Mirrors upstream `pallet_dap_satellite::DapSatelliteLegacyAdapter`. Use on pallets that still
/// expose `type Slashed/Slash: OnUnbalanced<NegativeImbalance>` (e.g. `pallet_alliance`,
/// `pallet_identity`, `pallet_referenda`, `pallet_ranked_collective`).
pub struct DapSatelliteLegacyAdapter<T, C>(PhantomData<(T, C)>);

impl<T, C> OnUnbalanced<<C as Currency<T::AccountId>>::NegativeImbalance>
	for DapSatelliteLegacyAdapter<T, C>
where
	T: pallet_dap_satellite::Config,
	C: Currency<T::AccountId>,
{
	fn on_nonzero_unbalanced(amount: <C as Currency<T::AccountId>>::NegativeImbalance) {
		let satellite = pallet_dap_satellite::Pallet::<T>::satellite_account();
		C::resolve_creating(&satellite, amount);
		log::debug!(
			target: LOG_TARGET,
			"💸 Deposited (legacy) to DAP satellite"
		);
	}
}

/// Drain the reducible balance of the legacy `py/trsry`-derived account into the local DAP
/// satellite buffer.
///
/// Mirrors upstream `pallet_dap_satellite::migrations::DrainLegacyTreasuryToDapSatellite`.
/// The legacy `PalletId` is hardcoded so the migration cannot be misconfigured to drain the wrong
/// account. Idempotent: early-returns with 1 read if the reducible balance is zero.
pub struct DrainLegacyTreasuryToDapSatellite<T>(PhantomData<T>);

impl<T> OnRuntimeUpgrade for DrainLegacyTreasuryToDapSatellite<T>
where
	T: pallet_dap_satellite::Config,
	T::Currency: Balanced<T::AccountId>,
{
	fn on_runtime_upgrade() -> Weight {
		let source: T::AccountId = LEGACY_TREASURY_PALLET_ID.into_account_truncating();
		// No further inflows to the legacy account are expected, but since this migration
		// runs on every runtime upgrade we use `Preserve` as a safeguard.
		// Worst case we just keep a "dead" account with only ED.
		let amount = <T::Currency as Inspect<T::AccountId>>::reducible_balance(
			&source,
			Preservation::Preserve,
			Fortitude::Polite,
		);
		if amount.is_zero() {
			log::info!(
				target: LOG_TARGET,
				"DrainLegacyTreasuryToDapSatellite: nothing to withdraw (reducible balance is zero)."
			);
			return T::DbWeight::get().reads(1);
		}

		match <T::Currency as Balanced<T::AccountId>>::withdraw(
			&source,
			amount,
			Precision::Exact,
			Preservation::Preserve,
			Fortitude::Polite,
		) {
			Ok(credit) => {
				<pallet_dap_satellite::Pallet<T> as OnUnbalanced<_>>::on_unbalanced(credit);
				log::info!(
					target: LOG_TARGET,
					"DrainLegacyTreasuryToDapSatellite: swept {amount:?} to DAP satellite."
				);
			},
			Err(_) => {
				frame_support::defensive!(
					"DrainLegacyTreasuryToDapSatellite: failed to withdraw from legacy treasury account"
				);
			},
		}

		// Distinct storage keys touched: source Account (balances + system),
		// satellite Account (balances + system) = 4 reads and 4 writes.
		T::DbWeight::get().reads_writes(4, 4)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<alloc::vec::Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;
		let source: T::AccountId = LEGACY_TREASURY_PALLET_ID.into_account_truncating();
		let balance = <T::Currency as Inspect<T::AccountId>>::reducible_balance(
			&source,
			Preservation::Preserve,
			Fortitude::Polite,
		);
		log::info!(
			target: LOG_TARGET,
			"DrainLegacyTreasuryToDapSatellite: pre-upgrade reducible balance = {balance:?}"
		);
		Ok(balance.encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;

		let pre_balance: <T::Currency as Inspect<T::AccountId>>::Balance =
			Decode::decode(&mut &state[..]).expect("pre_upgrade encoded the reducible balance");
		let source: T::AccountId = LEGACY_TREASURY_PALLET_ID.into_account_truncating();
		let post_balance = <T::Currency as Inspect<T::AccountId>>::reducible_balance(
			&source,
			Preservation::Preserve,
			Fortitude::Polite,
		);
		frame_support::ensure!(
			post_balance.is_zero(),
			"Legacy treasury reducible balance should be zero after migration"
		);

		let satellite = pallet_dap_satellite::Pallet::<T>::satellite_account();
		let satellite_balance = <T::Currency as Inspect<T::AccountId>>::total_balance(&satellite);
		frame_support::ensure!(
			satellite_balance >= pre_balance,
			"DAP satellite balance should have increased by at least the drained amount"
		);

		log::info!(
			target: LOG_TARGET,
			"DrainLegacyTreasuryToDapSatellite: post-upgrade OK. \
			 Legacy treasury reducible: {post_balance:?}, satellite total: {satellite_balance:?}"
		);
		Ok(())
	}
}
