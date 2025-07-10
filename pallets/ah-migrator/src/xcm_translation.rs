// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use codec::{Decode, Encode};
use xcm::{latest::prelude::*, VersionedLocation};

/// Helper trait for AccountId32 junction operations on latest XCM version
trait AccountId32JunctionOps {
	fn get_account_id32(&self) -> Option<[u8; 32]>;
	fn with_account_id32(self, id: [u8; 32]) -> Self;
}

impl AccountId32JunctionOps for Junction {
	fn get_account_id32(&self) -> Option<[u8; 32]> {
		match self {
			Junction::AccountId32 { id, .. } => Some(*id),
			_ => None,
		}
	}

	fn with_account_id32(self, new_id: [u8; 32]) -> Self {
		match self {
			Junction::AccountId32 { network, .. } => Junction::AccountId32 { network, id: new_id },
			other => other,
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Translate beneficiary location for treasury spends from RC format to AH format.
	///
	/// This function ONLY translates locations that match the pattern X1(Junction::AccountId32).
	/// All other location patterns are returned unchanged, as they are assumed to be
	/// destinations like `Location::new(1, Parachain(2030))` which don't require translation.
	///
	/// The rationale is that the only AccountId32 locations that need translation are those
	/// referring to direct account addresses on the relay chain, which need to be translated
	/// to their corresponding addresses on Asset Hub.
	///
	/// Returns the location unchanged if it doesn't match X1(AccountId32), or returns
	/// the translated location if it does match.
	pub fn translate_beneficiary_location(
		location: VersionedLocation,
	) -> Result<VersionedLocation, Error<T>> {
		// Convert to latest version for unified processing
		let latest_location: Location = location.clone().try_into().map_err(|_| {
			log::error!(
				target: LOG_TARGET,
				"Failed to convert VersionedLocation to latest version"
			);
			Error::<T>::FailedToConvertType
		})?;

		// Check if this is exactly X1(AccountId32) pattern
		if latest_location.parents == 0 && latest_location.interior.len() == 1 {
			if let Some(junction) = latest_location.interior.first() {
				if let Some(id) = junction.get_account_id32() {
					// This is X1(AccountId32), translate it
					let account_id = T::AccountId::decode(&mut &id[..])
						.map_err(|_| Error::<T>::FailedToConvertType)?;
					let translated_account = Self::translate_account_rc_to_ah(account_id);
					let translated_id: [u8; 32] = translated_account
						.encode()
						.try_into()
						.map_err(|_| Error::<T>::FailedToConvertType)?;
					let translated_junction = junction.clone().with_account_id32(translated_id);
					let translated_location = Location::new(0, translated_junction);

					// Convert back to original version
					let original_version = location.identify_version();
					return VersionedLocation::from(translated_location)
						.into_version(original_version)
						.map_err(|_| {
							log::error!(
								target: LOG_TARGET,
								"Failed to convert back to original XCM version {}",
								original_version
							);
							Error::<T>::FailedToConvertType
						});
				}
			}
		}

		// Not X1(AccountId32), return unchanged
		Ok(location)
	}
}
