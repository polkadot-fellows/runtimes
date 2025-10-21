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

//! Module for mapping XCM programs from Relay Chain to Asset Hub during the AHM.

use alloc::vec::Vec;
use pallet_ah_migrator::LOG_TARGET;
use xcm::prelude::*;

/// Reanchors an XCM used in `execute` to the Asset Hub.
///
/// It iterates over all instructions, changing locations and assets when needed.
///
/// Only some instructions are mapped, the rest will throw an error.
pub fn reanchor_xcm(
	xcm: Xcm<()>,
	ah_location: &Location,
	universal_location: &InteriorLocation,
) -> Result<Xcm<()>, ()> {
	let reanchored_instructions: Result<Vec<_>, ()> = xcm
		.0
		.into_iter()
		.map(|instruction| reanchor_instruction(instruction, ah_location, universal_location))
		.collect();

	Ok(Xcm(reanchored_instructions?))
}

/// Reanchors an XCM used in `send` to the Asset Hub.
///
/// Since the message is already sent somewhere else, mostly the destination
/// needs to be reanchored.
///
/// If `DescendOrigin` is used, it has to be mapped to `AliasOrigin` since
/// Asset Hub's location is one lower than the Relay.
/// For `AliasOrigin` to work, the recipient chain needs to comply with the
/// suggested aliasing rules in RFC#122:
/// https://github.com/polkadot-fellows/RFCs/blob/main/text/0122-alias-origin-on-asset-transfers.md#suggested-aliasing-rules.
pub fn reanchor_xcm_for_send(
	xcm: Xcm<()>,
	_ah_location: &Location,
	_universal_location: &InteriorLocation,
) -> Result<Xcm<()>, ()> {
	// For send operations, the message is already from the destination's perspective.
	let converted_instructions: Result<Vec<_>, ()> = xcm
		.0
		.into_iter()
		.map(|instruction| convert_descend_to_alias_origin(instruction))
		.collect();

	Ok(Xcm(converted_instructions?))
}

fn reanchor_instruction(
	instruction: Instruction<()>,
	ah_location: &Location,
	universal_location: &InteriorLocation,
) -> Result<Instruction<()>, ()> {
	use Instruction::*;

	// We only map a particular subset of instructions, to keep the logic minimal.
	match instruction {
		WithdrawAsset(assets) => {
			let reanchored_assets =
				assets.reanchored(ah_location, universal_location).map_err(|err| {
					log::error!(target: LOG_TARGET, "Failed to reanchor assets: {err:?}");
				})?;
			Ok(WithdrawAsset(reanchored_assets))
		},
		PayFees { asset } => {
			let reanchored_asset =
				asset.reanchored(ah_location, universal_location).map_err(|err| {
					log::error!(target: LOG_TARGET, "Failed to reanchor asset: {err:?}");
				})?;
			Ok(PayFees { asset: reanchored_asset })
		},
		DepositAsset { assets, beneficiary } => {
			let reanchored_beneficiary = if is_local_account(&beneficiary) {
				// Local accounts (parents: 0, AccountId32/AccountKey20) don't need reanchoring.
				beneficiary
			} else {
				beneficiary.reanchored(ah_location, universal_location).map_err(|err| {
					log::error!(target: LOG_TARGET, "Failed to reanchor beneficiary: {err:?}");
				})?
			};
			Ok(DepositAsset { assets, beneficiary: reanchored_beneficiary })
		},
		DepositReserveAsset { assets, dest, xcm } => {
			let reanchored_dest =
				dest.reanchored(ah_location, universal_location).map_err(|err| {
					log::error!(target: LOG_TARGET, "Failed to reanchor dest: {err:?}");
				})?;
			// The nested xcm is already from the perspective of the dest, so no reanchoring needed.
			Ok(DepositReserveAsset { assets, dest: reanchored_dest, xcm })
		},
		RefundSurplus => Ok(RefundSurplus),

		// All other instructions are not supported in the migration.
		instruction => {
			log::error!(
				target: LOG_TARGET,
				"Unsupported XCM instruction in migration: {instruction:?}",
			);
			Err(())
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

fn convert_descend_to_alias_origin(instruction: Instruction<()>) -> Result<Instruction<()>, ()> {
	use Instruction::*;

	match instruction {
		DescendOrigin(interior) => {
			// Convert `DescendOrigin` to `AliasOrigin` from Asset Hub.
			// `DescendOrigin` by the Relay Chain operated from one parent up,
			// so we need to add one parent	to the location when converting to `AliasOrigin`.
			let alias_location = Location::new(1, interior);
			Ok(AliasOrigin(alias_location))
		},
		// All other instructions pass through unchanged
		instruction => Ok(instruction),
	}
}

#[cfg(test)]
mod tests {
	use crate::ah_migration::{RcRuntimeCall, RcToAhCall, RcXcmCall, RuntimeCall};
	use xcm::prelude::*;

	#[test]
	fn map_xcm_execute() {
		// XCM on Relay Chain we want to execute.
		let xcm = Xcm::builder()
			.withdraw_asset((Here, 10_000_000_000u128))
			.pay_fees((Here, 100_000_000u128))
			.deposit_reserve_asset(
				AllCounted(1),
				Location::new(0, [Parachain(1004)]),
				// Whatever.
				Xcm::builder_unsafe()
					.clear_origin()
					.withdraw_asset((Parent, 10_000_000_000u128))
					.build(),
			)
			.refund_surplus()
			.deposit_asset(AllCounted(1), [1u8; 32])
			.build();
		let rc_call = RcRuntimeCall::XcmPallet(RcXcmCall::execute {
			message: Box::new(VersionedXcm::from(xcm)),
			max_weight: Weight::MAX,
		});
		let mapped_call = RcToAhCall::map(rc_call).expect("Call can be mapped");
		assert_eq!(
			mapped_call,
			RuntimeCall::PolkadotXcm(pallet_xcm::Call::execute {
				message: Box::new(VersionedXcm::V5(
					Xcm::builder()
						// `Here` becomes `Parent`.
						.withdraw_asset((Parent, 10_000_000_000u128))
						.pay_fees((Parent, 100_000_000u128))
						.deposit_reserve_asset(
							AllCounted(1),
							// Parachain location gets a parent.
							Location::new(1, [Parachain(1004)]),
							// The same remote xcm, since it was already meant
							// for the destination.
							Xcm::builder_unsafe()
								.clear_origin()
								.withdraw_asset((Parent, 10_000_000_000u128))
								.build(),
						)
						.refund_surplus()
						// Local accounts are not reanchored.
						.deposit_asset(AllCounted(1), [1u8; 32])
						.build()
				)),
				max_weight: Weight::MAX,
			})
		);
	}

	#[test]
	fn map_xcm_send() {
		let destination = Location::new(0, [Parachain(1004)]);
		// Relay wants to act as any parachain.
		let xcm = Xcm::builder_unsafe()
			.descend_origin(Parachain(1002))
			.withdraw_asset((Parent, 10_000_000_000u128))
			.buy_execution((Parent, 100_000_000u128), Unlimited)
			.transact(
				OriginKind::SovereignAccount,
				Weight::from_parts(10_000_000_000, 100_000),
				Vec::new(),
			)
			.refund_surplus()
			.deposit_asset(AllCounted(1), [1u8; 32])
			.build();
		let rc_call = RcRuntimeCall::XcmPallet(RcXcmCall::send {
			dest: Box::new(VersionedLocation::from(destination)),
			message: Box::new(VersionedXcm::from(xcm)),
		});
		let mapped_call = RcToAhCall::map(rc_call).expect("Call can be mapped");
		assert_eq!(
			mapped_call,
			RuntimeCall::PolkadotXcm(pallet_xcm::Call::send {
				message: Box::new(VersionedXcm::from(
					Xcm::builder_unsafe()
						// Descend becomes Alias with one additional parent.
						.alias_origin((Parent, Parachain(1002)))
						// The rest stays the same since it was already meant
						// for execution on the destination.
						.withdraw_asset((Parent, 10_000_000_000u128))
						.buy_execution((Parent, 100_000_000u128), Unlimited)
						.transact(
							OriginKind::SovereignAccount,
							Weight::from_parts(10_000_000_000, 100_000),
							Vec::new()
						)
						.refund_surplus()
						.deposit_asset(AllCounted(1), [1u8; 32])
						.build()
				)),
				dest: Box::new(VersionedLocation::from(
					// Added 1 more parent.
					Location::new(1, [Parachain(1004)])
				)),
			}),
		);
	}
}
