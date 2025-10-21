// Copyright (C) Parity Technologies (UK) Ltd.
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

use alloc::vec::Vec;
use pallet_ah_migrator::LOG_TARGET;
use xcm::latest::{Instruction, InteriorLocation, Junction, Location, Xcm, Reanchorable};

pub fn reanchor_xcm(
	xcm: Xcm<()>,
	ah_location: &Location,
	universal_location: &InteriorLocation,
) -> Result<Xcm<()>, ()> {
	let reanchored_instructions: Result<Vec<_>, ()> = xcm.0.into_iter().map(|instruction| {
		reanchor_instruction(instruction, ah_location, universal_location)
	}).collect();
	
	Ok(Xcm(reanchored_instructions?))
}

pub fn reanchor_xcm_for_send(
	xcm: Xcm<()>,
	_ah_location: &Location,
	_universal_location: &InteriorLocation,
) -> Result<Xcm<()>, ()> {
	// For send operations, the message is already from the destination's perspective
	// We only need to convert DescendOrigin to AliasOrigin
	let converted_instructions: Result<Vec<_>, ()> = xcm.0.into_iter().map(|instruction| {
		convert_descend_to_alias_origin(instruction)
	}).collect();
	
	Ok(Xcm(converted_instructions?))
}

pub fn reanchor_instruction(
	instruction: Instruction<()>,
	ah_location: &Location,
	universal_location: &InteriorLocation,
) -> Result<Instruction<()>, ()> {
	use Instruction::*;

	match instruction {
		// Only map the essential instructions that are actually needed
		WithdrawAsset(assets) => {
			let reanchored_assets = assets.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor assets: {err:?}");
			})?;
			Ok(WithdrawAsset(reanchored_assets))
		},
		PayFees { asset } => {
			let reanchored_asset = asset.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor asset: {err:?}");
			})?;
			Ok(PayFees { asset: reanchored_asset })
		},
		DepositAsset { assets, beneficiary } => {
			let reanchored_beneficiary = if is_local_account(&beneficiary) {
				// Local accounts (parents: 0, AccountId32/AccountKey20) don't need reanchoring
				beneficiary
			} else {
				beneficiary.reanchored(ah_location, universal_location).map_err(|err| {
					log::error!(target: LOG_TARGET, "Failed to reanchor beneficiary: {err:?}");
				})?
			};
			Ok(DepositAsset { assets, beneficiary: reanchored_beneficiary })
		},
		DepositReserveAsset { assets, dest, xcm } => {
			let reanchored_dest = dest.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor dest: {err:?}");
			})?;
			// The nested xcm is already from the perspective of the dest, so no reanchoring needed
			Ok(DepositReserveAsset { assets, dest: reanchored_dest, xcm })
		},
		
		// All other instructions pass through unchanged
		instruction => Ok(instruction),
	}
}



fn is_local_account(location: &Location) -> bool {
	// Check if this is a local account (parents: 0, AccountId32/AccountKey20)
	if location.parents != 0 {
		return false;
	}
	
	match location.interior.first() {
		Some(Junction::AccountId32 { .. }) | Some(Junction::AccountKey20 { .. }) => true,
		_ => false,
	}
}

fn convert_descend_to_alias_origin(instruction: Instruction<()>) -> Result<Instruction<()>, ()> {
	use Instruction::*;
	
	match instruction {
		DescendOrigin(interior) => {
			// Convert DescendOrigin to AliasOrigin for Asset Hub
			// DescendOrigin operates from one parent up, so we need to add one parent
			// to the location when converting to AliasOrigin
			let alias_location = Location::new(1, interior);
			Ok(AliasOrigin(alias_location))
		},
		// All other instructions pass through unchanged
		instruction => Ok(instruction),
	}
}
