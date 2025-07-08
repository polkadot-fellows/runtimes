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
use sp_std::{sync::Arc, vec::Vec};
use xcm::VersionedLocation;

/// Error type for translation operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TranslationError;

/// Helper macro to translate a single junction with error handling
macro_rules! translate_junction {
	($junction:expr) => {
		Self::translate_junction($junction).map_err(|_| TranslationError)?
	};
}

/// Macro to handle V3 junction translation patterns
macro_rules! translate_v3_junctions {
	($interior:expr) => {
		match $interior {
			xcm::v3::Junctions::Here => xcm::v3::Junctions::Here,
			xcm::v3::Junctions::X1(j1) => xcm::v3::Junctions::X1(translate_junction!(j1)),
			xcm::v3::Junctions::X2(j1, j2) =>
				xcm::v3::Junctions::X2(translate_junction!(j1), translate_junction!(j2)),
			xcm::v3::Junctions::X3(j1, j2, j3) => xcm::v3::Junctions::X3(
				translate_junction!(j1),
				translate_junction!(j2),
				translate_junction!(j3),
			),
			xcm::v3::Junctions::X4(j1, j2, j3, j4) => xcm::v3::Junctions::X4(
				translate_junction!(j1),
				translate_junction!(j2),
				translate_junction!(j3),
				translate_junction!(j4),
			),
			xcm::v3::Junctions::X5(j1, j2, j3, j4, j5) => xcm::v3::Junctions::X5(
				translate_junction!(j1),
				translate_junction!(j2),
				translate_junction!(j3),
				translate_junction!(j4),
				translate_junction!(j5),
			),
			xcm::v3::Junctions::X6(j1, j2, j3, j4, j5, j6) => xcm::v3::Junctions::X6(
				translate_junction!(j1),
				translate_junction!(j2),
				translate_junction!(j3),
				translate_junction!(j4),
				translate_junction!(j5),
				translate_junction!(j6),
			),
			xcm::v3::Junctions::X7(j1, j2, j3, j4, j5, j6, j7) => xcm::v3::Junctions::X7(
				translate_junction!(j1),
				translate_junction!(j2),
				translate_junction!(j3),
				translate_junction!(j4),
				translate_junction!(j5),
				translate_junction!(j6),
				translate_junction!(j7),
			),
			xcm::v3::Junctions::X8(j1, j2, j3, j4, j5, j6, j7, j8) => xcm::v3::Junctions::X8(
				translate_junction!(j1),
				translate_junction!(j2),
				translate_junction!(j3),
				translate_junction!(j4),
				translate_junction!(j5),
				translate_junction!(j6),
				translate_junction!(j7),
				translate_junction!(j8),
			),
		}
	};
}

/// Macro to translate Arc-based junctions for V4/V5
macro_rules! translate_arc_junctions {
	($junctions:expr, $junction_type:ty, $count:expr) => {{
		let translated: Result<Vec<$junction_type>, &'static str> =
			$junctions.iter().map(|j| Self::translate_junction(j.clone())).collect();
		let translated = translated.map_err(|_| TranslationError)?;

		// Convert Vec to fixed-size array
		let array: [$junction_type; $count] =
			translated.try_into().map_err(|_| TranslationError)?;
		Ok(Arc::from(array))
	}};
}

/// Macro to handle versioned location translation with error handling
macro_rules! translate_versioned_location {
	($version:ident, $location:expr, $translate_fn:ident, $version_name:literal) => {
		match Self::$translate_fn($location.clone()) {
			Ok(translated) => Ok(VersionedLocation::$version(translated)),
			Err(_) => {
				log::error!(
					target: LOG_TARGET,
					"Failed to translate {} location for treasury spend beneficiary",
					$version_name
				);
				Err(Error::<T>::FailedToConvertType)
			},
		}
	};
}

/// Trait to enable generic AccountId32 junction translation
trait AccountId32Junction {
	fn get_account_id32(&self) -> Option<([u8; 32], Option<xcm::v3::NetworkId>)>;
	fn from_account_id32(id: [u8; 32], network: Option<xcm::v3::NetworkId>) -> Self;
}

/// Macro to generate AccountId32Junction trait implementations
macro_rules! impl_account_id32_junction {
	($junction_type:ty) => {
		impl AccountId32Junction for $junction_type {
			fn get_account_id32(&self) -> Option<([u8; 32], Option<xcm::v3::NetworkId>)> {
				match self {
					Self::AccountId32 { network: _, id } => Some((*id, None)),
					_ => None,
				}
			}

			fn from_account_id32(id: [u8; 32], _network: Option<xcm::v3::NetworkId>) -> Self {
				Self::AccountId32 { network: None, id }
			}
		}
	};
}

// Implement for each XCM version
impl AccountId32Junction for xcm::v3::Junction {
	fn get_account_id32(&self) -> Option<([u8; 32], Option<xcm::v3::NetworkId>)> {
		match self {
			xcm::v3::Junction::AccountId32 { network, id } => Some((*id, *network)),
			_ => None,
		}
	}

	fn from_account_id32(id: [u8; 32], network: Option<xcm::v3::NetworkId>) -> Self {
		xcm::v3::Junction::AccountId32 { network, id }
	}
}

// Implement for V4 and V5 using the macro
impl_account_id32_junction!(xcm::v4::Junction);
impl_account_id32_junction!(xcm::v5::Junction);

impl<T: Config> Pallet<T> {
	/// Translate AccountId32 junctions in a VersionedLocation from RC format to AH format.
	///
	/// This function handles all supported XCM versions (V3, V4, V5) and applies account
	/// translation to any AccountId32 junctions found within the location structure.
	/// All other junction types are preserved unchanged.
	///
	/// Returns an error if the translation fails, following the same strict error handling
	/// pattern used in referenda migration for correctness.
	pub fn translate_beneficiary_location(
		location: VersionedLocation,
	) -> Result<VersionedLocation, Error<T>> {
		match location {
			VersionedLocation::V3(v3_location) => {
				translate_versioned_location!(V3, v3_location, translate_v3_location, "V3")
			},
			VersionedLocation::V4(v4_location) => {
				translate_versioned_location!(V4, v4_location, translate_v4_location_impl, "V4")
			},
			VersionedLocation::V5(v5_location) => {
				translate_versioned_location!(V5, v5_location, translate_v5_location_impl, "V5")
			},
		}
	}

	/// Translate AccountId32 junctions in XCM v3 MultiLocation.
	fn translate_v3_location(
		location: xcm::v3::MultiLocation,
	) -> Result<xcm::v3::MultiLocation, TranslationError> {
		let translated_junctions = translate_v3_junctions!(location.interior);

		Ok(xcm::v3::MultiLocation { parents: location.parents, interior: translated_junctions })
	}

	/// Implementation for V4 location translation
	fn translate_v4_location_impl(
		location: xcm::v4::Location,
	) -> Result<xcm::v4::Location, TranslationError> {
		use xcm::v4::Junctions;

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(junctions) =>
				Junctions::X1(translate_arc_junctions!(junctions, xcm::v4::Junction, 1)?),
			Junctions::X2(junctions) =>
				Junctions::X2(translate_arc_junctions!(junctions, xcm::v4::Junction, 2)?),
			Junctions::X3(junctions) =>
				Junctions::X3(translate_arc_junctions!(junctions, xcm::v4::Junction, 3)?),
			Junctions::X4(junctions) =>
				Junctions::X4(translate_arc_junctions!(junctions, xcm::v4::Junction, 4)?),
			Junctions::X5(junctions) =>
				Junctions::X5(translate_arc_junctions!(junctions, xcm::v4::Junction, 5)?),
			Junctions::X6(junctions) =>
				Junctions::X6(translate_arc_junctions!(junctions, xcm::v4::Junction, 6)?),
			Junctions::X7(junctions) =>
				Junctions::X7(translate_arc_junctions!(junctions, xcm::v4::Junction, 7)?),
			Junctions::X8(junctions) =>
				Junctions::X8(translate_arc_junctions!(junctions, xcm::v4::Junction, 8)?),
		};

		Ok(xcm::v4::Location { parents: location.parents, interior: translated_junctions })
	}

	/// Implementation for V5 location translation
	fn translate_v5_location_impl(
		location: xcm::v5::Location,
	) -> Result<xcm::v5::Location, TranslationError> {
		use xcm::v5::Junctions;

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(junctions) =>
				Junctions::X1(translate_arc_junctions!(junctions, xcm::v5::Junction, 1)?),
			Junctions::X2(junctions) =>
				Junctions::X2(translate_arc_junctions!(junctions, xcm::v5::Junction, 2)?),
			Junctions::X3(junctions) =>
				Junctions::X3(translate_arc_junctions!(junctions, xcm::v5::Junction, 3)?),
			Junctions::X4(junctions) =>
				Junctions::X4(translate_arc_junctions!(junctions, xcm::v5::Junction, 4)?),
			Junctions::X5(junctions) =>
				Junctions::X5(translate_arc_junctions!(junctions, xcm::v5::Junction, 5)?),
			Junctions::X6(junctions) =>
				Junctions::X6(translate_arc_junctions!(junctions, xcm::v5::Junction, 6)?),
			Junctions::X7(junctions) =>
				Junctions::X7(translate_arc_junctions!(junctions, xcm::v5::Junction, 7)?),
			Junctions::X8(junctions) =>
				Junctions::X8(translate_arc_junctions!(junctions, xcm::v5::Junction, 8)?),
		};

		Ok(xcm::v5::Location { parents: location.parents, interior: translated_junctions })
	}

	/// Generic junction translation for all XCM versions using trait bounds
	fn translate_junction<J>(junction: J) -> Result<J, &'static str>
	where
		J: AccountId32Junction + Clone,
	{
		match junction.get_account_id32() {
			Some((id, network)) => {
				let account_id =
					T::AccountId::decode(&mut &id[..]).expect("Account decoding should never fail");
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] = translated_account
					.encode()
					.try_into()
					.expect("Account encoding should never fail");

				Ok(J::from_account_id32(translated_id, network))
			},
			None => Ok(junction), // Non-AccountId32 junctions pass through unchanged
		}
	}
}
