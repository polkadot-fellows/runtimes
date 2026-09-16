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

//! XCM adapter for `pallet-accumulate-and-forward` used by the Kusama system chains.

use alloc::vec;
use core::marker::PhantomData;
use frame_support::{
	storage::{with_transaction, TransactionOutcome},
	traits::Get,
	BoundedVec,
};
use sp_runtime::DispatchError;
use xcm::latest::{prelude::*, AssetTransferFilter};
use xcm_executor::XcmExecutor;

const LOG_TARGET: &str = "xcm::accumulate-forward";

/// [`pallet_accumulate_and_forward::Forwarder`] teleporting to Asset Hub to burn there, since
/// Kusama tracks `TotalIssuance` on Asset Hub. The Polkadot counterpart is
/// `xcm_builder::TeleportForwarderForAccountId32`, which deposits to DAP staging instead.
///
/// Execution failures roll back. Once the message is queued nothing can be trapped, since the only
/// instruction after the receive burns exactly what it put in holding, but a rejection there
/// leaves the KSM burned here with Asset Hub untouched.
// TODO: drop this and use `xcm_builder::TeleportForwarderForAccountId32` once it is reusable
// for teleport-and-burn: https://github.com/paritytech/polkadot-sdk/issues/13238
pub struct TeleportAndBurnForwarder<XcmConfig, Dest, NativeAsset>(
	PhantomData<(XcmConfig, Dest, NativeAsset)>,
);

impl<XcmConfig, Dest, NativeAsset, AccountId, Balance>
	pallet_accumulate_and_forward::Forwarder<AccountId, Balance>
	for TeleportAndBurnForwarder<XcmConfig, Dest, NativeAsset>
where
	XcmConfig: xcm_executor::Config,
	Dest: Get<Location>,
	NativeAsset: Get<Location>,
	AccountId: Into<[u8; 32]>,
	Balance: Into<u128>,
{
	fn forward(source: AccountId, amount: Balance) -> Result<(), ()> {
		let dest = Dest::get();
		let asset = Asset { id: AssetId(NativeAsset::get()), fun: Fungible(amount.into()) };

		// `BurnAsset` takes concrete assets, so the amount has to be named as Asset Hub sees it.
		let remote_asset = asset
			.clone()
			.reanchored(&dest, &XcmConfig::UniversalLocation::get())
			.map_err(|asset| {
				log::error!(target: LOG_TARGET, "🚨 could not reanchor {asset:?} for {dest:?}");
			})?;
		// The XCM flow: `ReceiveTeleportedAsset → ClearOrigin → PayFees → RefundSurplus →
		// BurnAsset`, which Asset Hub's existing paid barrier accepts unchanged.
		let remote_xcm = Xcm(vec![RefundSurplus, BurnAsset(remote_asset.clone().into())]);
		let xcm: Xcm<XcmConfig::RuntimeCall> = Xcm(vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			DescendOrigin(Junction::AccountId32 { network: None, id: source.into() }.into()),
			WithdrawAsset(asset.clone().into()),
			InitiateTransfer {
				destination: dest,
				remote_fees: Some(AssetTransferFilter::Teleport(remote_asset.into())),
				preserve_origin: false,
				assets: BoundedVec::truncate_from(vec![]),
				remote_xcm,
			},
		]);

		with_transaction(|| -> TransactionOutcome<Result<(), DispatchError>> {
			let outcome = XcmExecutor::<XcmConfig>::prepare_and_execute(
				Location::here(),
				xcm,
				&mut [0u8; 32],
				Weight::MAX,
				Weight::MAX,
			);

			match outcome {
				Outcome::Complete { .. } => TransactionOutcome::Commit(Ok(())),
				exec_error => {
					log::debug!(
						target: LOG_TARGET,
						"accumulate-forward: XCM execution failed: {exec_error:?}"
					);

					TransactionOutcome::Rollback(Err(DispatchError::Other("XCM execution failed")))
				},
			}
		})
		.map_err(|_| ())
	}
}
