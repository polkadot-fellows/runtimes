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
use xcm::latest::{AssetFilter, Instruction, InteriorLocation, Junction, Location, Xcm, Reanchorable};

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

pub fn reanchor_instruction(
	instruction: Instruction<()>,
	ah_location: &Location,
	universal_location: &InteriorLocation,
) -> Result<Instruction<()>, ()> {
	use Instruction::*;

	match instruction {
		// Instructions that contain assets that need reanchoring
		WithdrawAsset(assets) => {
			let reanchored_assets = assets.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor assets: {err:?}");
			})?;
			Ok(WithdrawAsset(reanchored_assets))
		},
		ReserveAssetDeposited(assets) => {
			let reanchored_assets = assets.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor assets: {err:?}");
			})?;
			Ok(ReserveAssetDeposited(reanchored_assets))
		},
		ReceiveTeleportedAsset(assets) => {
			let reanchored_assets = assets.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor assets: {err:?}");
			})?;
			Ok(ReceiveTeleportedAsset(reanchored_assets))
		},
		PayFees { asset } => {
			let reanchored_asset = asset.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor asset: {err:?}");
			})?;
			Ok(PayFees { asset: reanchored_asset })
		},
		BuyExecution { fees, weight_limit } => {
			let reanchored_fees = fees.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor fees: {err:?}");
			})?;
			Ok(BuyExecution { fees: reanchored_fees, weight_limit })
		},
		
		// Instructions that contain locations that need reanchoring
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
		
		// Instructions that contain both locations and nested XCM that need reanchoring
		InitiateReserveWithdraw { assets, reserve, xcm } => {
			let reanchored_reserve = reserve.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor reserve: {err:?}");
			})?;
			let reanchored_xcm = reanchor_xcm(xcm, ah_location, universal_location)?;
			Ok(InitiateReserveWithdraw { assets, reserve: reanchored_reserve, xcm: reanchored_xcm })
		},
		InitiateTeleport { assets, dest, xcm } => {
			let reanchored_dest = dest.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor dest: {err:?}");
			})?;
			let reanchored_xcm = reanchor_xcm(xcm, ah_location, universal_location)?;
			Ok(InitiateTeleport { assets, dest: reanchored_dest, xcm: reanchored_xcm })
		},
		ExchangeAsset { give, want, maximal } => {
			let reanchored_give = reanchor_asset_filter(give, ah_location, universal_location)?;
			let reanchored_want = want.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor want assets: {err:?}");
			})?;
			Ok(ExchangeAsset { give: reanchored_give, want: reanchored_want, maximal })
		},
		
		// Instructions that don't contain locations or assets - pass through unchanged
		instruction => Ok(instruction),
	}
}


fn reanchor_asset_filter(
	filter: AssetFilter,
	ah_location: &Location,
	universal_location: &InteriorLocation,
) -> Result<AssetFilter, ()> {
	match filter {
		AssetFilter::Definite(assets) => {
			let reanchored_assets = assets.reanchored(ah_location, universal_location).map_err(|err| {
				log::error!(target: LOG_TARGET, "Failed to reanchor asset filter assets: {err:?}");
			})?;
			Ok(AssetFilter::Definite(reanchored_assets))
		},
		AssetFilter::Wild(wild) => {
			// Wild filters don't contain specific locations, so pass through unchanged
			Ok(AssetFilter::Wild(wild))
		},
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
