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

//! One-off migration that refunds reserves wrongfully dusted by the `currency -> fungible`
//! migration.
//!
//! That migration dropped the *reserved* balance of some accounts (e.g. crowdloan/lease deposits)
//! when they were the source of a transfer; once their free balance later fell below the ED the
//! reserve was burned for good. A documented case is [extrinsic `18210366-2`](https://polkadot.subscan.io/extrinsic/18210366-2),
//! losing `1257.831 DOT` on `12EXcpt1CwnSAF9d7YWrh91bQw6R5wmCpJUXPWi7vn2CZFpJ` (see
//! [issue #583](https://github.com/polkadot-fellows/runtimes/issues/583)). These gaps also break
//! the Asset Hub Migration accounting (e.g. `pallet_ah_ops::do_unreserve_crowdloan_reserve`).
//!
//! Fix: for each affected account, mint the missing amount and place it back on (legacy/unnamed)
//! reserve — exactly how it was originally held — growing total issuance by the same amount.

extern crate alloc;

use crate::{AccountId, Balance, Balances, Runtime};
use frame_support::{
	traits::{fungible::Mutate, OnRuntimeUpgrade, ReservableCurrency},
	weights::Weight,
};

#[cfg(feature = "try-runtime")]
use alloc::vec::Vec;
#[cfg(feature = "try-runtime")]
use frame_support::{ensure, traits::Currency};

const LOG_TARGET: &str = "runtime::refund_dusted_deposit";

/// Affected accounts (raw 32-byte public key) and the amount in plancks to mint and re-reserve.
///
/// This list is the single source of truth for the migration. Amounts come from the reserved
/// balance before/after the dusting transfer in the chain history. Only the account with on-chain
/// evidence is included for now; more can be appended once the archive analysis identifies them.
pub const AFFECTED_ACCOUNTS: &[([u8; 32], Balance)] = &[
	// 12EXcpt1CwnSAF9d7YWrh91bQw6R5wmCpJUXPWi7vn2CZFpJ — dusted in extrinsic 18210366-2.
	(
		[
			0x36, 0x8d, 0x7d, 0xf4, 0x7f, 0xf9, 0xf0, 0x15, 0xa2, 0x47, 0xdd, 0xea, 0x7b, 0x37,
			0xab, 0xb1, 0xd5, 0x63, 0x87, 0xb6, 0x32, 0xad, 0xf8, 0x39, 0x3b, 0xb7, 0x3f, 0x60,
			0x65, 0x40, 0xfd, 0x1f,
		],
		12_578_310_000_000,
	),
];

/// Sum of all amounts that this migration mints and reserves.
#[cfg(feature = "try-runtime")]
fn total_refund() -> Balance {
	AFFECTED_ACCOUNTS.iter().fold(0, |acc, (_, amount)| acc + *amount)
}

/// Refund the wrongfully dusted reserves. One-off: remove from the migration list after the
/// upgrade that ships it. Written defensively so one bad account can never brick the upgrade.
pub struct RefundDustedReserves;

impl OnRuntimeUpgrade for RefundDustedReserves {
	fn on_runtime_upgrade() -> Weight {
		let mut refunded: u32 = 0;

		for (raw, amount) in AFFECTED_ACCOUNTS {
			let who: AccountId = AccountId::from(*raw);

			// Mint the missing amount, re-creating the account if needed.
			if let Err(e) = <Balances as Mutate<AccountId>>::mint_into(&who, *amount) {
				log::error!(
					target: LOG_TARGET,
					"Failed to mint {amount} plancks into {who:?}: {e:?}",
				);
				continue;
			}

			// Re-reserve it as the deposit was originally held; defensive on the unexpected error.
			if let Err(e) = <Balances as ReservableCurrency<AccountId>>::reserve(&who, *amount) {
				log::error!(
					target: LOG_TARGET,
					"Failed to reserve {amount} plancks on {who:?}: {e:?}",
				);
				continue;
			}

			refunded = refunded.saturating_add(1);
			log::info!(
				target: LOG_TARGET,
				"Refunded {amount} plancks of dusted reserve to {who:?}",
			);
		}

		log::info!(
			target: LOG_TARGET,
			"RefundDustedReserves: refunded {refunded}/{} accounts",
			AFFECTED_ACCOUNTS.len(),
		);

		// Per account: a mint and a reserve. Charge a conservative 4 reads + 4 writes.
		let per_account = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(4, 4);
		per_account.saturating_mul(AFFECTED_ACCOUNTS.len() as u64)
	}

	#[cfg(feature = "try-runtime")]
	fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
		use codec::Encode;

		// Snapshot issuance and per-account reserve for `post_upgrade` to diff against.
		let issuance_before = <Balances as Currency<AccountId>>::total_issuance();
		let reserved_before: Vec<Balance> = AFFECTED_ACCOUNTS
			.iter()
			.map(|(raw, _)| {
				let who: AccountId = AccountId::from(*raw);
				<Balances as ReservableCurrency<AccountId>>::reserved_balance(&who)
			})
			.collect();

		Ok((issuance_before, reserved_before).encode())
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(state: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use codec::Decode;

		let (issuance_before, reserved_before): (Balance, Vec<Balance>) =
			Decode::decode(&mut &state[..]).map_err(|_| "Failed to decode pre_upgrade state")?;

		// Issuance must grow by exactly the total refund.
		let issuance_after = <Balances as Currency<AccountId>>::total_issuance();
		ensure!(
			issuance_after == issuance_before.saturating_add(total_refund()),
			"Total issuance did not grow by the expected refund amount"
		);

		// Each account's reserve must grow by exactly its refund.
		for ((raw, amount), before) in AFFECTED_ACCOUNTS.iter().zip(reserved_before.iter()) {
			let who: AccountId = AccountId::from(*raw);
			let reserved_after =
				<Balances as ReservableCurrency<AccountId>>::reserved_balance(&who);
			ensure!(
				reserved_after == before.saturating_add(*amount),
				"Account reserve was not increased by the expected amount"
			);
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::System;
	use frame_support::traits::Currency;
	use sp_core::crypto::Ss58Codec;
	use sp_runtime::BuildStorage;

	/// Clean test externalities backed by the real runtime types.
	fn new_test_ext() -> sp_io::TestExternalities {
		let storage = frame_system::GenesisConfig::<Runtime>::default().build_storage().unwrap();
		let mut ext: sp_io::TestExternalities = storage.into();
		ext.execute_with(|| System::set_block_number(1));
		ext
	}

	#[test]
	fn affected_account_decodes_to_expected_address() {
		// The embedded raw key must be the account from the issue.
		let (raw, amount) = AFFECTED_ACCOUNTS[0];
		let who: AccountId = AccountId::from(raw);
		let expected =
			AccountId::from_ss58check("12EXcpt1CwnSAF9d7YWrh91bQw6R5wmCpJUXPWi7vn2CZFpJ").unwrap();
		assert_eq!(who, expected);
		assert_eq!(amount, 12_578_310_000_000);
	}

	#[test]
	fn refund_dusted_reserves_works() {
		new_test_ext().execute_with(|| {
			// Affected accounts still hold residual free balance on-chain which a reserve needs to stay above the ED. Reproduce that minimal state.
			let existing_free: Balance = polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
			for (raw, _) in AFFECTED_ACCOUNTS {
				let who: AccountId = AccountId::from(*raw);
				<Balances as Mutate<AccountId>>::mint_into(&who, existing_free).unwrap();
			}

			let issuance_before = Balances::total_issuance();

			let _weight = RefundDustedReserves::on_runtime_upgrade();

			let mut expected_minted: Balance = 0;
			for (raw, amount) in AFFECTED_ACCOUNTS {
				let who: AccountId = AccountId::from(*raw);
				assert_eq!(
					<Balances as ReservableCurrency<AccountId>>::reserved_balance(&who),
					*amount,
					"reserve must be restored",
				);
				// The refund sits on reserve; free balance is untouched.
				assert_eq!(
					<Balances as Currency<AccountId>>::free_balance(&who),
					existing_free,
				);
				expected_minted += *amount;
			}

			assert_eq!(
				Balances::total_issuance(),
				issuance_before + expected_minted,
				"total issuance must grow by exactly the refunded amount",
			);
		});
	}

	#[test]
	fn refund_adds_to_a_preexisting_reserve() {
		new_test_ext().execute_with(|| {
			let (raw, amount) = AFFECTED_ACCOUNTS[0];
			let who: AccountId = AccountId::from(raw);

			// Seed a pre-existing free + reserved balance.
			let existing_free: Balance = 5 * polkadot_runtime_constants::currency::UNITS;
			let existing_reserved: Balance = 2 * polkadot_runtime_constants::currency::UNITS;
			<Balances as Mutate<AccountId>>::mint_into(&who, existing_free + existing_reserved)
				.unwrap();
			<Balances as ReservableCurrency<AccountId>>::reserve(&who, existing_reserved).unwrap();

			RefundDustedReserves::on_runtime_upgrade();

			// Refund stacks on top of the existing reserve; free is untouched.
			assert_eq!(
				<Balances as ReservableCurrency<AccountId>>::reserved_balance(&who),
				existing_reserved + amount,
			);
			assert_eq!(<Balances as Currency<AccountId>>::free_balance(&who), existing_free);
		});
	}

	#[cfg(feature = "try-runtime")]
	#[test]
	fn pre_and_post_upgrade_checks_pass() {
		new_test_ext().execute_with(|| {
			// Endow the residual free balance that backs the reserve (see `refund_dusted_reserves_works`).
			let existing_free: Balance = polkadot_runtime_constants::currency::EXISTENTIAL_DEPOSIT;
			for (raw, _) in AFFECTED_ACCOUNTS {
				let who: AccountId = AccountId::from(*raw);
				<Balances as Mutate<AccountId>>::mint_into(&who, existing_free).unwrap();
			}

			let state = RefundDustedReserves::pre_upgrade().unwrap();
			RefundDustedReserves::on_runtime_upgrade();
			RefundDustedReserves::post_upgrade(state).unwrap();
		});
	}
}
