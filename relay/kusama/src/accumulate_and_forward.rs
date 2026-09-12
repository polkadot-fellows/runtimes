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

//! XCM adapter for `pallet-accumulate-and-forward`.

use alloc::vec;
use core::marker::PhantomData;
use frame_support::{
	storage::{with_transaction, TransactionOutcome},
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
};
use sp_runtime::DispatchError;
use xcm::latest::prelude::*;
use xcm_executor::traits::TransactAsset;

const LOG_TARGET: &str = "xcm::accumulate-forward";

/// Fails `try-runtime` if the accumulation account lacks the ED, without which dust is burned.
pub struct EnsureAccumulationAccountFunded<T>(PhantomData<T>);

impl<T: pallet_accumulate_and_forward::Config> EnsureAccumulationAccountFunded<T> {
	fn is_funded() -> bool {
		use frame_support::traits::fungible::Inspect;

		let account = pallet_accumulate_and_forward::Pallet::<T>::accumulation_account();
		<T as pallet_accumulate_and_forward::Config>::Currency::balance(&account) >=
			<T as pallet_accumulate_and_forward::Config>::Currency::minimum_balance()
	}
}

impl<T: pallet_accumulate_and_forward::Config> OnRuntimeUpgrade
	for EnsureAccumulationAccountFunded<T>
{
	fn on_runtime_upgrade() -> Weight {
		if !Self::is_funded() {
			log::error!(
				target: LOG_TARGET,
				"🚨 accumulation account is below the ED: its sinks will burn. Fund it."
			);
		}

		<T as frame_system::Config>::DbWeight::get().reads(1)
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		frame_support::ensure!(
			Self::is_funded(),
			"accumulation account is not funded with the existential deposit"
		);
		Ok(())
	}
}

/// [`pallet_accumulate_and_forward::Forwarder`] teleporting to Asset Hub to burn there, since
/// Kusama tracks `TotalIssuance` on Asset Hub. Check-out is a no-op: this relay tracks no
/// teleports. A failed send rolls back; once queued, a failure on arrival traps the assets.
pub struct TeleportAndBurnForwarder<XcmConfig, Dest, NativeAsset>(
	PhantomData<(XcmConfig, Dest, NativeAsset)>,
);

impl<XcmConfig, Dest, NativeAsset> TeleportAndBurnForwarder<XcmConfig, Dest, NativeAsset>
where
	XcmConfig: xcm_executor::Config,
	Dest: Get<Location>,
	NativeAsset: Get<Location>,
{
	fn teleport_and_burn(source: [u8; 32], amount: u128) -> Result<(), XcmError> {
		type Transactor<C> = <C as xcm_executor::Config>::AssetTransactor;

		let dest = Dest::get();
		let asset = Asset { id: AssetId(NativeAsset::get()), fun: Fungible(amount) };
		let source_location: Location =
			Junction::AccountId32 { network: None, id: source }.into_location();
		let context = XcmContext { origin: None, message_id: [0; 32], topic: None };

		let withdrawn = Transactor::<XcmConfig>::withdraw_asset(&asset, &source_location, None)?;

		// TODO https://github.com/polkadot-fellows/runtimes/issues/404
		Transactor::<XcmConfig>::can_check_out(&dest, &asset, &context)?;

		let assets = withdrawn.reanchored_assets(
			&dest,
			&<XcmConfig as xcm_executor::Config>::UniversalLocation::get(),
		);

		send_xcm::<<XcmConfig as xcm_executor::Config>::XcmSender>(
			dest.clone(),
			Xcm(vec![
				UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
				ReceiveTeleportedAsset(assets.clone()),
				BurnAsset(assets),
			]),
		)?;

		Transactor::<XcmConfig>::check_out(&dest, &asset, &context);

		Ok(())
	}
}

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
		with_transaction(|| -> TransactionOutcome<Result<(), DispatchError>> {
			match Self::teleport_and_burn(source.into(), amount.into()) {
				Ok(()) => TransactionOutcome::Commit(Ok(())),
				Err(error) => {
					log::debug!(
						target: LOG_TARGET,
						"accumulate-forward: teleport-and-burn failed: {error:?}"
					);

					TransactionOutcome::Rollback(Err(DispatchError::Other(
						"teleport-and-burn failed",
					)))
				},
			}
		})
		.map_err(|_| ())
	}
}
