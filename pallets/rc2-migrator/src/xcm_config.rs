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

//! XCM adapters for the Relay Chain runtime, keyed on the migration stage.

use crate::{Config, RcMigrationStage};
use core::marker::PhantomData;
use frame_support::traits::ContainsPair;
use xcm::latest::prelude::*;

/// `Inner` until the migration starts, nothing after.
pub struct NothingOnceStarted<T, Inner>(PhantomData<(T, Inner)>);
impl<T: Config, Inner: ContainsPair<Asset, Location>> ContainsPair<Asset, Location>
	for NothingOnceStarted<T, Inner>
{
	fn contains(asset: &Asset, origin: &Location) -> bool {
		Inner::contains(asset, origin) && !RcMigrationStage::<T>::get().has_started()
	}
}
