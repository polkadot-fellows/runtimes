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

//! Check that TI Correction burns exactly the issuance no account held.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, Para},
};

type Rc = network::relay::Runtime;

/// The total issuance and what the accounts held before the migration.
#[derive(Clone, Debug)]
pub struct IssuancePre {
	pub issuance: u128,
	/// The free and reserved balance of every account.
	pub balances: u128,
}

/// TI Correction burns what the total issuance counted beyond every account's balance.
pub struct TiCorrectionChecker;

impl RcMigrationCheck for TiCorrectionChecker {
	type RcPrePayload = IssuancePre;
	type RcPostPayload = ();

	fn pre_check() -> Self::RcPrePayload {
		let balances = frame_system::Account::<Rc>::iter().map(|(_, info)| info.data.total()).sum();
		IssuancePre { issuance: pallet_balances::TotalIssuance::<Rc>::get(), balances }
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		let burned = rc_pre.issuance.saturating_sub(rc_pre.balances);
		assert_eq!(
			pallet_rc2_migrator::RcMigratedBalance::<Rc>::get().ti_corrected,
			burned,
			"TI Correction should burn the issuance no account held"
		);
		assert!(
			!pallet_rc2_migrator::RcIssuanceAudit::<Rc>::exists(),
			"TI Correction should take the audit"
		);
	}
}

impl<P: Para> ParaMigrationCheck<P> for TiCorrectionChecker {
	type RcPrePayload = IssuancePre;
	type RcPostPayload = ();
	type PrePayload = ();

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {}

	fn post_check(_: &Self::RcPrePayload, _: &Self::RcPostPayload, _: Self::PrePayload) {}
}
