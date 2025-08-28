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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! XCM configurations for Asset Hub for the AHM migration.

use crate::PhantomData;
use frame_support::traits::{ContainsPair, TypedGet};
use pallet_rc_migrator::types::MigrationStatus;
use sp_runtime::{traits::Get, AccountId32};
use xcm::latest::prelude::*;

/// To be used for `IsTeleport` filter. Disallows DOT teleports during the migration.
pub struct TrustedTeleporters<Stage, During, BeforeAfter>(
	PhantomData<(Stage, During, BeforeAfter)>,
);
impl<
		Stage: MigrationStatus,
		During: ContainsPair<Asset, Location>,
		BeforeAfter: ContainsPair<Asset, Location>,
	> ContainsPair<Asset, Location> for TrustedTeleporters<Stage, During, BeforeAfter>
{
	fn contains(asset: &Asset, origin: &Location) -> bool {
		let migration_ongoing = Stage::is_ongoing();
		log::trace!(target: "xcm::IsTeleport::contains", "migration ongoing: {:?}", migration_ongoing);
		let result = if migration_ongoing {
			During::contains(asset, origin)
		} else {
			// before and after migration use normal filter
			BeforeAfter::contains(asset, origin)
		};
		log::trace!(
			target: "xcm::IsTeleport::contains",
			"asset: {:?} origin {:?} result {:?}",
			asset, origin, result
		);
		result
	}
}

pub struct TreasuryAccount<Stage, PreMigrationAccount, PostMigrationAccount>(
	PhantomData<(Stage, PreMigrationAccount, PostMigrationAccount)>,
);
impl<Stage, PreMigrationAccount, PostMigrationAccount> Get<AccountId32>
	for TreasuryAccount<Stage, PreMigrationAccount, PostMigrationAccount>
where
	Stage: MigrationStatus,
	PreMigrationAccount: Get<AccountId32>,
	PostMigrationAccount: Get<AccountId32>,
{
	fn get() -> AccountId32 {
		let treasury_account = if Stage::is_finished() {
			PostMigrationAccount::get()
		} else {
			PreMigrationAccount::get()
		};
		log::trace!(target: "xcm::TreasuryAccount::get", "{:?}", treasury_account);
		treasury_account
	}
}

impl<Stage, PreMigrationAccount, PostMigrationAccount> TypedGet
	for TreasuryAccount<Stage, PreMigrationAccount, PostMigrationAccount>
where
	Stage: MigrationStatus,
	PreMigrationAccount: Get<AccountId32>,
	PostMigrationAccount: Get<AccountId32>,
{
	type Type = AccountId32;
	fn get() -> AccountId32 {
		<TreasuryAccount<Stage, PreMigrationAccount, PostMigrationAccount> as Get<AccountId32>>::get(
		)
	}
}

impl<Stage, PreMigrationAccount, PostMigrationAccount> Get<Location>
	for TreasuryAccount<Stage, PreMigrationAccount, PostMigrationAccount>
where
	Stage: MigrationStatus,
	PreMigrationAccount: Get<AccountId32>,
	PostMigrationAccount: Get<AccountId32>,
{
	fn get() -> Location {
		<TreasuryAccount<Stage, PreMigrationAccount, PostMigrationAccount> as Get<AccountId32>>::get(
		).into()
	}
}
