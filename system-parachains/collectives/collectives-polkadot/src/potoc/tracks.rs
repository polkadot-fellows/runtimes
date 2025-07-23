// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Track configurations for PoToC.

use alloc::borrow::Cow;
use crate::{Balance, BlockNumber, RuntimeOrigin, DAYS, DOLLARS, HOURS, MINUTES};
use polkadot_runtime_common::prod_or_fast;
use sp_runtime::Perbill;
use sp_runtime::str_array as s;

/// Referendum `TrackId` type.
pub type TrackId = u16;

/// Referendum track IDs.
pub mod constants {
	use super::TrackId;

	/// The members track.
	pub const MEMBERS: TrackId = 1;
}

/// The type implementing the [`pallet_referenda::TracksInfo`] trait for referenda pallet.
pub struct TracksInfo;

/// Information on the voting tracks.
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = TrackId;

	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;

	/// Return the array of available tracks and their information.
	fn tracks(	
	) -> impl Iterator<Item = Cow<'static, pallet_referenda::Track<Self::Id, Balance, BlockNumber>>> {
		use constants as tracks;
		const DATA: [pallet_referenda::Track<TrackId, Balance, BlockNumber>; 1] = [
		pallet_referenda::Track {
			id: tracks::MEMBERS,
			info: pallet_referenda::TrackInfo {
				name: s("members"),
				max_deciding: 10,
				decision_deposit: 5 * DOLLARS,
				prepare_period: prod_or_fast!(24 * HOURS, 1 * MINUTES),
				decision_period: prod_or_fast!(7 * DAYS, 5 * MINUTES),
				confirm_period: prod_or_fast!(24 * HOURS, 1 * MINUTES),
				min_enactment_period: prod_or_fast!(HOURS, 1 * MINUTES),
				min_approval: pallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(50),
					ceil: Perbill::from_percent(100),
				},
				min_support: pallet_referenda::Curve::LinearDecreasing {
					length: Perbill::from_percent(100),
					floor: Perbill::from_percent(0),
					ceil: Perbill::from_percent(100),
				},
			},
		},];
		DATA.iter().map(Cow::Borrowed)
	}

	/// Determine the voting track for the given `origin`.
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		use super::origins::Origin;
		use constants as tracks;

		#[cfg(feature = "runtime-benchmarks")]
		{
			// For benchmarks, we enable a root origin.
			// It is important that this is not available in production!
			let root: Self::RuntimeOrigin = frame_system::RawOrigin::Root.into();
			if &root == id {
				return Ok(tracks::MEMBERS)
			}
		}

		match Origin::try_from(id.clone()) {
			Ok(Origin::Members) => Ok(tracks::MEMBERS),
			_ => Err(()),
		}
	}
}

