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
	use pallet_ranked_collective::Rank;
	use polkadot_runtime_constants::currency::DOLLARS;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	/// The pallet configuration trait.
	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[derive(
		PartialEq,
		Eq,
		Clone,
		MaxEncodedLen,
		Encode,
		Decode,
		DecodeWithMemTracking,
		TypeInfo,
		RuntimeDebug,
	)]
	#[pallet::origin]
	pub enum Origin {
		Associate,
		Lead,
		Senior,
		Principal,
		Global,
		GlobalHead,
		RetainAtAssociate,
		RetainAtLead,
		RetainAtSenior,
		RetainAtPrincipal,
		PromoteToAssociate,
		PromoteToLead,
		PromoteToSenior,
		PromoteToPrincipal,
		FastPromoteToAssociate,
		FastPromoteToLead,
		FastPromoteToSenior,
		Tip,
		Treasurer,
	}

	impl Origin {
		/// Returns the rank that the origin `self` speaks for, or `None` if it doesn't speak for
		/// any.
		pub fn as_voice(&self) -> Option<pallet_ranked_collective::Rank> {
			Some(match &self {
				Origin::Associate => ranks::ASSOCIATE,
				Origin::Lead => ranks::LEAD,
				Origin::Senior => ranks::SENIOR,
				Origin::Principal => ranks::PRINCIPAL,
				Origin::Global => ranks::GLOBAL,
				Origin::GlobalHead => ranks::GLOBAL_HEAD,
				_ => return None,
			})
		}
	}

	pub struct EnsureCanRetainAt;

	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for EnsureCanRetainAt {
		type Success = Rank;

		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::RetainAtAssociate => Ok(ranks::ASSOCIATE),
				Origin::RetainAtLead => Ok(ranks::LEAD),
				Origin::RetainAtSenior => Ok(ranks::SENIOR),
				Origin::RetainAtPrincipal => Ok(ranks::PRINCIPAL),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::RetainAtPrincipal))
		}
	}

	pub struct EnsureCanPromoteTo;

	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for EnsureCanPromoteTo {
		type Success = Rank;

		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::PromoteToAssociate => Ok(ranks::ASSOCIATE),
				Origin::PromoteToLead => Ok(ranks::LEAD),
				Origin::PromoteToSenior => Ok(ranks::SENIOR),
				Origin::PromoteToPrincipal => Ok(ranks::PRINCIPAL),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::PromoteToPrincipal))
		}
	}

	pub struct EnsureCanFastPromoteTo;

	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for EnsureCanFastPromoteTo {
		type Success = Rank;

		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::FastPromoteToAssociate => Ok(ranks::ASSOCIATE),
				Origin::FastPromoteToLead => Ok(ranks::LEAD),
				Origin::FastPromoteToSenior => Ok(ranks::SENIOR),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::FastPromoteToSenior))
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

	/// Ensures [`Origin::GlobalHead`] origin.
	pub struct GlobalHead;
	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for GlobalHead {
		type Success = ();
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::GlobalHead => Ok(()),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::GlobalHead))
		}
	}

	/// Ensures [`Origin::Senior`] origin.
	pub struct Senior;
	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for Senior {
		type Success = ();
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().and_then(|o| match o {
				Origin::Senior => Ok(()),
				r => Err(O::from(r)),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::Senior))
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
			ranks::GLOBAL_HEAD.ge(&R::get()).then(|| O::from(Origin::GlobalHead)).ok_or(())
		}
	}
}
