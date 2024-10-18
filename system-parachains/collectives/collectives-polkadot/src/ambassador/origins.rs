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

#[frame_support::pallet]
pub mod pallet_origins {
	use crate::ambassador::ranks;
	use frame_support::pallet_prelude::*;
	use pallet_ranked_collective::Rank;

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
		Ambassadors,
		/// Plurality voice of the [ranks::SENIOR_AMBASSADOR] members or above given via
		/// referendum.
		SeniorAmbassadors,
		/// Plurality voice of the [ranks::HEAD_AMBASSADOR] members given via referendum.
		HeadAmbassadors,
	}

	impl Origin {
		/// Returns the rank that the origin `self` speaks for, or `None` if it doesn't speak for
		/// any.
		pub fn as_voice(&self) -> Option<Rank> {
			Some(match &self {
				Origin::Ambassadors => ranks::AMBASSADOR,
				Origin::SeniorAmbassadors => ranks::SENIOR_AMBASSADOR,
				Origin::HeadAmbassadors => ranks::HEAD_AMBASSADOR,
			})
		}
	}

	/// Ensures [`Origin::HeadAmbassadors`] origin.
	pub struct HeadAmbassadors;
	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for HeadAmbassadors {
		type Success = ();
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::HeadAmbassadors => Ok(()),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::HeadAmbassadors))
		}
	}

	/// Ensures [`Origin::SeniorAmbassadors`] origin.
	pub struct SeniorAmbassadors;
	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for SeniorAmbassadors {
		type Success = ();
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::SeniorAmbassadors => Ok(()),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::SeniorAmbassadors))
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
			ranks::HEAD_AMBASSADOR
				.ge(&R::get())
				.then(|| O::from(Origin::HeadAmbassadors))
				.ok_or(())
		}
	}
}
