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

//! XCM configurations for the Relay Chain for the AHM migration.

use crate::{types::MigrationStatus, PhantomData};
use frame_support::traits::ContainsPair;
use xcm::latest::prelude::*;

/// To be used for `IsTeleport` filter. Disallows DOT teleports during the migration.
pub struct FalseIfMigrating<Stage, Inner>(PhantomData<(Stage, Inner)>);
impl<Stage: MigrationStatus, Inner: ContainsPair<Asset, Location>> ContainsPair<Asset, Location>
	for FalseIfMigrating<Stage, Inner>
{
	fn contains(asset: &Asset, origin: &Location) -> bool {
		let migration_ongoing = Stage::is_ongoing();
		log::trace!(target: "xcm::IsTeleport::contains", "migration ongoing: {:?}", migration_ongoing);
		let result = if migration_ongoing {
			// during migration, no teleports (in or out) allowed
			false
		} else {
			// before and after migration use normal filter
			Inner::contains(asset, origin)
		};
		log::trace!(
			target: "xcm::IsTeleport::contains",
			"asset: {:?} origin {:?} result {:?}",
			asset, origin, result
		);
		result
	}
}
