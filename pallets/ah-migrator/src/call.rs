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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use crate::*;
use frame_support::traits::Bounded;

pub type BoundedCallOf<T> =
	Bounded<<T as frame_system::Config>::RuntimeCall, <T as frame_system::Config>::Hashing>;

impl<T: Config> Pallet<T> {
	// Maps a RC BoundedCall to an AH BoundedCall.
	// The preimage from the RC BoundedCall is unrequested on the relay chain, then re-created on
	// Asset Hub with translated parameters. Notice that the unrequest operation may delete the
	// preimage hash from storage.
	pub fn map_rc_ah_call(
		rc_bounded_call: &BoundedCallOf<T>,
	) -> Result<BoundedCallOf<T>, Error<T>> {
		let encoded_call = if let Ok(e) = Self::fetch_preimage(rc_bounded_call) {
			e
		} else {
			return Err(Error::<T>::PreimageNotFound);
		};

		let mut unrequested_preimage = false;

		if let Some(hash) = rc_bounded_call.lookup_hash() {
			if T::Preimage::is_requested(&hash) {
				T::Preimage::unrequest(&hash);
			} else {
				unrequested_preimage = true;
			}
		}

		let call = if let Ok(call) = T::RcToAhCall::try_convert(&encoded_call) {
			call
		} else {
			log::error!(target: LOG_TARGET, "Failed to convert call: {:?}", rc_bounded_call);
			return Err(Error::<T>::FailedToConvertCall);
		};

		log::debug!(target: LOG_TARGET, "mapped call: {:?}", call);

		let ah_bounded_call = T::Preimage::bound(call).map_err(|err| {
			defensive!("Failed to bound call: {:?}", err);
			Error::<T>::FailedToBoundCall
		})?;

		if let Some(hash) = ah_bounded_call.lookup_hash() {
			if T::Preimage::is_requested(&hash) && unrequested_preimage {
				log::warn!(target: LOG_TARGET, "RC unrequested preimage has become requested on AH with hash {:?}", hash);
			}
		}

		if ah_bounded_call.lookup_needed() {
			// Noted preimages for referendums that did not pass will need to be manually removed
			// later.
			log::debug!(target: LOG_TARGET, "New preimage was noted for call");
		}

		Ok(ah_bounded_call)
	}

	fn fetch_preimage(bounded_call: &BoundedCallOf<T>) -> Result<Vec<u8>, Error<T>> {
		match bounded_call {
			Bounded::Inline(encoded) => Ok(encoded.clone().into_inner()),
			Bounded::Legacy { hash, .. } => {
				let encoded = if let Ok(encoded) = T::Preimage::fetch(hash, None) {
					encoded
				} else {
					// not an error since a submitter can delete the preimage for ongoing referendum
					log::warn!(target: LOG_TARGET, "No preimage found for call hash: {:?}", hash);
					return Err(Error::<T>::PreimageNotFound);
				};
				Ok(encoded.into_owned())
			},
			Bounded::Lookup { hash, len } => {
				let encoded = if let Ok(encoded) = T::Preimage::fetch(hash, Some(*len)) {
					encoded
				} else {
					// not an error since a submitter can delete the preimage for ongoing referendum
					log::warn!(target: LOG_TARGET, "No preimage found for call hash: {:?}", (hash, len));
					return Err(Error::<T>::PreimageNotFound);
				};
				Ok(encoded.into_owned())
			},
		}
	}

	// Helper to convert the call without using the preimage pallet. Used in migration checks.
	#[cfg(feature = "std")]
	pub fn map_rc_ah_call_no_preimage(
		encoded_call: Vec<u8>,
	) -> Result<call::BoundedCallOf<T>, Error<T>> {
		use frame_support::traits::BoundedInline;
		use sp_runtime::traits::Hash;

		// Convert call.
		let call = if let Ok(call) = T::RcToAhCall::try_convert(&encoded_call) {
			call
		} else {
			return Err(Error::<T>::FailedToConvertCall);
		};

		// Bound it.
		let data = call.encode();
		let len = data.len() as u32;
		Ok(match BoundedInline::try_from(data) {
			Ok(bounded) => Bounded::Inline(bounded),
			Err(unbounded) => Bounded::Lookup {
				hash: <<T as frame_system::Config>::Hashing as Hash>::hash(&unbounded[..]),
				len,
			},
		})
	}
}
