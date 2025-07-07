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
use sp_std::{sync::Arc, vec::Vec};
use xcm::VersionedLocation;

impl<T: Config> Pallet<T> {
	/// Translate AccountId32 junctions in a VersionedLocation from RC format to AH format.
	///
	/// This function handles all supported XCM versions (V3, V4, V5) and applies account
	/// translation to any AccountId32 junctions found within the location structure.
	/// All other junction types are preserved unchanged.
	pub fn translate_beneficiary_location(location: VersionedLocation) -> VersionedLocation {
		match location {
			VersionedLocation::V3(v3_location) => {
				match Self::translate_v3_location(v3_location.clone()) {
					Ok(translated) => VersionedLocation::V3(translated),
					Err(_) => {
						log::warn!(
							target: LOG_TARGET,
							"Failed to translate V3 location, returning original"
						);
						VersionedLocation::V3(v3_location)
					},
				}
			},
			VersionedLocation::V4(v4_location) => {
				match Self::translate_v4_location(v4_location.clone()) {
					Ok(translated) => VersionedLocation::V4(translated),
					Err(_) => {
						log::warn!(
							target: LOG_TARGET,
							"Failed to translate V4 location, returning original"
						);
						VersionedLocation::V4(v4_location)
					},
				}
			},
			VersionedLocation::V5(v5_location) => {
				match Self::translate_v5_location(v5_location.clone()) {
					Ok(translated) => VersionedLocation::V5(translated),
					Err(_) => {
						log::warn!(
							target: LOG_TARGET,
							"Failed to translate V5 location, returning original"
						);
						VersionedLocation::V5(v5_location)
					},
				}
			},
		}
	}

	/// Translate AccountId32 junctions in XCM v3 MultiLocation.
	fn translate_v3_location(
		location: xcm::v3::MultiLocation,
	) -> Result<xcm::v3::MultiLocation, ()> {
		use xcm::v3::{Junction, Junctions};

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(j1) => Junctions::X1(Self::translate_v3_junction(j1)?),
			Junctions::X2(j1, j2) =>
				Junctions::X2(Self::translate_v3_junction(j1)?, Self::translate_v3_junction(j2)?),
			Junctions::X3(j1, j2, j3) => Junctions::X3(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
			),
			Junctions::X4(j1, j2, j3, j4) => Junctions::X4(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
			),
			Junctions::X5(j1, j2, j3, j4, j5) => Junctions::X5(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
			),
			Junctions::X6(j1, j2, j3, j4, j5, j6) => Junctions::X6(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
				Self::translate_v3_junction(j6)?,
			),
			Junctions::X7(j1, j2, j3, j4, j5, j6, j7) => Junctions::X7(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
				Self::translate_v3_junction(j6)?,
				Self::translate_v3_junction(j7)?,
			),
			Junctions::X8(j1, j2, j3, j4, j5, j6, j7, j8) => Junctions::X8(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
				Self::translate_v3_junction(j6)?,
				Self::translate_v3_junction(j7)?,
				Self::translate_v3_junction(j8)?,
			),
		};

		Ok(xcm::v3::MultiLocation { parents: location.parents, interior: translated_junctions })
	}

	/// Translate AccountId32 junctions in XCM v4 Location.
	fn translate_v4_location(location: xcm::v4::Location) -> Result<xcm::v4::Location, ()> {
		use xcm::v4::{Junction, Junctions};

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X1(Arc::from([translated[0].clone()]))
			},
			Junctions::X2(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X2(Arc::from([translated[0].clone(), translated[1].clone()]))
			},
			Junctions::X3(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X3(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
				]))
			},
			Junctions::X4(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X4(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
				]))
			},
			Junctions::X5(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X5(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
				]))
			},
			Junctions::X6(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X6(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
				]))
			},
			Junctions::X7(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X7(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
				]))
			},
			Junctions::X8(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X8(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
					translated[7].clone(),
				]))
			},
		};

		Ok(xcm::v4::Location { parents: location.parents, interior: translated_junctions })
	}

	/// Translate AccountId32 junctions in XCM v5 Location.
	fn translate_v5_location(location: xcm::v5::Location) -> Result<xcm::v5::Location, ()> {
		use xcm::v5::{Junction, Junctions};

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X1(Arc::from([translated[0].clone()]))
			},
			Junctions::X2(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X2(Arc::from([translated[0].clone(), translated[1].clone()]))
			},
			Junctions::X3(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X3(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
				]))
			},
			Junctions::X4(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X4(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
				]))
			},
			Junctions::X5(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X5(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
				]))
			},
			Junctions::X6(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X6(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
				]))
			},
			Junctions::X7(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X7(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
				]))
			},
			Junctions::X8(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X8(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
					translated[7].clone(),
				]))
			},
		};

		Ok(xcm::v5::Location { parents: location.parents, interior: translated_junctions })
	}

	/// Translate AccountId32 junction in XCM v3.
	fn translate_v3_junction(junction: xcm::v3::Junction) -> Result<xcm::v3::Junction, ()> {
		use xcm::v3::Junction;

		match junction {
			Junction::AccountId32 { network, id } => {
				// Convert [u8; 32] to AccountId, translate, then back to [u8; 32]
				let account_id = T::AccountId::decode(&mut &id[..]).map_err(|_| ())?;
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] =
					translated_account.encode().try_into().map_err(|_| ())?;

				Ok(Junction::AccountId32 { network, id: translated_id })
			},
			// All other junction types pass through unchanged
			other => Ok(other),
		}
	}

	/// Translate AccountId32 junction in XCM v4.
	fn translate_v4_junction(junction: xcm::v4::Junction) -> Result<xcm::v4::Junction, ()> {
		use xcm::v4::Junction;

		match junction {
			Junction::AccountId32 { network, id } => {
				// Convert [u8; 32] to AccountId, translate, then back to [u8; 32]
				let account_id = T::AccountId::decode(&mut &id[..]).map_err(|_| ())?;
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] =
					translated_account.encode().try_into().map_err(|_| ())?;

				Ok(Junction::AccountId32 { network, id: translated_id })
			},
			// All other junction types pass through unchanged
			other => Ok(other),
		}
	}

	/// Translate AccountId32 junction in XCM v5.
	fn translate_v5_junction(junction: xcm::v5::Junction) -> Result<xcm::v5::Junction, ()> {
		use xcm::v5::Junction;

		match junction {
			Junction::AccountId32 { network, id } => {
				// Convert [u8; 32] to AccountId, translate, then back to [u8; 32]
				let account_id = T::AccountId::decode(&mut &id[..]).map_err(|_| ())?;
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] =
					translated_account.encode().try_into().map_err(|_| ())?;

				Ok(Junction::AccountId32 { network, id: translated_id })
			},
			// All other junction types pass through unchanged
			other => Ok(other),
		}
	}
}
