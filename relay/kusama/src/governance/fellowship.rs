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

//! Elements of governance concerning the Polkadot Fellowship. This is only a temporary arrangement
//! since the Polkadot Fellowship belongs under the Polkadot Relay. However, that is not yet in
//! place, so until then it will need to live here. Once it is in place and there exists a bridge
//! between Polkadot/Kusama then this code can be removed.

use alloc::borrow::Cow;
use frame_support::traits::{MapSuccess, TryMapSuccess};
use sp_arithmetic::traits::CheckedSub;
use sp_runtime::{
	morph_types, str_array as s,
	traits::{Replace, ReplaceWithDefault, TypedGet},
};

use super::*;

parameter_types! {
	pub const AlarmInterval: BlockNumber = 1;
	pub const SubmissionDeposit: Balance = 0;
	pub const UndecidingTimeout: BlockNumber = 7 * DAYS;
}

const TRACKS_DATA: [pallet_referenda::Track<u16, Balance, BlockNumber>; 10] = [
	pallet_referenda::Track {
		id: 0,
		info: pallet_referenda::TrackInfo {
			name: s("candidates"),
			max_deciding: 10,
			decision_deposit: 100 * QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 1,
		info: pallet_referenda::TrackInfo {
			name: s("members"),
			max_deciding: 10,
			decision_deposit: 10 * QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 2,
		info: pallet_referenda::TrackInfo {
			name: s("proficients"),
			max_deciding: 10,
			decision_deposit: 10 * QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 3,
		info: pallet_referenda::TrackInfo {
			name: s("fellows"),
			max_deciding: 10,
			decision_deposit: 10 * QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 4,
		info: pallet_referenda::TrackInfo {
			name: s("senior fellows"),
			max_deciding: 10,
			decision_deposit: 10 * QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 5,
		info: pallet_referenda::TrackInfo {
			name: s("experts"),
			max_deciding: 10,
			decision_deposit: QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 6,
		info: pallet_referenda::TrackInfo {
			name: s("senior experts"),
			max_deciding: 10,
			decision_deposit: QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 7,
		info: pallet_referenda::TrackInfo {
			name: s("masters"),
			max_deciding: 10,
			decision_deposit: QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 8,
		info: pallet_referenda::TrackInfo {
			name: s("senior masters"),
			max_deciding: 10,
			decision_deposit: QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
		id: 9,
		info: pallet_referenda::TrackInfo {
			name: s("grand masters"),
			max_deciding: 10,
			decision_deposit: QUID,
			prepare_period: 30 * MINUTES,
			decision_period: 7 * DAYS,
			confirm_period: 30 * MINUTES,
			min_enactment_period: MINUTES,
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
];

pub struct TracksInfo;
impl pallet_referenda::TracksInfo<Balance, BlockNumber> for TracksInfo {
	type Id = u16;
	type RuntimeOrigin = <RuntimeOrigin as frame_support::traits::OriginTrait>::PalletsOrigin;
	fn tracks(
	) -> impl Iterator<Item = Cow<'static, pallet_referenda::Track<Self::Id, Balance, BlockNumber>>>
	{
		TRACKS_DATA.iter().map(Cow::Borrowed)
	}
	fn track_for(id: &Self::RuntimeOrigin) -> Result<Self::Id, ()> {
		use super::origins::Origin;

		#[cfg(feature = "runtime-benchmarks")]
		{
			// For benchmarks, we enable a root origin.
			// It is important that this is not available in production!
			let root: Self::RuntimeOrigin = frame_system::RawOrigin::Root.into();
			if &root == id {
				return Ok(9)
			}
		}

		match Origin::try_from(id.clone()) {
			Ok(Origin::FellowshipInitiates) => Ok(0),
			Ok(Origin::Fellowship1Dan) => Ok(1),
			Ok(Origin::Fellowship2Dan) => Ok(2),
			Ok(Origin::Fellowship3Dan) | Ok(Origin::Fellows) => Ok(3),
			Ok(Origin::Fellowship4Dan) => Ok(4),
			Ok(Origin::Fellowship5Dan) | Ok(Origin::FellowshipExperts) => Ok(5),
			Ok(Origin::Fellowship6Dan) => Ok(6),
			Ok(Origin::Fellowship7Dan | Origin::FellowshipMasters) => Ok(7),
			Ok(Origin::Fellowship8Dan) => Ok(8),
			Ok(Origin::Fellowship9Dan) => Ok(9),
			_ => Err(()),
		}
	}
}

pub type FellowshipReferendaInstance = pallet_referenda::Instance2;

impl pallet_referenda::Config<FellowshipReferendaInstance> for Runtime {
	type WeightInfo = weights::pallet_referenda_fellowship_referenda::WeightInfo<Self>;
	type RuntimeCall = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type Scheduler = Scheduler;
	type Currency = Balances;
	type SubmitOrigin =
		pallet_ranked_collective::EnsureMember<Runtime, FellowshipCollectiveInstance, 1>;
	type CancelOrigin = FellowshipExperts;
	type KillOrigin = FellowshipMasters;
	type Slash = Treasury;
	type Votes = pallet_ranked_collective::Votes;
	type Tally = pallet_ranked_collective::TallyOf<Runtime, FellowshipCollectiveInstance>;
	type SubmissionDeposit = SubmissionDeposit;
	type MaxQueued = ConstU32<100>;
	type UndecidingTimeout = UndecidingTimeout;
	type AlarmInterval = AlarmInterval;
	type Tracks = TracksInfo;
	type Preimages = Preimage;
	type BlockNumberProvider = System;
}

pub type FellowshipCollectiveInstance = pallet_ranked_collective::Instance1;

morph_types! {
	/// A `TryMorph` implementation to reduce a scalar by a particular amount, checking for
	/// underflow.
	pub type CheckedReduceBy<N: TypedGet>: TryMorph = |r: N::Type| -> Result<N::Type, ()> {
		r.checked_sub(&N::get()).ok_or(())
	} where N::Type: CheckedSub;
}

impl pallet_ranked_collective::Config<FellowshipCollectiveInstance> for Runtime {
	type WeightInfo = weights::pallet_ranked_collective::WeightInfo<Self>;
	type RuntimeEvent = RuntimeEvent;
	// Promotion is by any of:
	// - Root can demote arbitrarily.
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote by the rank *above* the new rank.
	type PromoteOrigin = EitherOf<
		frame_system::EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>,
		EitherOf<
			MapSuccess<FellowshipAdmin, Replace<ConstU16<9>>>,
			TryMapSuccess<origins::EnsureFellowship, CheckedReduceBy<ConstU16<1>>>,
		>,
	>;
	// Demotion is by any of:
	// - Root can demote arbitrarily.
	// - the FellowshipAdmin origin (i.e. token holder referendum);
	// - a vote by the rank two above the current rank.
	type DemoteOrigin = EitherOf<
		frame_system::EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>,
		EitherOf<
			MapSuccess<FellowshipAdmin, Replace<ConstU16<9>>>,
			TryMapSuccess<origins::EnsureFellowship, CheckedReduceBy<ConstU16<2>>>,
		>,
	>;
	// Exchange is by any of:
	// - Root can exchange arbitrarily.
	// - the Fellows origin
	type ExchangeOrigin =
		EitherOf<frame_system::EnsureRootWithSuccess<Self::AccountId, ConstU16<65535>>, Fellows>;
	type AddOrigin = MapSuccess<Self::PromoteOrigin, ReplaceWithDefault<()>>;
	type RemoveOrigin = Self::DemoteOrigin;
	type Polls = FellowshipReferenda;
	type MinRankOfClass = sp_runtime::traits::Identity;
	type MemberSwappedHandler = ();
	type VoteWeight = pallet_ranked_collective::Geometric;
	type MaxMemberCount = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = ();
}
