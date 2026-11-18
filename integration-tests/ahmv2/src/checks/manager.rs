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

//! Checks of the manager's reap at the end of the cool-off: the manager leaves the Relay Chain
//! whole and its balance arrives on its account on Asset Hub.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, AssetHubPara, CoretimePara},
};
use pallet_rc2_migrator::accounts::AccountsMigrator;
use sp_runtime::AccountId32;

type Rc = network::relay::Runtime;
type Ah = network::ah::Runtime;

/// The manager on the Relay Chain before the migration.
#[derive(Clone, Debug)]
pub struct RcManagerPre {
	pub manager: AccountId32,
	/// Its free balance, all of which the reap sends to Asset Hub.
	pub balance: u128,
	/// Where the reap sends it on Asset Hub.
	pub payee: AccountId32,
}

/// Check that the manager is reaped and paid on Asset Hub.
pub struct ManagerChecker;

impl RcMigrationCheck for ManagerChecker {
	type RcPrePayload = RcManagerPre;
	type RcPostPayload = ();

	fn pre_check() -> Self::RcPrePayload {
		let manager = pallet_rc2_migrator::Manager::<Rc>::get()
			.expect("the run appoints a manager, so the reap is checked");
		let account = frame_system::Account::<Rc>::get(&manager);
		// The reap burns only the manager's free balance.
		assert_eq!(
			(account.data.reserved, account.data.frozen, account.consumers),
			(0, 0, 0),
			"the manager has a reserve, a freeze or a consumer reference"
		);
		RcManagerPre {
			payee: AccountsMigrator::<Rc>::ah_destination(&manager),
			balance: account.data.free,
			manager,
		}
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		assert_eq!(pallet_rc2_migrator::Manager::<Rc>::get(), None, "the manager was not cleared");
		assert_eq!(
			frame_system::Account::<Rc>::get(&rc_pre.manager).data.total(),
			0,
			"the manager {:?} should have left the Relay Chain whole",
			rc_pre.manager
		);
	}
}

impl ParaMigrationCheck<CoretimePara> for ManagerChecker {
	type RcPrePayload = RcManagerPre;
	type RcPostPayload = ();
	type PrePayload = ();

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, _: Self::PrePayload) {}
}

impl ParaMigrationCheck<AssetHubPara> for ManagerChecker {
	type RcPrePayload = RcManagerPre;
	type RcPostPayload = ();
	/// The payee's balance before the migration.
	type PrePayload = u128;

	fn pre_check(rc_pre: &Self::RcPrePayload) -> Self::PrePayload {
		frame_system::Account::<Ah>::get(&rc_pre.payee).data.total()
	}

	fn post_check(rc_pre: &Self::RcPrePayload, _: &Self::RcPostPayload, ah_pre: Self::PrePayload) {
		let arrived = frame_system::Account::<Ah>::get(&rc_pre.payee)
			.data
			.total()
			.checked_sub(ah_pre)
			.expect("the manager's payee did not shrink");
		assert_eq!(
			arrived, rc_pre.balance,
			"the manager's payee got something other than its balance"
		);
	}
}
