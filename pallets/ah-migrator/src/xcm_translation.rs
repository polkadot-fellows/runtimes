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
	/// Translate AccountId32 junctions in a VersionedLocation from RC format to AH format.
	///
	/// This function leverages the XCM SDK's built-in version conversion infrastructure
	/// to handle all supported XCM versions (V3, V4, V5) by converting to the latest
	/// version for processing, then converting back to the original version.
	///
	/// TODO: Three options to evaluate.
	/// 1. Should we translate ALL AccountId32 junctions within any VersionedLocation used as
	/// treasury spend beneficiaries as we are doing now?
	/// 2. Should we not translate any location at all if we assume that the only use case is
	/// `beneficiary: Location::new(1, Parachain(2030))` ?
	/// 3. Should we go for some pattern whitelisting  e.g. if we assume that the other relevant use
	///    case is the following:
	/// ```text
	/// TreasurySpend {
	///      beneficiary: Location::new(0, AccountId32 {
	///          network: None,
	///          id: [/* Direct bytes of para2030 sovereign account */]
	///          // 13YMK2eeopZtUNpeHnJ1Ws2HqMQG6Ts9PGCZYGyFbSYoZfcm
	///      }),
	///      amount: 1000_DOT,
	///  }
	/// ```
	///  Are there more cases to whitelist in case?
	///
	/// Returns an error if version conversion or translation
	/// fails.
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

		// Apply account translation to latest version
		let translated_latest = Self::translate_location_latest(latest_location)?;

		// Convert back to original version
		let original_version = location.identify_version();
		VersionedLocation::from(translated_latest)
			.into_version(original_version)
			.map_err(|_| {
				log::error!(
					target: LOG_TARGET,
					"Failed to convert back to original XCM version {}",
					original_version
				);
				Error::<T>::FailedToConvertType
			})
	}

	/// Translate AccountId32 junctions in the latest XCM Location format.
	///
	/// This function handles the actual account translation logic on the latest
	/// XCM version, eliminating the need for version-specific implementations.
	fn translate_location_latest(location: Location) -> Result<Location, Error<T>> {
		let translated_junctions = Self::translate_junctions_latest(location.interior)?;
		Ok(Location { parents: location.parents, interior: translated_junctions })
	}

	/// Translate junctions in the latest XCM format
	fn translate_junctions_latest(junctions: Junctions) -> Result<Junctions, Error<T>> {
		junctions
			.iter()
			.map(|junction| Self::translate_junction_latest(junction.clone()))
			.try_fold(Junctions::Here, |mut acc, translated_junction| {
				let junction = translated_junction?;
				acc.push(junction).map_err(|_| Error::<T>::FailedToConvertType)?;
				Ok(acc)
			})
	}

	/// Translate a single junction in the latest XCM format
	fn translate_junction_latest(junction: Junction) -> Result<Junction, Error<T>> {
		match junction.get_account_id32() {
			Some(id) => {
				let account_id = T::AccountId::decode(&mut &id[..])
					.map_err(|_| Error::<T>::FailedToConvertType)?;
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] = translated_account
					.encode()
					.try_into()
					.map_err(|_| Error::<T>::FailedToConvertType)?;
				Ok(junction.with_account_id32(translated_id))
			},
			None => Ok(junction), // Non-AccountId32 junctions pass through unchanged
		}
	}
}
