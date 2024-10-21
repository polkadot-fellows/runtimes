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

//! PoToC custom origins.

use super::ranks;
pub use pallet_origins::*;

#[frame_support::pallet]
pub mod pallet_origins {
	use super::ranks;
	use frame_support::pallet_prelude::*;
	use pallet_ranked_collective::Rank;

	#[pallet::config]
	pub trait Config: frame_system::Config {}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[derive(PartialEq, Eq, Clone, MaxEncodedLen, Encode, Decode, TypeInfo, RuntimeDebug)]
	#[pallet::origin]
	pub enum Origin {
		/// Plurality voice of the [ranks::MEMBER] members given via referendum.
		Members,
	}

	impl Origin {
		/// Returns the rank that the origin `self` speaks for, or `None` if it doesn't speak for
		/// any.
		pub fn as_voice(&self) -> Option<Rank> {
			Some(match &self {
				Origin::Members => ranks::MEMBER,
			})
		}
	}

	/// Ensures [`Origin::Members`] origin.
	pub struct Members;
	impl<O: Into<Result<Origin, O>> + From<Origin>> EnsureOrigin<O> for Members {
		type Success = ();
		fn try_origin(o: O) -> Result<Self::Success, O> {
			o.into().map(|o| match o {
				Origin::Members => (),
			})
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn try_successful_origin() -> Result<O, ()> {
			Ok(O::from(Origin::Members))
		}
	}
}
