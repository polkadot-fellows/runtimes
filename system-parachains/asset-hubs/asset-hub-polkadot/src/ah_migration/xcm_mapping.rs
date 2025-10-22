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
#[allow(clippy::result_unit_err)]
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
			let reanchored_assets = reanchor_asset_filter(assets, ah_location, universal_location)?;
			// The nested xcm is already from the perspective of the dest, so no reanchoring needed.
			Ok(DepositReserveAsset { assets: reanchored_assets, dest: reanchored_dest, xcm })
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
	if location.parents != 0 && location.interior.len() != 1 {
		return false;
	}

	matches!(
		location.interior.first(),
		Some(Junction::AccountId32 { .. }) | Some(Junction::AccountKey20 { .. })
	)
}

fn reanchor_asset_filter(
	filter: AssetFilter,
	ah_location: &Location,
	universal_location: &InteriorLocation,
) -> Result<AssetFilter, ()> {
	match filter {
		AssetFilter::Definite(assets) => {
			let reanchored_assets =
				assets.reanchored(ah_location, universal_location).map_err(|err| {
					log::error!(target: LOG_TARGET, "Failed to reanchor asset filter assets: {err:?}");
				})?;
			Ok(AssetFilter::Definite(reanchored_assets))
		},
		AssetFilter::Wild(wild) => {
			// Wild filters don't contain specific locations, so pass through unchanged.
			Ok(AssetFilter::Wild(wild))
		},
	}
}

#[cfg(test)]
mod tests {
	use crate::ah_migration::{RcRuntimeCall, RcToAhCall, RcXcmCall, RuntimeCall, RuntimeOrigin};

	use codec::DecodeAll;
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

	// Test that the referendum from https://polkadot.subsquare.io/referenda/1770 gets mapped
	// and the mapping is correct.
	#[test]
	fn mapping_complex_ref_works() {
		let call_hex = "0x1a02081a0300016d6f646c70792f747273727900000000000000000000000000000000000000006303051400040000000f00109b70ce1627300000000700e40b54020e010204000100c91f08130100000700e40b5402000d010204010100b10f140d010204000101006d6f646c70792f7472737279000000000000000000000000000000000000000062a4f7b739a90104020000000000630005000100c91f05180b0100b10f00040100000700e40b5402130100000700e40b5402000601010700e40b5402821a0600b50142007369626cec03000000000000000000000000000000000000000000000000000014000000008848065a1627000000000000000000010a01204e000001102700000005000000de0000000038be697903000000000000000000000000d02d048daefb39000000000000000000140d010204010100b10f";
		let call_bytes = hex::decode(&call_hex[2..]).expect("Invalid hex string");
		let rc_call = RcRuntimeCall::decode_all(&mut call_bytes.as_slice()).expect("Invalid bytes");
		let ah_call = RcToAhCall::map(rc_call).expect("Failed to map RC call to AH call");
		assert_eq!(
			ah_call,
			RuntimeCall::Utility(pallet_utility::Call::batch_all {
				calls: vec![
					RuntimeCall::Utility(pallet_utility::Call::dispatch_as {
						as_origin: Box::new(RuntimeOrigin::signed(
							sp_runtime::AccountId32::from([109, 111, 100, 108, 112, 121, 47, 116, 114, 115, 114, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
						).caller),
						call: Box::new(RuntimeCall::PolkadotXcm(pallet_xcm::Call::execute {
							message: Box::new(VersionedXcm::V5(
								Xcm::builder()
									// DOT turns from `Here` to `Parent`.
									.withdraw_asset((Parent, 11_002_600_000_000_000u128))
									.pay_fees((Parent, 10_000_000_000u128))
									.deposit_reserve_asset(
										AllCounted(1),
										// A `Parent` is added.
										(Parent, Parachain(2034)),
										// This stays exactly the same as it was already meant for the destination.
										Xcm::builder_unsafe()
											.buy_execution(
												(Parent, 10_000_000_000u128),
												Unlimited
											)
											.deposit_asset(
												AllCounted(1),
												(Parent, Parachain(1004))
											)
											.build()
									)
									.refund_surplus()
									.deposit_asset(
										AllCounted(1),
										// Local accounts don't get reanchored.
										[109, 111, 100, 108, 112, 121, 47, 116, 114, 115, 114, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
									)
									.build()
								)),
							max_weight: Weight::from_parts(771_615_000, 10_830),
						})),
					}),
					RuntimeCall::Scheduler(pallet_scheduler::Call::schedule_after {
						after: 2,
						maybe_periodic: None,
						priority: 0,
						call: Box::new(RuntimeCall::PolkadotXcm(pallet_xcm::Call::send {
							dest: Box::new(VersionedLocation::V5(Location::new(1, [Parachain(2034)]))),
							message: Box::new(VersionedXcm::V5(
								Xcm::builder_unsafe()
									// This descend won't really work if origin is AH,
									// so it shouldn't be used in refs pre AHM.
									.descend_origin(Parachain(1004))
									.withdraw_asset((Parent, 10_000_000_000u128))
									.buy_execution((Parent, 10_000_000_000u128), Unlimited)
									.transact(
										OriginKind::SovereignAccount,
										Weight::from_parts(10000000000, 100000),
										hex::decode("42007369626cec03000000000000000000000000000000000000000000000000000014000000008848065a1627000000000000000000010a01204e000001102700000005000000de0000000038be697903000000000000000000000000d02d048daefb39000000000000000000").unwrap()
									)
									.refund_surplus()
									.deposit_asset(
										AllCounted(1),
										(Parent, Parachain(1004))
									)
									.build()
							))
						}))
					})
				],
			}),
		);
	}
}
