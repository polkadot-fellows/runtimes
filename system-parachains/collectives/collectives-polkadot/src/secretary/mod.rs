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

//! The Polkadot Secretary Collective.

use core::marker::PhantomData;

use crate::{fellowship::FellowshipAdminBodyId, *};
use frame_support::{
	parameter_types,
	traits::{tokens::GetSalary, EitherOf, MapSuccess, PalletInfoAccess, PollStatus, Polling},
};
use frame_system::{pallet_prelude::BlockNumberFor, EnsureRootWithSuccess};
use pallet_ranked_collective::{MemberIndex, TallyOf, Votes};
use pallet_xcm::{EnsureXcm, IsVoiceOfBody};
use polkadot_runtime_constants::time::HOURS;
use sp_core::{ConstU128, ConstU32};
use sp_runtime::{
	traits::{ConstU16, ConvertToValue, Identity, Replace},
	DispatchError,
};

use xcm::prelude::*;
use xcm_builder::{AliasesIntoAccountId32, PayOverXcm};

use self::xcm_config::AssetHubUsdt;

/// The Secretary members' ranks.
pub mod ranks {
	use pallet_ranked_collective::Rank;

	pub const SECRETARY_CANDIDATE: Rank = 0;
	pub const SECRETARY: Rank = 1;
}

/// Origins of:
/// - Root;
/// - FellowshipAdmin (i.e. token holder referendum);
/// - Plurality vote from Fellows can promote, demote, remove and approve rank retention of members
///   of the Secretary Collective (rank `2`).
type ApproveOrigin = EitherOf<
	EnsureRootWithSuccess<AccountId, ConstU16<65535>>,
	EitherOf<
		EitherOf<
			MapSuccess<
				EnsureXcm<IsVoiceOfBody<RelayChainLocation, FellowshipAdminBodyId>>,
				Replace<ConstU16<65535>>,
			>,
			MapSuccess<
				EnsureXcm<IsVoiceOfBody<AssetHubLocation, FellowshipAdminBodyId>>,
				Replace<ConstU16<65535>>,
			>,
		>,
		MapSuccess<Fellows, Replace<ConstU16<65535>>>,
	>,
>;

pub struct SecretaryPolling<T: pallet_ranked_collective::Config<I>, I: 'static>(
	PhantomData<(T, I)>,
);

impl<T: pallet_ranked_collective::Config<I>, I: 'static> Polling<TallyOf<T, I>>
	for SecretaryPolling<T, I>
{
	type Index = MemberIndex;
	type Votes = Votes;
	type Class = u16;
	type Moment = BlockNumberFor<T>;

	fn classes() -> Vec<Self::Class> {
		vec![]
	}

	fn as_ongoing(_index: Self::Index) -> Option<(TallyOf<T, I>, Self::Class)> {
		None
	}

	fn access_poll<R>(
		_index: Self::Index,
		f: impl FnOnce(PollStatus<&mut TallyOf<T, I>, Self::Moment, Self::Class>) -> R,
	) -> R {
		f(PollStatus::None)
	}

	fn try_access_poll<R>(
		_index: Self::Index,
		f: impl FnOnce(
			PollStatus<&mut TallyOf<T, I>, Self::Moment, Self::Class>,
		) -> Result<R, DispatchError>,
	) -> Result<R, DispatchError> {
		f(PollStatus::None)
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn create_ongoing(_class: Self::Class) -> Result<Self::Index, ()> {
		Err(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn end_ongoing(_index: Self::Index, _approved: bool) -> Result<(), ()> {
		Err(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn max_ongoing() -> (Self::Class, u32) {
		(0, 0)
	}
}

pub type SecretaryCollectiveInstance = pallet_ranked_collective::Instance3;

impl pallet_ranked_collective::Config<SecretaryCollectiveInstance> for Runtime {
	type WeightInfo = weights::pallet_ranked_collective_secretary_collective::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = ApproveOrigin;
	type RemoveOrigin = ApproveOrigin;
	type PromoteOrigin = ApproveOrigin;
	type DemoteOrigin = ApproveOrigin;
	type ExchangeOrigin = ApproveOrigin;
	type Polls = SecretaryPolling<Runtime, SecretaryCollectiveInstance>;
	type MinRankOfClass = Identity;
	type MemberSwappedHandler = crate::SecretarySalary;
	type VoteWeight = pallet_ranked_collective::Geometric;
	type MaxMemberCount = ();
	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkSetup = crate::SecretarySalary;
}

pub type SecretarySalaryInstance = pallet_salary::Instance3;

parameter_types! {
	// The interior location on AssetHub for the paying account. This is the Secretary Salary
	// pallet instance. This sovereign account will need funding.
	pub SecretarySalaryInteriorLocation: InteriorLocation = PalletInstance(<crate::SecretarySalary as PalletInfoAccess>::index() as u8).into();
}

const USDT_UNITS: u128 = 1_000_000;

/// [`PayOverXcm`] setup to pay the Secretary salary on the AssetHub in USDT.
pub type SecretarySalaryPaymaster = PayOverXcm<
	SecretarySalaryInteriorLocation,
	crate::xcm_config::XcmRouter,
	crate::PolkadotXcm,
	ConstU32<{ 6 * HOURS }>,
	AccountId,
	(),
	ConvertToValue<AssetHubUsdt>,
	AliasesIntoAccountId32<(), AccountId>,
>;

pub struct SalaryForRank;
impl GetSalary<u16, AccountId, Balance> for SalaryForRank {
	fn get_salary(rank: u16, _who: &AccountId) -> Balance {
		if rank == 1 {
			6666 * USDT_UNITS
		} else {
			0
		}
	}
}

impl pallet_salary::Config<SecretarySalaryInstance> for Runtime {
	type WeightInfo = weights::pallet_salary_secretary_salary::WeightInfo<Runtime>;
	type RuntimeEvent = RuntimeEvent;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Paymaster = SecretarySalaryPaymaster;
	#[cfg(feature = "runtime-benchmarks")]
	type Paymaster = crate::impls::benchmarks::PayWithEnsure<
		SecretarySalaryPaymaster,
		crate::impls::benchmarks::OpenHrmpChannel<ConstU32<1000>>,
	>;
	type Members = pallet_ranked_collective::Pallet<Runtime, SecretaryCollectiveInstance>;

	#[cfg(not(feature = "runtime-benchmarks"))]
	type Salary = SalaryForRank;
	#[cfg(feature = "runtime-benchmarks")]
	type Salary = frame_support::traits::tokens::ConvertRank<
		crate::impls::benchmarks::RankToSalary<Balances>,
	>;
	// 15 days to register for a salary payment.
	type RegistrationPeriod = ConstU32<{ 15 * DAYS }>;
	// 15 days to claim the salary payment.
	type PayoutPeriod = ConstU32<{ 15 * DAYS }>;
	// Total monthly salary budget.
	type Budget = ConstU128<{ 6666 * USDT_UNITS }>;
}
