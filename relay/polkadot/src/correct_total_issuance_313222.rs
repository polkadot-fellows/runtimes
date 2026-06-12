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

//! One-off migration correcting the `Balances::TotalIssuance` over-count from block `313222`.
//!
//! At [`313222-3`](https://polkadot.subscan.io/extrinsic/313222-3) a `sudo` -> `system.killStorage`
//! deleted account `11gh7WpYe5CKN82opEkrZUJsHeaFZa3pdEkEZFSMkV3k1Xj` (~20K DOT) by writing the trie
//! directly, bypassing the balances pallet, so `TotalIssuance` was never decremented and has
//! over-counted ever since ([#147](https://github.com/polkadot-fellows/runtimes/issues/147)). The
//! Fellowship measured the discrepancy at exactly `19,999.998 DOT`.
//!
//! Fix: decrease `TotalIssuance` by that amount via the audited
//! [`pallet_balances::Pallet::force_adjust_total_issuance`] (same call used for the Kusama
//! correction). Pure accounting fix — no account is credited. Idempotent via a storage marker, so a
//! double-run can't burn twice; still a one-off — remove from `migrations::Unreleased` after release.

extern crate alloc;

use crate::{Balance, Balances, Runtime};
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight};
use frame_system::RawOrigin;
use pallet_balances::AdjustmentDirection;

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::ensure;

const LOG_TARGET: &str = "runtime::correct_total_issuance_313222";

/// The over-count to remove, in plancks: `19,999.998 DOT * 10^10 = 199,999,980,000,000`.
pub const DUSTED_OVER_ISSUANCE: Balance = 199_999_980_000_000;

/// Marker key, laid out like a pallet `StorageValue` (`twox_128(prefix) ++ twox_128(name)`) under a
/// unique prefix so it can't collide with real storage.
fn applied_storage_key() -> [u8; 32] {
	let mut key = [0u8; 32];
	key[..16].copy_from_slice(&sp_io::hashing::twox_128(b"Block313222TiCorrection"));
	key[16..].copy_from_slice(&sp_io::hashing::twox_128(b"Applied"));
	key
}

/// Whether the correction has already been applied.
fn already_applied() -> bool {
	frame_support::storage::unhashed::get::<bool>(&applied_storage_key()).unwrap_or(false)
}

/// Record that the correction has been applied.
fn mark_applied() {
	frame_support::storage::unhashed::put(&applied_storage_key(), &true);
}

/// Realign `TotalIssuance` after the block `313222` `killStorage` dusting. One-off and idempotent.
pub struct CorrectBlock313222TotalIssuance;

impl OnRuntimeUpgrade for CorrectBlock313222TotalIssuance {
	fn on_runtime_upgrade() -> Weight {
		let db_weight = <Runtime as frame_system::Config>::DbWeight::get();

		if already_applied() {
			log::info!(
				target: LOG_TARGET,
				"TotalIssuance correction for block 313222 already applied; skipping",
			);
			return db_weight.reads(1);
		}

		// Root-only extrinsic: runs the `InactiveIssuance` guard and emits `TotalIssuanceForced`. On
		// error, log and leave `TotalIssuance` untouched rather than brick the upgrade.
		match Balances::force_adjust_total_issuance(
			RawOrigin::Root.into(),
			AdjustmentDirection::Decrease,
			DUSTED_OVER_ISSUANCE,
		) {
			Ok(()) => {
				mark_applied();
				log::info!(
					target: LOG_TARGET,
					"Decreased TotalIssuance by {DUSTED_OVER_ISSUANCE} plancks to correct the \
					 block 313222 killStorage dusting (issue #147)",
				);
			},
			Err(e) => {
				log::error!(
					target: LOG_TARGET,
					"Failed to decrease TotalIssuance by {DUSTED_OVER_ISSUANCE} plancks: {e:?}",
				);
			},
		}

		// Marker + `TotalIssuance`/`InactiveIssuance` reads; `TotalIssuance` + marker writes.
		db_weight.reads_writes(3, 2)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		let was_applied = already_applied();
		let issuance_before = pallet_balances::TotalIssuance::<Runtime>::get();
		Ok((was_applied, issuance_before).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;

		let (was_applied, issuance_before): (bool, Balance) =
			Decode::decode(&mut &state[..]).map_err(|_| "Failed to decode pre_upgrade state")?;

		let issuance_after = pallet_balances::TotalIssuance::<Runtime>::get();

		if was_applied {
			// Already corrected: no-op expected.
			ensure!(
				issuance_after == issuance_before,
				"TotalIssuance changed even though the correction was already applied",
			);
		} else {
			// Fresh run: TotalIssuance drops by exactly the dusted amount and the marker is set.
			ensure!(
				issuance_after == issuance_before.saturating_sub(DUSTED_OVER_ISSUANCE),
				"TotalIssuance did not decrease by exactly the dusted amount",
			);
			ensure!(already_applied(), "Applied marker was not set after the correction");
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{AccountId, System};
	use frame_support::traits::fungible::Mutate;
	use polkadot_runtime_constants::currency::UNITS;
	use sp_runtime::BuildStorage;

	/// Clean test externalities on the real runtime types.
	fn new_test_ext() -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}

	/// Mint into a throwaway account so `TotalIssuance` sits above the amount we burn.
	fn seed_issuance(amount: Balance) {
		let sink = AccountId::from([0xee; 32]);
		<Balances as Mutate<AccountId>>::mint_into(&sink, amount).unwrap();
	}

	#[test]
	fn dusted_amount_is_exactly_19_999_998_dot() {
		// Guard the constant against a typo: 19,999.998 DOT in plancks.
		assert_eq!(DUSTED_OVER_ISSUANCE, 19_999 * UNITS + 998 * (UNITS / 1_000));
		assert_eq!(DUSTED_OVER_ISSUANCE, 199_999_980_000_000);
	}

	#[test]
	fn corrects_total_issuance_once() {
		new_test_ext().execute_with(|| {
			seed_issuance(1_000_000 * UNITS);
			let before = pallet_balances::TotalIssuance::<Runtime>::get();

			CorrectBlock313222TotalIssuance::on_runtime_upgrade();

			assert_eq!(
				pallet_balances::TotalIssuance::<Runtime>::get(),
				before - DUSTED_OVER_ISSUANCE,
				"TotalIssuance must drop by exactly the dusted amount",
			);
			assert!(already_applied(), "marker must be set after the correction");
		});
	}

	#[test]
	fn is_idempotent() {
		new_test_ext().execute_with(|| {
			seed_issuance(1_000_000 * UNITS);
			let before = pallet_balances::TotalIssuance::<Runtime>::get();

			// Many runs must burn the amount only once.
			CorrectBlock313222TotalIssuance::on_runtime_upgrade();
			let after_first = pallet_balances::TotalIssuance::<Runtime>::get();
			CorrectBlock313222TotalIssuance::on_runtime_upgrade();
			CorrectBlock313222TotalIssuance::on_runtime_upgrade();

			assert_eq!(after_first, before - DUSTED_OVER_ISSUANCE);
			assert_eq!(
				pallet_balances::TotalIssuance::<Runtime>::get(),
				after_first,
				"re-running the migration must not change TotalIssuance again",
			);
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn pre_and_post_upgrade_checks_pass() {
		new_test_ext().execute_with(|| {
			seed_issuance(1_000_000 * UNITS);

			let state = CorrectBlock313222TotalIssuance::pre_upgrade().unwrap();
			CorrectBlock313222TotalIssuance::on_runtime_upgrade();
			CorrectBlock313222TotalIssuance::post_upgrade(state).unwrap();

			// Second pass (already-applied path) must also pass.
			let state = CorrectBlock313222TotalIssuance::pre_upgrade().unwrap();
			CorrectBlock313222TotalIssuance::on_runtime_upgrade();
			CorrectBlock313222TotalIssuance::post_upgrade(state).unwrap();
		});
	}
}
