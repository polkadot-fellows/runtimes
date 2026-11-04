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

//! Checks of the total issuance on all three chains against what the Relay Chain booked.
//!
//! Every stage books what it burns on the Relay Chain in `RcMigratedBalance`, by destination.
//! These checks hold over the whole migration whatever the stages are: the Relay Chain's issuance
//! fell by exactly what was booked as leaving, and each receiving chain took in exactly what was
//! booked for it. The per-stage checkers check where it went.

use crate::{
	checks::{ParaMigrationCheck, RcMigrationCheck},
	mock::{network, AssetHubPara, CoretimePara},
};
use pallet_rc2_migrator::{MigratedBalances, RcMigratedBalance};

type Rc = network::relay::Runtime;
type Ct = network::ct::Runtime;
type Ah = network::ah::Runtime;

/// The Relay Chain's issuance before the migration.
pub type RcIssuancePre = u128;

/// Asset Hub's issuance and checking account before the migration.
#[derive(Clone, Debug)]
pub struct AhIssuancePre {
	pub total_issuance: u128,
	pub checking: u128,
}

fn ah_checking_account() -> u128 {
	let checking = pallet_xcm::Pallet::<Ah>::check_account();
	frame_system::Account::<Ah>::get(checking).data.total()
}

/// Issuance moves only as the Relay Chain booked it.
pub struct IssuanceChecker;

impl RcMigrationCheck for IssuanceChecker {
	type RcPrePayload = RcIssuancePre;
	type RcPostPayload = MigratedBalances;

	fn pre_check() -> Self::RcPrePayload {
		pallet_balances::TotalIssuance::<Rc>::get()
	}

	fn post_check(rc_pre: &Self::RcPrePayload) -> Self::RcPostPayload {
		let booked = RcMigratedBalance::<Rc>::get();
		let issuance = pallet_balances::TotalIssuance::<Rc>::get();

		// Nothing changed the issuance without booking it.
		assert_eq!(booked.kept, issuance, "the booked balance kept here is not the issuance");
		// What left is exactly what was booked as leaving.
		let left = booked.ct_reserved + booked.ct_free + booked.ah_free + booked.ti_corrected;
		assert_eq!(rc_pre - issuance, left, "the issuance fell by other than what was booked");

		booked
	}
}

impl ParaMigrationCheck<CoretimePara> for IssuanceChecker {
	type RcPrePayload = RcIssuancePre;
	type RcPostPayload = MigratedBalances;
	type PrePayload = u128;

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {
		assert_eq!(pallet_ct_migrator::CtMintedTotal::<Ct>::get(), 0);
		pallet_balances::TotalIssuance::<Ct>::get()
	}

	fn post_check(_: &Self::RcPrePayload, rc_post: &Self::RcPostPayload, ct_pre: Self::PrePayload) {
		// Minted exactly what the Relay Chain booked for here, and nothing else changed the
		// issuance.
		let minted = pallet_ct_migrator::CtMintedTotal::<Ct>::get();
		assert_eq!(minted, rc_post.ct_reserved + rc_post.ct_free);
		assert_eq!(pallet_balances::TotalIssuance::<Ct>::get(), ct_pre + minted);
	}
}

impl ParaMigrationCheck<AssetHubPara> for IssuanceChecker {
	type RcPrePayload = RcIssuancePre;
	type RcPostPayload = MigratedBalances;
	type PrePayload = AhIssuancePre;

	fn pre_check(_: &Self::RcPrePayload) -> Self::PrePayload {
		AhIssuancePre {
			total_issuance: pallet_balances::TotalIssuance::<Ah>::get(),
			checking: ah_checking_account(),
		}
	}

	fn post_check(_: &Self::RcPrePayload, rc_post: &Self::RcPostPayload, ah_pre: Self::PrePayload) {
		// A teleport in moves balance out of the checking account; it mints nothing.
		assert_eq!(pallet_balances::TotalIssuance::<Ah>::get(), ah_pre.total_issuance);
		// The checking account paid out exactly what the Relay Chain booked for here.
		let paid = ah_pre.checking - ah_checking_account();
		assert_eq!(paid, rc_post.ah_free);
	}
}
