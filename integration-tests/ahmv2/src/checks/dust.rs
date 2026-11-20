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

//! Checks of the Sweep Dust stage: every account below the existential deposit ends empty, its
//! balance arrives on Asset Hub where the stage's rules send it, and no empty record is left that
//! nothing needs.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, AssetHubPara, CoretimePara},
};
use frame_support::traits::Get;
use pallet_rc2_migrator::accounts::AccountsMigrator;
use sp_runtime::AccountId32;
use std::collections::{BTreeMap, BTreeSet};

type Rc = network::relay::Runtime;
type Ah = network::ah::Runtime;

/// The dust on the Relay Chain before the migration, and where Sweep Dust sends it.
#[derive(Clone, Debug)]
pub struct RcDustPre {
	/// Every account below the existential deposit, other than the manager and the pots.
	pub accounts: Vec<AccountId32>,
	/// What each owner on Asset Hub receives.
	pub to_owners: BTreeMap<AccountId32, u128>,
	/// What the sweep beneficiary receives.
	pub to_beneficiary: u128,
	/// The owners on Asset Hub that the accounts stage also sends balance to.
	pub shared: BTreeSet<AccountId32>,
}

/// Where Sweep Dust sends each account's dust, worked out from the Relay Chain's state before the
/// migration. No earlier stage touches these accounts.
pub fn expected_dust() -> RcDustPre {
	let rc_ed = <Rc as pallet_balances::Config>::ExistentialDeposit::get();
	let ah_ed = <Rc as pallet_rc2_migrator::Config>::AhExistentialDeposit::get();
	let pots = <Rc as pallet_rc2_migrator::Config>::SweptAccounts::get();
	let beneficiary = <Rc as pallet_rc2_migrator::Config>::SweepBeneficiary::get();
	let manager = pallet_rc2_migrator::Manager::<Rc>::get();

	let mut dust = RcDustPre {
		accounts: Vec::new(),
		to_owners: BTreeMap::new(),
		to_beneficiary: 0,
		shared: BTreeSet::new(),
	};
	let mut accounts_stage = BTreeSet::new();
	for (who, info) in frame_system::Account::<Rc>::iter() {
		if !AccountsMigrator::<Rc>::deferred(&who, &info, manager.as_ref()) {
			accounts_stage.insert(AccountsMigrator::<Rc>::ah_destination(&who));
			continue;
		}
		let amount = info.data.total();
		if amount >= rc_ed || pots.contains(&who) || manager.as_ref() == Some(&who) {
			continue;
		}
		dust.accounts.push(who.clone());
		if amount == 0 {
			continue;
		}
		let owned = !AccountsMigrator::<Rc>::is_pure_proxy(&who, &info);
		let to = if owned && amount >= ah_ed {
			AccountsMigrator::<Rc>::ah_destination(&who)
		} else {
			beneficiary.clone()
		};
		if to == beneficiary {
			dust.to_beneficiary += amount;
		} else {
			*dust.to_owners.entry(to).or_default() += amount;
		}
	}
	dust.shared = dust
		.to_owners
		.keys()
		.filter(|to| accounts_stage.contains(*to))
		.cloned()
		.collect();
	dust
}

pub struct SweepDustChecker;

impl RcMigrationCheck for SweepDustChecker {
	type RcPrePayload = RcDustPre;
	type RcPostPayload = ();

	fn pre_check() -> Self::RcPrePayload {
		expected_dust()
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		for who in &rc_pre.accounts {
			let data = frame_system::Account::<Rc>::get(who).data;
			assert_eq!(
				(data.free, data.reserved),
				(0, 0),
				"dust on {who:?} should have been swept"
			);
		}

		// No empty record is left that at most one provider reference keeps, whichever stage
		// emptied it. The manager and the pots are not for Sweep Dust to remove.
		let manager = pallet_rc2_migrator::Manager::<Rc>::get();
		let pots = <Rc as pallet_rc2_migrator::Config>::SweptAccounts::get();
		for (who, account) in frame_system::Account::<Rc>::iter() {
			if account.data.total() != 0 || manager.as_ref() == Some(&who) || pots.contains(&who) {
				continue;
			}
			assert!(
				!matches!(
					(account.providers, account.consumers, account.sufficients),
					(1, 0, 0) | (0, 0, 0)
				),
				"{who:?} is empty with at most one provider reference and should have been removed"
			);
		}

		let to_owners: u128 = rc_pre.to_owners.values().sum();
		log::info!(
			target: "runtime::rc2-migrator",
			"Swept {} of dust out of {} account(s): {to_owners} to {} owner(s), {} to the beneficiary",
			to_owners + rc_pre.to_beneficiary,
			rc_pre.accounts.len(),
			rc_pre.to_owners.len(),
			rc_pre.to_beneficiary,
		);
	}
}

impl ParaMigrationCheck<CoretimePara> for SweepDustChecker {
	type RcPrePayload = RcDustPre;
	type RcPostPayload = ();
	type PrePayload = ();

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, _: Self::PrePayload) {}
}

impl ParaMigrationCheck<AssetHubPara> for SweepDustChecker {
	type RcPrePayload = RcDustPre;
	type RcPostPayload = ();
	/// Each owner's balance before the migration.
	type PrePayload = BTreeMap<AccountId32, u128>;

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		rc_pre
			.to_owners
			.keys()
			.map(|owner| (owner.clone(), frame_system::Account::<Ah>::get(owner).data.total()))
			.collect()
	}

	fn post_check(rc_pre: &Self::RcPrePayload, _: &Self::RcPostPayload, ah_pre: Self::PrePayload) {
		// `SweepChecker` checks what the beneficiary receives.
		for (owner, dust) in &rc_pre.to_owners {
			let arrived = frame_system::Account::<Ah>::get(owner)
				.data
				.total()
				.checked_sub(ah_pre[owner])
				.unwrap_or_else(|| panic!("{owner:?} shrank on Asset Hub"));
			if rc_pre.shared.contains(owner) {
				assert!(arrived >= *dust, "{owner:?} got {arrived}, less than its {dust} of dust");
			} else {
				assert_eq!(arrived, *dust, "{owner:?} should get exactly its dust");
			}
		}
	}
}
