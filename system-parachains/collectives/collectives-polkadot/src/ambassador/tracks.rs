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

const fn percent(x: i32) -> sp_arithmetic::FixedI64 {
	sp_arithmetic::FixedI64::from_rational(x as u128, 100)
}
use crate::{Balance, BlockNumber, RuntimeOrigin, DAYS, DOLLARS, HOURS, MINUTES};
use alloc::borrow::Cow;
use pallet_ranked_collective_ambassador::Rank;
use pallet_referenda::Curve;
use sp_runtime::{str_array as s, traits::Convert, Perbill};

/// Referendum `TrackId` type.
pub type TrackId = u16;

/// Referendum track IDs.
pub mod constants {
	use super::TrackId;

	pub const ASSOCIATE: TrackId = 1;
	pub const LEAD: TrackId = 2;
	pub const SENIOR: TrackId = 3;
	pub const PRINCIPAL: TrackId = 4;
	pub const GLOBAL: TrackId = 5;
	pub const GLOBAL_HEAD: TrackId = 6;

	pub const RETAIN_AT_ASSOCIATE: TrackId = 11;
	pub const RETAIN_AT_LEAD: TrackId = 12;
	pub const RETAIN_AT_SENIOR: TrackId = 13;
	pub const RETAIN_AT_PRINCIPAL: TrackId = 14;
	pub const RETAIN_AT_GLOBAL: TrackId = 15; // this should be opengov vote

	pub const PROMOTE_TO_ASSOCIATE: TrackId = 21;
	pub const PROMOTE_TO_LEAD: TrackId = 22;
	pub const PROMOTE_TO_SENIOR: TrackId = 23;
	pub const PROMOTE_TO_PRINCIPAL: TrackId = 24;
	pub const PROMOTE_TO_GLOBAL: TrackId = 25;

	pub const FAST_PROMOTE_TO_ASSOCIATE: TrackId = 31;
	pub const FAST_PROMOTE_TO_LEAD: TrackId = 32;
	pub const FAST_PROMOTE_TO_SENIOR: TrackId = 33;

	pub const TIP: TrackId = 41;
	pub const TREASURER: TrackId = 42;
}

pub struct MinRankOfClass;
impl Convert<TrackId, Rank> for MinRankOfClass {
	fn convert(a: TrackId) -> Rank {
		match a {
			// Just a regular vote: the track ID is conveniently the same as the minimum rank.
			regular @ 1..=6 => regular,
			// A retention vote; the track ID turns out to be 8 more than the minimum required rank.
			retention @ 11..=15 => retention - 8,
			// A promotion vote; the track ID turns out to be 18 more than the minimum required
			// rank.
			promotion @ 21..=25 => promotion - 18,
			// A fast promotion vote; the track ID turns out to be 28 more than the minimum required
			// rank.
			fast_promote @ 31..=33 => fast_promote - 28,
			// Tipping Origin SeniorAmbassador
			41 => 3,
			// Treasurer Orirign GlobalAmbassador
			42 => 5,
			_ => Rank::MAX,
		}
	}
}

const RETAIN_MAX_DECIDING: u32 = 25;
const RETAIN_DECISION_DEPOSIT: Balance = 5 * DOLLARS;
const RETAIN_PREPARE_PERIOD: BlockNumber = 0;
const RETAIN_DECISION_PERIOD: BlockNumber = 14 * DAYS;
const RETAIN_CONFIRM_PERIOD: BlockNumber = HOURS;
const RETAIN_MIN_ENACTMENT_PERIOD: BlockNumber = 0;
const RETAIN_MIN_APPROVAL: pallet_referenda::Curve = pallet_referenda::Curve::LinearDecreasing {
	length: Perbill::from_percent(100),
	floor: Perbill::from_percent(60),
	ceil: Perbill::from_percent(100),
};
const RETAIN_MIN_SUPPORT: pallet_referenda::Curve = pallet_referenda::Curve::LinearDecreasing {
	length: Perbill::from_percent(100),
	floor: Perbill::from_percent(10),
	ceil: Perbill::from_percent(100),
};

const PROMOTE_MAX_DECIDING: u32 = 10;
const PROMOTE_DECISION_DEPOSIT: Balance = 5 * DOLLARS;
const PROMOTE_PREPARE_PERIOD: BlockNumber = 0;
const PROMOTE_DECISION_PERIOD: BlockNumber = 30 * DAYS;
const PROMOTE_CONFIRM_PERIOD: BlockNumber = HOURS;
const PROMOTE_MIN_ENACTMENT_PERIOD: BlockNumber = 0;
const PROMOTE_MIN_APPROVAL: pallet_referenda::Curve = pallet_referenda::Curve::LinearDecreasing {
	length: Perbill::from_percent(100),
	floor: Perbill::from_percent(60),
	ceil: Perbill::from_percent(100),
};
const PROMOTE_MIN_SUPPORT: pallet_referenda::Curve = pallet_referenda::Curve::LinearDecreasing {
	length: Perbill::from_percent(100),
	floor: Perbill::from_percent(10),
	ceil: Perbill::from_percent(100),
};

const FAST_PROMOTE_MAX_DECIDING: u32 = 10;
const FAST_PROMOTE_DECISION_DEPOSIT: Balance = 5 * DOLLARS;
const FAST_PROMOTE_PREPARE_PERIOD: BlockNumber = 0;
const FAST_PROMOTE_DECISION_PERIOD: BlockNumber = 30 * DAYS;
const FAST_PROMOTE_CONFIRM_PERIOD: BlockNumber = HOURS;
const FAST_PROMOTE_MIN_ENACTMENT_PERIOD: BlockNumber = 0;
const FAST_PROMOTE_MIN_APPROVAL: pallet_referenda::Curve =
	pallet_referenda::Curve::LinearDecreasing {
		length: Perbill::from_percent(100),
		floor: Perbill::from_percent(66),
		ceil: Perbill::from_percent(100),
	};
const FAST_PROMOTE_MIN_SUPPORT: pallet_referenda::Curve =
	pallet_referenda::Curve::LinearDecreasing {
		length: Perbill::from_percent(100),
		floor: Perbill::from_percent(50),
		ceil: Perbill::from_percent(100),
	};

const APP_TIP: Curve = Curve::make_linear(10, 28, percent(50), percent(100));
const SUP_TIP: Curve = Curve::make_reciprocal(1, 28, percent(4), percent(0), percent(50));

const APP_TREASURER: Curve = Curve::make_reciprocal(4, 28, percent(80), percent(50), percent(100));
const SUP_TREASURER: Curve = Curve::make_linear(28, 28, percent(0), percent(50));

/// The type implementing the [`pallet_referenda::TracksInfo`] trait for referenda pallet.
pub struct TracksInfo;

/// Information on the voting tracks.
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = TrackId;

	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;

	/// Return the array of available tracks and their information.
	fn tracks(
	) -> impl Iterator<Item = Cow<'static, pallet_referenda::Track<Self::Id, Balance, BlockNumber>>>
	{
		use constants as tracks;
		const DATA: [pallet_referenda::Track<TrackId, Balance, BlockNumber>; 21] = [
			pallet_referenda::Track {
				id: tracks::ASSOCIATE,
				info: pallet_referenda::TrackInfo {
					name: s("associate"),
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
			},
			pallet_referenda::Track {
				id: tracks::LEAD,
				info: pallet_referenda::TrackInfo {
					name: s("lead"),
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
			},
			pallet_referenda::Track {
				id: tracks::SENIOR,
				info: pallet_referenda::TrackInfo {
					name: s("senior"),
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
			},
			pallet_referenda::Track {
				id: tracks::PRINCIPAL,
				info: pallet_referenda::TrackInfo {
					name: s("principal"),
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
			},
			pallet_referenda::Track {
				id: tracks::GLOBAL,
				info: pallet_referenda::TrackInfo {
					name: s("global"),
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
			},
			pallet_referenda::Track {
				id: tracks::GLOBAL_HEAD,
				info: pallet_referenda::TrackInfo {
					name: s("global head"),
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
			},
			pallet_referenda::Track {
				id: tracks::RETAIN_AT_ASSOCIATE,
				info: pallet_referenda::TrackInfo {
					name: s("retain at associate"),
					max_deciding: RETAIN_MAX_DECIDING,
					decision_deposit: RETAIN_DECISION_DEPOSIT,
					prepare_period: RETAIN_PREPARE_PERIOD,
					decision_period: RETAIN_DECISION_PERIOD,
					confirm_period: RETAIN_CONFIRM_PERIOD,
					min_enactment_period: RETAIN_MIN_ENACTMENT_PERIOD,
					min_approval: RETAIN_MIN_APPROVAL,
					min_support: RETAIN_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::RETAIN_AT_LEAD,
				info: pallet_referenda::TrackInfo {
					name: s("retain at lead"),
					max_deciding: RETAIN_MAX_DECIDING,
					decision_deposit: RETAIN_DECISION_DEPOSIT,
					prepare_period: RETAIN_PREPARE_PERIOD,
					decision_period: RETAIN_DECISION_PERIOD,
					confirm_period: RETAIN_CONFIRM_PERIOD,
					min_enactment_period: RETAIN_MIN_ENACTMENT_PERIOD,
					min_approval: RETAIN_MIN_APPROVAL,
					min_support: RETAIN_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::RETAIN_AT_SENIOR,
				info: pallet_referenda::TrackInfo {
					name: s("retain at senior"),
					max_deciding: RETAIN_MAX_DECIDING,
					decision_deposit: RETAIN_DECISION_DEPOSIT,
					prepare_period: RETAIN_PREPARE_PERIOD,
					decision_period: RETAIN_DECISION_PERIOD,
					confirm_period: RETAIN_CONFIRM_PERIOD,
					min_enactment_period: RETAIN_MIN_ENACTMENT_PERIOD,
					min_approval: RETAIN_MIN_APPROVAL,
					min_support: RETAIN_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::RETAIN_AT_PRINCIPAL,
				info: pallet_referenda::TrackInfo {
					name: s("retain at principal"),
					max_deciding: RETAIN_MAX_DECIDING,
					decision_deposit: RETAIN_DECISION_DEPOSIT,
					prepare_period: RETAIN_PREPARE_PERIOD,
					decision_period: RETAIN_DECISION_PERIOD,
					confirm_period: RETAIN_CONFIRM_PERIOD,
					min_enactment_period: RETAIN_MIN_ENACTMENT_PERIOD,
					min_approval: RETAIN_MIN_APPROVAL,
					min_support: RETAIN_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::RETAIN_AT_GLOBAL,
				info: pallet_referenda::TrackInfo {
					name: s("retain at global"),
					max_deciding: RETAIN_MAX_DECIDING,
					decision_deposit: RETAIN_DECISION_DEPOSIT,
					prepare_period: RETAIN_PREPARE_PERIOD,
					decision_period: RETAIN_DECISION_PERIOD,
					confirm_period: RETAIN_CONFIRM_PERIOD,
					min_enactment_period: RETAIN_MIN_ENACTMENT_PERIOD,
					min_approval: RETAIN_MIN_APPROVAL,
					min_support: RETAIN_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::PROMOTE_TO_ASSOCIATE,
				info: pallet_referenda::TrackInfo {
					name: s("promote to associate"),
					max_deciding: PROMOTE_MAX_DECIDING,
					decision_deposit: PROMOTE_DECISION_DEPOSIT,
					prepare_period: PROMOTE_PREPARE_PERIOD,
					decision_period: PROMOTE_DECISION_PERIOD,
					confirm_period: PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: PROMOTE_MIN_APPROVAL,
					min_support: PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::PROMOTE_TO_LEAD,
				info: pallet_referenda::TrackInfo {
					name: s("promote to lead"),
					max_deciding: PROMOTE_MAX_DECIDING,
					decision_deposit: PROMOTE_DECISION_DEPOSIT,
					prepare_period: PROMOTE_PREPARE_PERIOD,
					decision_period: PROMOTE_DECISION_PERIOD,
					confirm_period: PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: PROMOTE_MIN_APPROVAL,
					min_support: PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::PROMOTE_TO_SENIOR,
				info: pallet_referenda::TrackInfo {
					name: s("promote to senior"),
					max_deciding: PROMOTE_MAX_DECIDING,
					decision_deposit: PROMOTE_DECISION_DEPOSIT,
					prepare_period: PROMOTE_PREPARE_PERIOD,
					decision_period: PROMOTE_DECISION_PERIOD,
					confirm_period: PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: PROMOTE_MIN_APPROVAL,
					min_support: PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::PROMOTE_TO_PRINCIPAL,
				info: pallet_referenda::TrackInfo {
					name: s("promote to principal"),
					max_deciding: PROMOTE_MAX_DECIDING,
					decision_deposit: PROMOTE_DECISION_DEPOSIT,
					prepare_period: PROMOTE_PREPARE_PERIOD,
					decision_period: PROMOTE_DECISION_PERIOD,
					confirm_period: PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: PROMOTE_MIN_APPROVAL,
					min_support: PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::PROMOTE_TO_GLOBAL,
				info: pallet_referenda::TrackInfo {
					name: s("promote to global"),
					max_deciding: PROMOTE_MAX_DECIDING,
					decision_deposit: PROMOTE_DECISION_DEPOSIT,
					prepare_period: PROMOTE_PREPARE_PERIOD,
					decision_period: PROMOTE_DECISION_PERIOD,
					confirm_period: PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: PROMOTE_MIN_APPROVAL,
					min_support: PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::FAST_PROMOTE_TO_ASSOCIATE,
				info: pallet_referenda::TrackInfo {
					name: s("fast promote to associate"),
					max_deciding: FAST_PROMOTE_MAX_DECIDING,
					decision_deposit: FAST_PROMOTE_DECISION_DEPOSIT,
					prepare_period: FAST_PROMOTE_PREPARE_PERIOD,
					decision_period: FAST_PROMOTE_DECISION_PERIOD,
					confirm_period: FAST_PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: FAST_PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: FAST_PROMOTE_MIN_APPROVAL,
					min_support: FAST_PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::FAST_PROMOTE_TO_LEAD,
				info: pallet_referenda::TrackInfo {
					name: s("fast promote to lead"),
					max_deciding: FAST_PROMOTE_MAX_DECIDING,
					decision_deposit: FAST_PROMOTE_DECISION_DEPOSIT,
					prepare_period: FAST_PROMOTE_PREPARE_PERIOD,
					decision_period: FAST_PROMOTE_DECISION_PERIOD,
					confirm_period: FAST_PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: FAST_PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: FAST_PROMOTE_MIN_APPROVAL,
					min_support: FAST_PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::FAST_PROMOTE_TO_SENIOR,
				info: pallet_referenda::TrackInfo {
					name: s("fast promote to senior"),
					max_deciding: FAST_PROMOTE_MAX_DECIDING,
					decision_deposit: FAST_PROMOTE_DECISION_DEPOSIT,
					prepare_period: FAST_PROMOTE_PREPARE_PERIOD,
					decision_period: FAST_PROMOTE_DECISION_PERIOD,
					confirm_period: FAST_PROMOTE_CONFIRM_PERIOD,
					min_enactment_period: FAST_PROMOTE_MIN_ENACTMENT_PERIOD,
					min_approval: FAST_PROMOTE_MIN_APPROVAL,
					min_support: FAST_PROMOTE_MIN_SUPPORT,
				},
			},
			pallet_referenda::Track {
				id: tracks::TIP,
				info: pallet_referenda::TrackInfo {
					name: s("tip"),
					max_deciding: 200,
					decision_deposit: DOLLARS * 10, // 1 DOT
					prepare_period: MINUTES,
					decision_period: 7 * DAYS,
					confirm_period: 10 * MINUTES,
					min_enactment_period: MINUTES,
					min_approval: APP_TIP,
					min_support: SUP_TIP,
				},
			},
			pallet_referenda::Track {
				id: tracks::TREASURER,
				info: pallet_referenda::TrackInfo {
					name: s("treasurer"),
					max_deciding: 10,
					decision_deposit: DOLLARS, // 1,000 DOT
					prepare_period: 2 * HOURS,
					decision_period: 28 * DAYS,
					confirm_period: 7 * DAYS,
					min_enactment_period: 24 * HOURS,
					min_approval: APP_TREASURER,
					min_support: SUP_TREASURER,
				},
			},
		];
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
				return Ok(tracks::GLOBAL_HEAD)
			}
		}

		match Origin::try_from(id.clone()) {
			Ok(Origin::AssociateAmbassador) => Ok(tracks::ASSOCIATE),
			Ok(Origin::LeadAmbassador) => Ok(tracks::LEAD),
			Ok(Origin::SeniorAmbassador) => Ok(tracks::SENIOR),
			Ok(Origin::PrincipalAmbassador) => Ok(tracks::PRINCIPAL),
			Ok(Origin::GlobalAmbassador) => Ok(tracks::GLOBAL),
			Ok(Origin::GlobalHeadAmbassador) => Ok(tracks::GLOBAL_HEAD),

			Ok(Origin::RetainAtAssociateAmbassador) => Ok(tracks::RETAIN_AT_ASSOCIATE),
			Ok(Origin::RetainAtLeadAmbassador) => Ok(tracks::RETAIN_AT_LEAD),
			Ok(Origin::RetainAtSeniorAmbassador) => Ok(tracks::RETAIN_AT_SENIOR),
			Ok(Origin::RetainAtPrincipalAmbassador) => Ok(tracks::RETAIN_AT_PRINCIPAL),

			Ok(Origin::PromoteToAssociateAmbassador) => Ok(tracks::PROMOTE_TO_ASSOCIATE),
			Ok(Origin::PromoteToLeadAmbassador) => Ok(tracks::PROMOTE_TO_LEAD),
			Ok(Origin::PromoteToSeniorAmbassador) => Ok(tracks::PROMOTE_TO_SENIOR),
			Ok(Origin::PromoteToPrincipalAmbassador) => Ok(tracks::PROMOTE_TO_PRINCIPAL),

			Ok(Origin::FastPromoteToAssociateAmbassador) => Ok(tracks::FAST_PROMOTE_TO_ASSOCIATE),
			Ok(Origin::FastPromoteToLeadAmbassador) => Ok(tracks::FAST_PROMOTE_TO_LEAD),
			Ok(Origin::FastPromoteToSeniorAmbassador) => Ok(tracks::FAST_PROMOTE_TO_SENIOR),

			Ok(Origin::Tip) => Ok(tracks::TIP),
			Ok(Origin::Treasurer) => Ok(tracks::TREASURER),
			_ => Err(()),
		}
	}
}

// implements [`frame_support::traits::Get`] for [`TracksInfo`]
