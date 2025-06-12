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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Code shared between all runtimes
#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;
use frame_support::traits::{Contains, OriginTrait};
use xcm::latest::{Location, OriginKind};
use xcm_executor::traits::ConvertOrigin;

/// TODO: `LocationAsSuperuser` is temporary placed here, the final solution will be imported from
/// `xcm_builder` (depends on backports) instead.
///
/// A converter that allows a specific `Location` to act as a superuser (`RuntimeOrigin::root()`)
/// if it matches the predefined `SuperuserLocation` filter and `OriginKind::Superuser`.
pub struct LocationAsSuperuser<SuperuserLocation, RuntimeOrigin>(
	PhantomData<(SuperuserLocation, RuntimeOrigin)>,
);
impl<SuperuserLocation: Contains<Location>, RuntimeOrigin: OriginTrait> ConvertOrigin<RuntimeOrigin>
	for LocationAsSuperuser<SuperuserLocation, RuntimeOrigin>
{
	fn convert_origin(
		origin: impl Into<Location>,
		kind: OriginKind,
	) -> Result<RuntimeOrigin, Location> {
		let origin = origin.into();
		log::trace!(target: "xcm::origin_conversion", "LocationAsSuperuser origin: {:?}, kind: {:?}", origin, kind);
		match (kind, &origin) {
			(OriginKind::Superuser, loc) if SuperuserLocation::contains(loc) =>
				Ok(RuntimeOrigin::root()),
			_ => Err(origin),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use frame_support::{construct_runtime, derive_impl, parameter_types, traits::Equals};
	use xcm::latest::{Junction::*, Junctions::*, OriginKind};

	type Block = frame_system::mocking::MockBlock<Test>;

	construct_runtime!(
		pub enum Test
		{
			System: frame_system,
		}
	);

	#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
	impl frame_system::Config for Test {
		type Block = Block;
	}

	parameter_types! {
		pub SuperuserLocation: Location = Location::new(0, Parachain(1));
	}

	#[test]
	fn superuser_location_works() {
		let test_conversion = |loc, kind| {
			LocationAsSuperuser::<Equals<SuperuserLocation>, RuntimeOrigin>::convert_origin(
				loc, kind,
			)
		};

		// Location that was set as SuperUserLocation should result in success conversion to Root
		assert!(matches!(test_conversion(SuperuserLocation::get(), OriginKind::Superuser), Ok(..)));
		// Same Location as SuperUserLocation::get()
		assert!(matches!(
			test_conversion(Location::new(0, Parachain(1)), OriginKind::Superuser),
			Ok(..)
		));

		// Same Location but different origin kind
		assert!(matches!(test_conversion(SuperuserLocation::get(), OriginKind::Native), Err(..)));
		assert!(matches!(
			test_conversion(SuperuserLocation::get(), OriginKind::SovereignAccount),
			Err(..)
		));
		assert!(matches!(test_conversion(SuperuserLocation::get(), OriginKind::Xcm), Err(..)));

		// No other location should result in successful conversion to Root
		// thus expecting Err in all cases below
		//
		// Non-matching parachain number
		assert!(matches!(
			test_conversion(Location::new(0, Parachain(2)), OriginKind::Superuser),
			Err(..)
		));
		// Non-matching parents count
		assert!(matches!(
			test_conversion(Location::new(1, Parachain(1)), OriginKind::Superuser),
			Err(..)
		));
		// Child location of SuperUserLocation
		assert!(matches!(
			test_conversion(
				Location::new(1, [Parachain(1), GeneralIndex(0)]),
				OriginKind::Superuser
			),
			Err(..)
		));
		// Here
		assert!(matches!(test_conversion(Location::new(0, Here), OriginKind::Superuser), Err(..)));
		// Parent
		assert!(matches!(test_conversion(Location::new(1, Here), OriginKind::Superuser), Err(..)));
		// Some random account
		assert!(matches!(
			test_conversion(
				Location::new(0, AccountId32 { network: None, id: [0u8; 32] }),
				OriginKind::Superuser
			),
			Err(..)
		));
		// Child location of SuperUserLocation
		assert!(matches!(
			test_conversion(
				Location::new(0, [Parachain(1), AccountId32 { network: None, id: [1u8; 32] }]),
				OriginKind::Superuser
			),
			Err(..)
		));
	}
}
