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
use frame_support::{parameter_types, traits::ContainsPair};
use xcm::latest::prelude::*;
use xcm_builder::Case;

#[cfg(feature = "ahm-polkadot")]
use polkadot_runtime_constants::system_parachain::*;
#[cfg(feature = "ahm-westend")]
use westend_runtime_constants::system_parachain::*;

parameter_types! {
	pub const Dot: AssetFilter = Wild(AllOf { fun: WildFungible, id: AssetId(Here.into_location()) });
	pub AssetHubLocation: Location = Parachain(ASSET_HUB_ID).into_location();
	pub DotForAssetHub: (AssetFilter, Location) = (Dot::get(), AssetHubLocation::get());
	pub CollectivesLocation: Location = Parachain(COLLECTIVES_ID).into_location();
	pub DotForCollectives: (AssetFilter, Location) = (Dot::get(), CollectivesLocation::get());
	pub CoretimeLocation: Location = Parachain(BROKER_ID).into_location();
	pub DotForCoretime: (AssetFilter, Location) = (Dot::get(), CoretimeLocation::get());
	pub BridgeHubLocation: Location = Parachain(BRIDGE_HUB_ID).into_location();
	pub DotForBridgeHub: (AssetFilter, Location) = (Dot::get(), BridgeHubLocation::get());
	pub People: Location = Parachain(PEOPLE_ID).into_location();
	pub DotForPeople: (AssetFilter, Location) = (Dot::get(), People::get());
}

/// Polkadot Relay recognizes/respects System Parachains as teleporters.
pub type TrustedTeleportersBeforeAndAfter = (
	Case<DotForAssetHub>,
	Case<DotForCollectives>,
	Case<DotForBridgeHub>,
	Case<DotForCoretime>,
	Case<DotForPeople>,
);

/// To be used for `IsTeleport` filter. Disallows DOT teleports during the migration.
pub struct TrustedTeleporters<Stage>(PhantomData<Stage>);
impl<Stage: MigrationStatus> ContainsPair<Asset, Location> for TrustedTeleporters<Stage> {
	fn contains(asset: &Asset, origin: &Location) -> bool {
		let migration_ongoing = Stage::is_ongoing();
		log::trace!(target: "xcm::IsTeleport::contains", "migration ongoing: {:?}", migration_ongoing);
		let result = if migration_ongoing {
			// during migration, no teleports (in or out) allowed
			false
		} else {
			// before and after migration use normal filter
			TrustedTeleportersBeforeAndAfter::contains(asset, origin)
		};
		log::trace!(
			target: "xcm::IsTeleport::contains",
			"asset: {:?} origin {:?} result {:?}",
			asset, origin, result
		);
		result
	}
}
