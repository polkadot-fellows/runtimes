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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

//! Code shared between all runtimes
#![cfg_attr(not(feature = "std"), no_std)]

/// TODO: Remove this implementation once `polkadot-sdk` includes support
/// for generating an `overestimated_xcm` using `WithdrawAsset` instructions, as done here
///
/// A `ToParachainDeliveryHelper` implementation already exists in `polkadot-sdk`,
/// but it incorrectly estimates the worst-case delivery fees, causing insufficient
/// funds to be deposited for benchmarking
///
/// This version is a temporary replacement until the issue is fixed upstream
#[cfg(feature = "runtime-benchmarks")]
pub mod xcm_sender {

	extern crate alloc;

	use alloc::{vec, vec::Vec};
	use frame_support::traits::Get;
	use polkadot_primitives::Id as ParaId;
	use polkadot_runtime_common::xcm_sender::{EnsureForParachain, PriceForMessageDelivery};
	use xcm::{latest::MAX_ITEMS_IN_ASSETS, prelude::*};

	pub const MAX_INSTRUCTIONS_TO_DECODE: usize = 100;

	/// Implementation of `xcm_builder::EnsureDelivery` which helps to ensure delivery to the
	/// `ParaId` parachain (sibling or child). Deposits existential deposit for origin (if needed).
	/// Deposits estimated fee to the origin account (if needed).
	/// Allows to trigger additional logic for specific `ParaId` (e.g. open HRMP channel) (if
	/// needed).
	pub struct ToParachainDeliveryHelper<
		XcmConfig,
		ExistentialDeposit,
		PriceForDelivery,
		ParaId,
		ToParaIdHelper,
	>(
		core::marker::PhantomData<(
			XcmConfig,
			ExistentialDeposit,
			PriceForDelivery,
			ParaId,
			ToParaIdHelper,
		)>,
	);

	impl<
			XcmConfig: xcm_executor::Config,
			ExistentialDeposit: Get<Option<Asset>>,
			PriceForDelivery: PriceForMessageDelivery<Id = ParaId>,
			Parachain: Get<ParaId>,
			ToParachainHelper: EnsureForParachain,
		> xcm_builder::EnsureDelivery
		for ToParachainDeliveryHelper<
			XcmConfig,
			ExistentialDeposit,
			PriceForDelivery,
			Parachain,
			ToParachainHelper,
		>
	{
		fn ensure_successful_delivery(
			origin_ref: &Location,
			dest: &Location,
			fee_reason: xcm_executor::traits::FeeReason,
		) -> (Option<xcm_executor::FeesMode>, Option<Assets>) {
			use xcm_executor::{
				traits::{FeeManager, TransactAsset},
				FeesMode,
			};

			// check if the destination matches the expected `Parachain`.
			if let Some(Parachain(para_id)) = dest.first_interior() {
				if ParaId::from(*para_id) != Parachain::get() {
					return (None, None)
				}
			} else {
				return (None, None)
			}

			let mut fees_mode = None;
			if !XcmConfig::FeeManager::is_waived(Some(origin_ref), fee_reason) {
				// if not waived, we need to set up accounts for paying and receiving fees

				// mint ED to origin if needed
				if let Some(ed) = ExistentialDeposit::get() {
					XcmConfig::AssetTransactor::deposit_asset(&ed, origin_ref, None).unwrap();
				}

				// overestimate delivery fee
				let mut max_assets: Vec<Asset> = Vec::new();
				for i in 0..MAX_ITEMS_IN_ASSETS {
					max_assets.push((GeneralIndex(i as u128), 100u128).into());
				}
				let overestimated_xcm =
					vec![WithdrawAsset(max_assets.into()); MAX_INSTRUCTIONS_TO_DECODE].into();
				let overestimated_fees =
					PriceForDelivery::price_for_delivery(Parachain::get(), &overestimated_xcm);

				// mint overestimated fee to origin
				for fee in overestimated_fees.inner() {
					XcmConfig::AssetTransactor::deposit_asset(fee, origin_ref, None).unwrap();
				}

				// allow more initialization for target parachain
				ToParachainHelper::ensure(Parachain::get());

				// expected worst case - direct withdraw
				fees_mode = Some(FeesMode { jit_withdraw: true });
			}
			(fees_mode, None)
		}
	}
}
