// Copyright (C) Polkadot Fellows.
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

//! Checks of the accounts stage over all three chains.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, AssetHubPara, CoretimePara},
};
use migrator_types::translate_destination;
use pallet_rc2_migrator::{accounts::AccountsMigrator, MigratedBalances};
use sp_runtime::AccountId32;
use std::collections::BTreeMap;

type Rc = network::relay::Runtime;
type Ct = network::ct::Runtime;
type Ah = network::ah::Runtime;

/// One Relay Chain account as the accounts stage found it.
#[derive(Clone, Debug)]
pub struct RcAccount {
	/// Free plus reserved balance.
	pub total: u128,
	/// Reserved balance, holds included.
	pub reserved: u128,
	/// Whether a later stage handles the account.
	pub deferred: bool,
	/// Whether the account is a pure proxy, so all of it goes to the Coretime chain.
	pub pure_like: bool,
}

impl RcAccount {
	/// An account whose whole balance is teleported to Asset Hub.
	fn is_plain(&self) -> bool {
		self.reserved == 0 && !self.pure_like
	}
}

/// What the migrating accounts that land on one destination account add up to. A child sovereign
/// and a sibling-format account for the same para share a destination.
#[derive(Clone, Debug)]
pub struct Expected {
	/// Sum of the sources' totals.
	total: u128,
	/// Sum of the sources' reserved balances.
	reserved: u128,
	/// Every source is plain.
	all_plain: bool,
	/// Every source is pure-like.
	all_pure_like: bool,
}

/// What the Relay Chain held before the migration.
#[derive(Clone, Debug)]
pub struct RcAccountsPre {
	pub accounts: Vec<(AccountId32, RcAccount)>,
	/// The migrating accounts, grouped by the account they land on.
	pub destinations: BTreeMap<AccountId32, Expected>,
	pub total_issuance: u128,
}

/// How much `who`'s balance grew from `before` to `after`. Panics if it shrank, which no step of
/// the migration may cause.
fn grown(who: &AccountId32, before: u128, after: u128) -> u128 {
	after
		.checked_sub(before)
		.unwrap_or_else(|| panic!("{who:?} shrank: {before} -> {after}"))
}

fn account_data<T>(who: &AccountId32) -> pallet_balances::AccountData<u128>
where
	T: frame_system::Config<
		AccountId = AccountId32,
		AccountData = pallet_balances::AccountData<u128>,
	>,
{
	frame_system::Account::<T>::get(who).data
}

/// Every account leaves the Relay Chain whole or is left for a later stage, and each lands where
/// the split rule sends it: plain accounts on Asset Hub, pure-like accounts on the Coretime chain,
/// deposit holders on both.
pub struct AccountsChecker;

impl RcMigrationCheck for AccountsChecker {
	type RcPrePayload = RcAccountsPre;
	type RcPostPayload = MigratedBalances;

	fn pre_check() -> Self::RcPrePayload {
		let manager = pallet_rc2_migrator::Manager::<Rc>::get();
		let mut destinations: BTreeMap<AccountId32, Expected> = BTreeMap::new();
		let accounts = frame_system::Account::<Rc>::iter()
			.map(|(who, info)| {
				let account = RcAccount {
					total: info.data.total(),
					reserved: info.data.reserved,
					deferred: AccountsMigrator::<Rc>::deferred(&who, &info, manager.as_slice()),
					pure_like: AccountsMigrator::<Rc>::is_pure_proxy(&who, &info),
				};
				if !account.deferred {
					let dest =
						destinations.entry(translate_destination(&who)).or_insert(Expected {
							total: 0,
							reserved: 0,
							all_plain: true,
							all_pure_like: true,
						});
					dest.total += account.total;
					dest.reserved += account.reserved;
					dest.all_plain &= account.is_plain();
					dest.all_pure_like &= account.pure_like;
				}
				(who, account)
			})
			.collect();

		RcAccountsPre {
			accounts,
			destinations,
			total_issuance: pallet_balances::TotalIssuance::<Rc>::get(),
		}
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		let mut migrated = 0u128;
		for (who, account) in &rc_pre.accounts {
			let data = account_data::<Rc>(who);
			if account.deferred {
				assert_eq!(
					data.total(),
					account.total,
					"account {who:?} is for a later stage and must be untouched"
				);
			} else {
				// Gone, or a zero-balance shell some pallet still references.
				assert_eq!(
					(data.free, data.reserved),
					(0, 0),
					"account {who:?} should have left the Relay Chain whole"
				);
				migrated += account.total;
			}
		}

		// Everything the accounts left with is booked, split by destination, and the issuance
		// fell by exactly that.
		let booked = pallet_rc2_migrator::RcMigratedBalance::<Rc>::get();
		assert_eq!(booked.ct_reserved + booked.ct_free + booked.ah_free, migrated);
		let issuance = pallet_balances::TotalIssuance::<Rc>::get();
		assert_eq!(issuance, rc_pre.total_issuance - migrated);
		assert_eq!(booked.kept, issuance);

		// No preimage deposit survives the stage's init.
		let preimage_deposits = AccountsMigrator::<Rc>::preimage_deposits();
		assert!(preimage_deposits.is_empty(), "preimage deposits left: {preimage_deposits:?}");

		booked
	}
}

/// Coretime balances of every destination, and the chain's issuance, before the migration.
#[derive(Clone, Debug)]
pub struct CtAccountsPre {
	pub balances: BTreeMap<AccountId32, pallet_balances::AccountData<u128>>,
	pub total_issuance: u128,
}

impl ParaMigrationCheck<CoretimePara> for AccountsChecker {
	type RcPrePayload = RcAccountsPre;
	type RcPostPayload = MigratedBalances;
	type PrePayload = CtAccountsPre;

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		assert_eq!(pallet_ct_migrator::CtMintedTotal::<Ct>::get(), 0);
		let balances = rc_pre
			.destinations
			.keys()
			.map(|dest| (dest.clone(), account_data::<Ct>(dest)))
			.collect();
		CtAccountsPre { balances, total_issuance: pallet_balances::TotalIssuance::<Ct>::get() }
	}

	fn post_check(
		rc_pre: &Self::RcPrePayload,
		rc_post: &Self::RcPostPayload,
		ct_pre: Self::PrePayload,
	) {
		let failed: Vec<_> = pallet_ct_migrator::FailedAccounts::<Ct>::iter_keys().collect();
		assert!(failed.is_empty(), "accounts failed to integrate: {failed:?}");

		// Minted exactly what the Relay Chain booked for here, and nothing else changed the
		// issuance.
		let minted = pallet_ct_migrator::CtMintedTotal::<Ct>::get();
		assert_eq!(minted, rc_post.ct_reserved + rc_post.ct_free);
		assert_eq!(pallet_balances::TotalIssuance::<Ct>::get(), ct_pre.total_issuance + minted);

		let mut arrived = 0u128;
		let (mut unattributed_count, mut unattributed_sum) = (0u32, 0u128);
		for (dest, expected) in &rc_pre.destinations {
			let before = &ct_pre.balances[dest];
			let data = account_data::<Ct>(dest);
			let delta = grown(dest, before.total(), data.total());
			arrived += delta;

			if expected.all_plain {
				assert_eq!(delta, 0, "plain account {dest:?} must not land on Coretime");
			}
			if expected.all_pure_like {
				assert_eq!(delta, expected.total, "pure-like {dest:?} must land on Coretime whole");
			}
			if expected.reserved > 0 {
				assert!(
					grown(dest, before.reserved, data.reserved) <= expected.reserved,
					"{dest:?} holds more on Coretime than it reserved on the Relay Chain"
				);
				if let Some(amount) = unattributed_hold(dest) {
					unattributed_count += 1;
					unattributed_sum += amount;
				}
			}
		}
		assert_eq!(arrived, minted, "Coretime minted for accounts that did not migrate");

		// Reserve no record explained is parked here; say how much, so its size is known.
		log::info!(
			target: "runtime::coretime",
			"Unattributed reserve parked on Coretime: {unattributed_count} account(s), {unattributed_sum} units"
		);
	}
}

/// What `who` holds on the Coretime chain under `UnattributedReserve`.
fn unattributed_hold(who: &AccountId32) -> Option<u128> {
	let reason = network::ct::RuntimeHoldReason::CtMigrator(
		pallet_ct_migrator::HoldReason::UnattributedReserve,
	);
	pallet_balances::Holds::<Ct>::get(who)
		.iter()
		.find(|hold| hold.id == reason)
		.map(|hold| hold.amount)
}

/// Asset Hub balances of every destination, its issuance and its checking account, before the
/// migration.
#[derive(Clone, Debug)]
pub struct AhAccountsPre {
	pub balances: BTreeMap<AccountId32, u128>,
	pub total_issuance: u128,
	pub checking: u128,
}

fn ah_checking_account() -> u128 {
	account_data::<Ah>(&pallet_xcm::Pallet::<Ah>::check_account()).total()
}

impl ParaMigrationCheck<AssetHubPara> for AccountsChecker {
	type RcPrePayload = RcAccountsPre;
	type RcPostPayload = MigratedBalances;
	type PrePayload = AhAccountsPre;

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		let balances = rc_pre
			.destinations
			.keys()
			.map(|dest| (dest.clone(), account_data::<Ah>(dest).total()))
			.collect();
		AhAccountsPre {
			balances,
			total_issuance: pallet_balances::TotalIssuance::<Ah>::get(),
			checking: ah_checking_account(),
		}
	}

	fn post_check(
		rc_pre: &Self::RcPrePayload,
		rc_post: &Self::RcPostPayload,
		ah_pre: Self::PrePayload,
	) {
		// A teleport in moves balance out of the checking account; it mints nothing.
		assert_eq!(pallet_balances::TotalIssuance::<Ah>::get(), ah_pre.total_issuance);

		let mut arrived = 0u128;
		for (dest, expected) in &rc_pre.destinations {
			let delta = grown(dest, ah_pre.balances[dest], account_data::<Ah>(dest).total());
			arrived += delta;

			if expected.all_plain {
				assert_eq!(delta, expected.total, "plain account {dest:?} must land on AH whole");
			}
			if expected.all_pure_like {
				assert_eq!(delta, 0, "pure-like {dest:?} must not land on AH");
			}
		}

		// The checking account paid out exactly what the Relay Chain booked for here.
		let paid = ah_pre.checking - ah_checking_account();
		assert_eq!(
			paid, arrived,
			"the checking account paid for something other than the migration"
		);
		assert_eq!(paid, rc_post.ah_free);
	}
}
