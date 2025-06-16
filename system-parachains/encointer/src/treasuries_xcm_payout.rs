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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

//! `PayOverXcm` struct for paying through XCM and getting the status back.

use crate::xcm_config::KsmLocation;
use alloc::vec;
use core::marker::PhantomData;
use encointer_balances_tx_payment::ONE_KSM;
use frame_support::traits::{tokens::PaymentStatus, Get};
use pallet_encointer_treasuries::Transfer;
use sp_runtime::traits::TryConvert;
use xcm::{opaque::lts::Weight, prelude::*};
use xcm_builder::LocatableAssetId;
use xcm_executor::traits::{QueryHandler, QueryResponseStatus};

pub const BASE_FEE: u128 = 4 * ONE_KSM / 10;

/// Transfer an asset at asset hub.
///
/// The idea is to only support stable coins for now.
pub struct TransferOverXcm<
	Router,
	Querier,
	Timeout,
	Transactors,
	AssetKind,
	AssetKindToLocatableAsset,
	TransactorRefToLocation,
>(
	PhantomData<(
		Router,
		Querier,
		Timeout,
		Transactors,
		AssetKind,
		AssetKindToLocatableAsset,
		TransactorRefToLocation,
	)>,
);
impl<
		Router: SendXcm,
		Querier: QueryHandler,
		Timeout: Get<Querier::BlockNumber>,
		Transactor: Clone + core::fmt::Debug,
		AssetKind: Clone + core::fmt::Debug,
		AssetKindToLocatableAsset: TryConvert<AssetKind, LocatableAssetId>,
		TransactorRefToLocation: for<'a> TryConvert<&'a Transactor, Location>,
	> Transfer
	for TransferOverXcm<
		Router,
		Querier,
		Timeout,
		Transactor,
		AssetKind,
		AssetKindToLocatableAsset,
		TransactorRefToLocation,
	>
{
	type Balance = u128;
	type Payer = Transactor;
	type Beneficiary = Transactor;
	type AssetKind = AssetKind;
	type Id = QueryId;
	type Error = xcm::latest::Error;

	fn transfer(
		from: &Self::Payer,
		to: &Self::Beneficiary,
		asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		let locatable = AssetKindToLocatableAsset::try_convert(asset_kind)
			.map_err(|_| xcm::latest::Error::InvalidLocation)?;
		let LocatableAssetId { asset_id, location: asset_location } = locatable;
		let destination = Querier::UniversalLocation::get()
			.invert_target(&asset_location)
			.map_err(|()| Self::Error::LocationNotInvertible)?;

		log::info!("Destination: {:?}", destination);

		let from_location = TransactorRefToLocation::try_convert(from)
			.map_err(|_| xcm::latest::Error::InvalidLocation)?;

		log::info!("From Location: {:?}", from_location);

		// Transform `from` into Location::new(1, XX([Parachain(source), ...from.interior }])
		// We need this one for the refunds.
		let from_at_target = destination
			.clone()
			.appended_with(from_location.clone())
			.map_err(|_| Self::Error::LocationFull)?;

		log::info!("From at target: {:?}", from_location);

		let beneficiary = TransactorRefToLocation::try_convert(to)
			.map_err(|_| xcm::latest::Error::InvalidLocation)?;

		let query_id = Querier::new_query(
			asset_location.clone(),
			Timeout::get(),
			from_location.interior.clone(),
		);

		let fee_asset = fee_asset(BASE_FEE);

		let message = Xcm(vec![
			// Transform origin into Location::new(1, X2([Parachain(42), from.interior }])
			DescendOrigin(from_location.interior.clone()),
			// For simplicity, we assume now that the treasury has KSM and pays fees with KSM.
			WithdrawAsset(vec![fee_asset.clone()].into()),
			PayFees { asset: fee_asset },
			WithdrawAsset(vec![Asset { id: asset_id.clone(), fun: Fungible(amount) }].into()),
			SetAppendix(Xcm(vec![
				ReportError(QueryResponseInfo {
					destination: destination.clone(),
					query_id,
					max_weight: Weight::zero(),
				}),
				RefundSurplus,
				DepositAsset {
					assets: AssetFilter::Wild(WildAsset::All),
					beneficiary: from_at_target,
				},
			])),
			TransferAsset { beneficiary, assets: (asset_id, amount).into() },
		]);

		let (ticket, _) = Router::validate(&mut Some(destination), &mut Some(message))?;
		Router::deliver(ticket)?;
		Ok(query_id)
	}

	fn check_payment(id: Self::Id) -> PaymentStatus {
		use QueryResponseStatus::*;
		match Querier::take_response(id) {
			Ready { response, .. } => match response {
				Response::ExecutionResult(None) => PaymentStatus::Success,
				Response::ExecutionResult(Some(_)) => PaymentStatus::Failure,
				_ => PaymentStatus::Unknown,
			},
			Pending { .. } => PaymentStatus::InProgress,
			NotFound | UnexpectedVersion => PaymentStatus::Unknown,
		}
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_successful(
		_: &Self::Payer,
		_: &Self::Beneficiary,
		_: Self::AssetKind,
		_: Self::Balance,
	) {
		// We cannot generally guarantee this will go through successfully since we don't have any
		// control over the XCM transport layers. We just assume that the benchmark environment
		// will be sending it somewhere sensible.
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_concluded(id: Self::Id) {
		Querier::expect_response(id, Response::ExecutionResult(None));
	}
}

// Todo: this is going to be replaced, as we will have a proper fee mechanism
pub fn fee_asset(amount: u128) -> Asset {
	(KsmLocation::get(), amount).into()
}
