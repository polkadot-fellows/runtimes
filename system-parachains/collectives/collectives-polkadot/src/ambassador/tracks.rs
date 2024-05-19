// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! The Ambassador Program's referenda voting tracks.

use super::Origin;
use crate::{Balance, BlockNumber, RuntimeOrigin, DAYS, DOLLARS, HOURS};
use sp_runtime::Perbill;

/// Referendum `TrackId` type.
pub type TrackId = u16;

/// Referendum track IDs.
pub mod constants {
	use super::TrackId;

	pub const AMBASSADOR: TrackId = 1;
	pub const SENIOR_AMBASSADOR: TrackId = 2;
	pub const HEAD_AMBASSADOR: TrackId = 3;
}

/// The type implementing the [`pallet_referenda::TracksInfo`] trait for referenda pallet.
pub struct TracksInfo;

/// Information on the voting tracks.
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = TrackId;

	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;

	/// Return the array of available tracks and their information.
	fn tracks() -> &'static [(Self::Id, pallet_referenda::TrackInfo<Balance, BlockNumber>)] {
		static DATA: [(TrackId, pallet_referenda::TrackInfo<Balance, BlockNumber>); 3] = [
			(
				constants::AMBASSADOR,
				pallet_referenda::TrackInfo {
					name: "ambassador",
					max_deciding: 10,
					decision_deposit: 5 * DOLLARS,
					prepare_period: 24 * HOURS,
					decision_period: 7 * DAYS,
					confirm_period: 24 * HOURS,
					min_enactment_period: HOURS,
					min_approval: pallet_referenda::Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(50),
						ceil: Perbill::from_percent(100),
					},
					min_support: pallet_referenda::Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(0),
						ceil: Perbill::from_percent(50),
					},
				},
			),
			(
				constants::SENIOR_AMBASSADOR,
				pallet_referenda::TrackInfo {
					name: "senior ambassador",
					max_deciding: 10,
					decision_deposit: 5 * DOLLARS,
					prepare_period: 24 * HOURS,
					decision_period: 7 * DAYS,
					confirm_period: 24 * HOURS,
					min_enactment_period: HOURS,
					min_approval: pallet_referenda::Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(50),
						ceil: Perbill::from_percent(100),
					},
					min_support: pallet_referenda::Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(0),
						ceil: Perbill::from_percent(50),
					},
				},
			),
			(
				constants::HEAD_AMBASSADOR,
				pallet_referenda::TrackInfo {
					name: "head ambassador",
					max_deciding: 10,
					decision_deposit: 5 * DOLLARS,
					prepare_period: 24 * HOURS,
					decision_period: 7 * DAYS,
					confirm_period: 24 * HOURS,
					min_enactment_period: HOURS,
					min_approval: pallet_referenda::Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(50),
						ceil: Perbill::from_percent(100),
					},
					min_support: pallet_referenda::Curve::LinearDecreasing {
						length: Perbill::from_percent(100),
						floor: Perbill::from_percent(0),
						ceil: Perbill::from_percent(50),
					},
				},
			),
		];
		&DATA[..]
	}

	/// Determine the voting track for the given `origin`.
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		#[cfg(feature = "runtime-benchmarks")]
		{
			// For benchmarks, we enable a root origin.
			// It is important that this is not available in production!
			let root: Self::RuntimeOrigin = frame_system::RawOrigin::Root.into();
			if &root == id {
				return Ok(constants::HEAD_AMBASSADOR)
			}
		}

		match Origin::try_from(id.clone()) {
			Ok(Origin::Ambassadors) => Ok(constants::AMBASSADOR),
			Ok(Origin::SeniorAmbassadors) => Ok(constants::SENIOR_AMBASSADOR),
			Ok(Origin::HeadAmbassadors) => Ok(constants::HEAD_AMBASSADOR),
			_ => Err(()),
		}
	}
}

// implements [`frame_support::traits::Get`] for [`TracksInfo`]
pallet_referenda::impl_tracksinfo_get!(TracksInfo, Balance, BlockNumber);
