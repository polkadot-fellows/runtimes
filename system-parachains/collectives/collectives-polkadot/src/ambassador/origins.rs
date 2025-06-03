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

//! The Ambassador Program's origins.

pub use pallet_origins::*;

#[frame_support::pallet]
pub mod pallet_origins {
	use crate::{ambassador::ranks, Balance};
	use frame_support::pallet_prelude::*;
	use pallet_ranked_collective_ambassador::Rank;
	use polkadot_runtime_constants::currency::DOLLARS;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	/// The pallet configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[derive(PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, TypeInfo, RuntimeDebug)]
	#[pallet::origin]
	pub enum Origin {
		/// Plurality voice of the [ranks::AMBASSADOR] members or above given via
		/// referendum.
		AssociateAmbassador,
		/// Plurality voice of the [ranks::SENIOR_AMBASSADOR] members or above given via
		/// referendum.
		LeadAmbassador,
		SeniorAmbassador,
		PrincipalAmbassador,
		GlobalAmbassador,
		/// Plurality voice of the [ranks::HEAD_AMBASSADOR] members given via referendum.
		GlobalHeadAmbassador,
		RetainAtAssociateAmbassador,
		RetainAtLeadAmbassador,
		RetainAtSeniorAmbassador,
		RetainAtPrincipalAmbassador,
		PromoteToAssociateAmbassador,
		PromoteToLeadAmbassador,
		PromoteToSeniorAmbassador,
		PromoteToPrincipalAmbassador,
		FastPromoteToAssociateAmbassador,
		FastPromoteToLeadAmbassador,
		FastPromoteToSeniorAmbassador,
		Tip,
		Treasurer,
	}

	impl Origin {
		/// Returns the rank that the origin `self` speaks for, or `None` if it doesn't speak for
		/// any.
		///
		/// `Some` will be returned only for the first 9 elements of [Origin].
		pub fn as_voice(&self) -> Option<pallet_ranked_collective_ambassador::Rank> {
			Some(match &self {
				Origin::AssociateAmbassador => ranks::ASSOCIATE_AMBASSADOR,
				Origin::LeadAmbassador => ranks::LEAD_AMBASSADOR,
				Origin::SeniorAmbassador => ranks::SENIOR_AMBASSADOR,
				Origin::PrincipalAmbassador => ranks::PRINCIPAL_AMBASSADOR,
				Origin::GlobalAmbassador => ranks::GLOBAL_AMBASSADOR,
				Origin::GlobalHeadAmbassador => ranks::GLOBAL_HEAD_AMBASSADOR,
				_ => return None,
			})
		}
	}

	/*
	/// A `TryMorph` implementation which is designed to convert an aggregate `RuntimeOrigin`
	/// value into the Fellowship voice it represents if it is a Fellowship pallet origin an
	/// appropriate variant.
	///
	/// See also [Origin::as_voice].
	pub struct ToVoice;
	impl<'a, O: 'a + TryInto<&'a Origin>> sp_runtime::traits::TryMorph<O> for ToVoice {
		type Outcome = pallet_ranked_collective_ambassador::Rank;
		fn try_morph(o: O) -> Result<pallet_ranked_collective_ambassador::Rank, ()> {
			o.try_into().ok().and_then(Origin::as_voice).ok_or(())
		}
	}
	*/

	pub struct EnsureCanRetainAt;

	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for EnsureCanRetainAt {
		type Success = Rank;

		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::RetainAtAssociateAmbassador => Ok(ranks::ASSOCIATE_AMBASSADOR),
				Origin::RetainAtLeadAmbassador => Ok(ranks::LEAD_AMBASSADOR),
				Origin::RetainAtSeniorAmbassador => Ok(ranks::SENIOR_AMBASSADOR),
				Origin::RetainAtPrincipalAmbassador => Ok(ranks::PRINCIPAL_AMBASSADOR),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::RetainAtPrincipalAmbassador))
		}
	}

	pub struct EnsureCanPromoteTo;

	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for EnsureCanPromoteTo {
		type Success = Rank;

		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::PromoteToAssociateAmbassador => Ok(ranks::ASSOCIATE_AMBASSADOR),
				Origin::PromoteToLeadAmbassador => Ok(ranks::LEAD_AMBASSADOR),
				Origin::PromoteToSeniorAmbassador => Ok(ranks::SENIOR_AMBASSADOR),
				Origin::PromoteToPrincipalAmbassador => Ok(ranks::PRINCIPAL_AMBASSADOR),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::PromoteToPrincipalAmbassador))
		}
	}

	pub struct EnsureCanFastPromoteTo;

	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for EnsureCanFastPromoteTo {
		type Success = Rank;

		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::FastPromoteToAssociateAmbassador => Ok(ranks::ASSOCIATE_AMBASSADOR),
				Origin::FastPromoteToLeadAmbassador => Ok(ranks::LEAD_AMBASSADOR),
				Origin::FastPromoteToSeniorAmbassador => Ok(ranks::SENIOR_AMBASSADOR),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::FastPromoteToSeniorAmbassador))
		}
	}

	pub struct Spender;

	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for Spender {
		type Success = Balance; // ← Changed to Balance

		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::Tip => Ok(250 * DOLLARS),          // ← Return value
				Origin::Treasurer => Ok(10_000 * DOLLARS), // ← Return value
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			// Use highest-privilege origin for benchmarks
			Ok(O::from(Origin::Treasurer))
		}
	}

	/// Ensures [`Origin::GlobalHeadAmbassador`] origin.
	pub struct GlobalHeadAmbassador;
	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for GlobalHeadAmbassador {
		type Success = ();
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::GlobalHeadAmbassador => Ok(()),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::GlobalHeadAmbassador))
		}
	}

	/// Ensures [`Origin::SeniorAmbassador`] origin.
	pub struct SeniorAmbassador;
	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for SeniorAmbassador {
		type Success = ();
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::SeniorAmbassador => Ok(()),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::SeniorAmbassador))
		}
	}

	/// Ensures that the origin is a plurality voice of the a given rank `R` or above.
	/// Success is the corresponding origin rank.
	pub struct EnsureAmbassadorsFrom<R>(PhantomData<R>);
	impl<R: Get<Rank>, O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O>
		for EnsureAmbassadorsFrom<R>
	{
		type Success = Rank;
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match Origin::as_voice(&o) {
				Some(r) if r >= R::get() => Ok(r),
				_ => Err(O::from(o)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			ranks::GLOBAL_HEAD_AMBASSADOR
				.ge(&R::get())
				.then(|| O::from(Origin::GlobalHeadAmbassador))
				.ok_or(())
		}
	}
}
