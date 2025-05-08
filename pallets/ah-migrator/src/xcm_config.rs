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

use assets_common::matching::{FromSiblingParachain, IsForeignConcreteAsset, ParentLocation};
use cumulus_primitives_core::ParaId;
use frame_support::parameter_types;
use frame_support::traits::{Contains, Equals};
use parachains_common::xcm_config::{ConcreteAssetFromSystem, ParentRelayOrSiblingParachains};
use xcm::latest::prelude::*;
use xcm_builder::{AllowExplicitUnpaidExecutionFrom, IsSiblingSystemParachain};

#[cfg(feature = "ahm-polkadot")]
use polkadot_runtime_constants::system_parachain;
#[cfg(feature = "ahm-westend")]
use westend_runtime_constants::system_parachain;

pub mod common {
	use super::*;
	parameter_types! {
		pub const AssetHubParaId: ParaId = ParaId::new(system_parachain::ASSET_HUB_ID);
		pub const DotLocation: Location = Location::parent();
	}

	pub struct FellowshipEntities;
	impl Contains<Location> for FellowshipEntities {
		fn contains(location: &Location) -> bool {
			matches!(
			location.unpack(),
			(
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					Plurality { id: BodyId::Technical, .. }
				]
			) | (
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::FELLOWSHIP_SALARY_PALLET_INDEX
					)
				]
			) | (
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::FELLOWSHIP_TREASURY_PALLET_INDEX
					)
				]
			)
		)
		}
	}

	pub struct AmbassadorEntities;
	impl Contains<Location> for AmbassadorEntities {
		fn contains(location: &Location) -> bool {
			matches!(
			location.unpack(),
			(
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::AMBASSADOR_SALARY_PALLET_INDEX
					)
				]
			) | (
				1,
				[
					Parachain(system_parachain::COLLECTIVES_ID),
					PalletInstance(
						collectives_polkadot_runtime_constants::AMBASSADOR_TREASURY_PALLET_INDEX
					)
				]
			)
		)
		}
	}

	// Teleport filters are a special case because we might want to have finer control over which
	// one to use at fine-grained stages of the migration.

	/// Cases where a remote origin is accepted as trusted Teleporter for a given asset:
	///
	/// - DOT with the parent Relay Chain and sibling system parachains; and
	/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
	pub type TrustedTeleportersBeforeAfter = (
		ConcreteAssetFromSystem<DotLocation>,
		IsForeignConcreteAsset<FromSiblingParachain<AssetHubParaId>>,
	);

	/// During migration we only allow teleports of foreign assets (not DOT).
	///
	/// - Sibling parachains' assets from where they originate (as `ForeignCreators`).
	pub type TrustedTeleportersDuring =
		IsForeignConcreteAsset<FromSiblingParachain<AssetHubParaId>>;
}

pub mod before {
	use super::common::{AmbassadorEntities, AssetHubParaId, FellowshipEntities};
	use super::*;

	parameter_types! {
		pub RelayTreasuryLocation: Location =
			(Parent, PalletInstance(polkadot_runtime_constants::TREASURY_PALLET_ID)).into();
	}

	pub struct ParentOrParentsPlurality;
	impl Contains<Location> for ParentOrParentsPlurality {
		fn contains(location: &Location) -> bool {
			matches!(location.unpack(), (1, []) | (1, [Plurality { .. }]))
		}
	}

	/// For use in XCM Barriers: the locations listed below get free execution:
	///
	/// Parent, its pluralities (i.e. governance bodies), the Fellows plurality, AmbassadorEntities
	/// and sibling system parachains' root get free execution.
	pub type UnpaidExecutionBeforeDuring = AllowExplicitUnpaidExecutionFrom<(
		ParentOrParentsPlurality,
		IsSiblingSystemParachain<ParaId, AssetHubParaId>,
		Equals<RelayTreasuryLocation>,
		FellowshipEntities,
		AmbassadorEntities,
	)>;

	/// Locations that will not be charged fees in the executor, either execution or delivery.
	///
	/// We only waive fees for system functions, which these locations represent.
	pub type WaivedLocationsBeforeDuring = (
		Equals<ParentLocation>,
		IsSiblingSystemParachain<ParaId, AssetHubParaId>,
		Equals<RelayTreasuryLocation>,
		FellowshipEntities,
		AmbassadorEntities,
	);
}

pub mod during {
	use super::*;
}

pub mod after {
	use super::common::{AmbassadorEntities, AssetHubParaId, FellowshipEntities};
	use super::*;

	/// For use in XCM Barriers: the locations listed below get free execution:
	///
	/// Parent, the Fellows plurality, AmbassadorEntities and sibling system parachains' root
	/// get free execution.
	pub type UnpaidExecutionAfter = AllowExplicitUnpaidExecutionFrom<(
		// outside this pallet, when the `Runtime` type is available, the below can be replaced with
		// `RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>`
		Equals<ParentLocation>,
		IsSiblingSystemParachain<ParaId, AssetHubParaId>,
		FellowshipEntities,
		AmbassadorEntities,
	)>;

	/// Locations that will not be charged fees in the executor, either execution or delivery.
	///
	/// We only waive fees for system functions, which these locations represent.
	pub type WaivedLocationsAfter = (
		// outside this pallet, when the `Runtime` type is available, the below can be replaced with
		// `RelayOrOtherSystemParachains<AllSiblingSystemParachains, Runtime>`
		Equals<ParentLocation>,
		IsSiblingSystemParachain<ParaId, AssetHubParaId>,
		FellowshipEntities,
		AmbassadorEntities,
	);
}
