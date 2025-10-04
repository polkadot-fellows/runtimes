// Copyright (c) 2023 Encointer Association
// This file is part of Encointer
//
// Encointer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Encointer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Encointer.  If not, see <http://www.gnu.org/licenses/>.

//! `TransferOverXcm` struct for paying through XCM and getting the status back.

use crate::xcm_config::KsmLocation;
use alloc::vec;
use core::marker::PhantomData;
use frame_support::traits::{tokens::PaymentStatus, Get};
use sp_runtime::traits::TryConvert;
use xcm::{latest::Error, prelude::*};
use xcm_builder::LocatableAssetId;
use xcm_executor::traits::{QueryHandler, QueryResponseStatus};

pub use pallet_encointer_treasuries::Transfer;

// This is the value that has been queried from the Asset Hub Kusama runtime.
// There is an integration test in `integration-tests/emulated/tests/encointer/encointer-kusama/
// That verifies that this fee is correct and will catch fee changes in Asset-Hub Kusama
pub const REMOTE_XCM_TRANSFER_REMOTE_EXECUTION_FEE: u128 = 12699199987;

pub trait GetRemoteFee {
	fn get_remote_fee(xcm: Xcm<()>, asset_id: Option<AssetId>) -> Asset;
}

pub struct ConstantKsmFee;

impl GetRemoteFee for ConstantKsmFee {
	fn get_remote_fee(_xcm: Xcm<()>, _asset_id: Option<AssetId>) -> Asset {
		fee_asset(REMOTE_XCM_TRANSFER_REMOTE_EXECUTION_FEE)
	}
}

pub fn fee_asset(amount: u128) -> Asset {
	(KsmLocation::get(), amount).into()
}

/// Transfer an asset on a remote chain (in practice this should be only asset hub).
///
/// It is similar to the `PayOverXcm` struct from the polkadot-sdk with the difference
/// that the source account executing the transaction is function parameter.
///
/// The account transferring funds remotely will be for example:
///  * `Location::new(1, XX([Parachain(SourceParaId), from_location.interior ])`
#[allow(clippy::type_complexity)]
pub struct TransferOverXcm<
	Router,
	Querier,
	Timeout,
	Transactors,
	AssetKind,
	AssetKindToLocatableAsset,
	TransactorRefToLocation,
	RemoteFee,
>(
	PhantomData<(
		Router,
		Querier,
		Timeout,
		Transactors,
		AssetKind,
		AssetKindToLocatableAsset,
		TransactorRefToLocation,
		RemoteFee,
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
		RemoteFee: GetRemoteFee,
	> Transfer
	for TransferOverXcm<
		Router,
		Querier,
		Timeout,
		Transactor,
		AssetKind,
		AssetKindToLocatableAsset,
		TransactorRefToLocation,
		RemoteFee,
	>
{
	type Balance = u128;
	type Payer = Transactor;
	type Beneficiary = Transactor;
	type AssetKind = AssetKind;
	type Id = QueryId;
	type Error = Error;

	fn transfer(
		from: &Self::Payer,
		to: &Self::Beneficiary,
		asset_kind: Self::AssetKind,
		amount: Self::Balance,
	) -> Result<Self::Id, Self::Error> {
		let (message, asset_location, query_id) =
			Self::get_remote_transfer_xcm(from, to, asset_kind, amount)?;

		let (ticket, _delivery_fees) =
			Router::validate(&mut Some(asset_location), &mut Some(message))?;
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
		asset_kind: Self::AssetKind,
		_: Self::Balance,
	) {
		let locatable = AssetKindToLocatableAsset::try_convert(asset_kind).unwrap();
		Router::ensure_successful_delivery(Some(locatable.location));
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn ensure_concluded(id: Self::Id) {
		Querier::expect_response(id, Response::ExecutionResult(None));
	}
}

impl<
		Router: SendXcm,
		Querier: QueryHandler,
		Timeout: Get<Querier::BlockNumber>,
		Transactor: Clone + core::fmt::Debug,
		AssetKind: Clone + core::fmt::Debug,
		AssetKindToLocatableAsset: TryConvert<AssetKind, LocatableAssetId>,
		TransactorRefToLocation: for<'a> TryConvert<&'a Transactor, Location>,
		RemoteFee: GetRemoteFee,
	>
	TransferOverXcm<
		Router,
		Querier,
		Timeout,
		Transactor,
		AssetKind,
		AssetKindToLocatableAsset,
		TransactorRefToLocation,
		RemoteFee,
	>
{
	/// Gets the XCM executing the transfer on the remote chain.
	pub fn get_remote_transfer_xcm(
		from: &<Self as Transfer>::Payer,
		to: &<Self as Transfer>::Beneficiary,
		asset_kind: <Self as Transfer>::AssetKind,
		amount: <Self as Transfer>::Balance,
	) -> Result<(Xcm<()>, Location, QueryId), Error> {
		let locatable = Self::locatable_asset_id(asset_kind)?;
		let LocatableAssetId { asset_id, location: asset_location } = locatable;

		let origin_location_on_remote = Self::origin_location_on_remote(&asset_location)?;

		let from_location = TransactorRefToLocation::try_convert(from).map_err(|e| {
			log::error!("Could not convert `from` into Location: {e:?}");
			Error::InvalidLocation
		})?;

		let beneficiary = TransactorRefToLocation::try_convert(to).map_err(|e| {
			log::error!("Could not convert `beneficiary` into Location: {e:?}");
			Error::InvalidLocation
		})?;

		let query_id = Querier::new_query(
			asset_location.clone(),
			Timeout::get(),
			from_location.interior.clone(),
		);

		let fee_asset = RemoteFee::get_remote_fee(Xcm::new(), None);

		let message = remote_transfer_xcm(
			from_location,
			origin_location_on_remote,
			beneficiary,
			asset_id,
			amount,
			fee_asset,
			query_id,
		)?;

		Ok((message, asset_location, query_id))
	}

	/// Returns the `from` relative to the asset's location.
	///
	/// This is the account that executes the transfer on the remote chain.
	pub fn from_on_remote(
		from: &<Self as Transfer>::Payer,
		asset_kind: <Self as Transfer>::AssetKind,
	) -> Result<Location, Error> {
		let from_location = TransactorRefToLocation::try_convert(from).map_err(|e| {
			log::error!("Could not convert `from` into Location: {e:?}");
			Error::InvalidLocation
		})?;

		let locatable = Self::locatable_asset_id(asset_kind)?;

		let origin_location_on_remote = Self::origin_location_on_remote(&locatable.location)?;
		origin_location_on_remote
			.appended_with(from_location)
			.map_err(|_| Error::LocationFull)
	}

	fn origin_location_on_remote(asset_location: &Location) -> Result<Location, Error> {
		let origin_on_remote = Querier::UniversalLocation::get()
			.invert_target(asset_location)
			.map_err(|()| Error::LocationNotInvertible)?;
		Ok(origin_on_remote)
	}

	fn locatable_asset_id(
		asset_kind: <Self as Transfer>::AssetKind,
	) -> Result<LocatableAssetId, Error> {
		AssetKindToLocatableAsset::try_convert(asset_kind).map_err(|e| {
			log::error!("Could not convert asset kind to locatable asset: {e:?}");
			Error::InvalidLocation
		})
	}
}

fn remote_transfer_xcm(
	from_location: Location,
	destination: Location,
	beneficiary: Location,
	asset_id: AssetId,
	amount: u128,
	remote_fee: Asset,
	_query_id: QueryId,
) -> Result<Xcm<()>, Error> {
	// Transform `from` into Location::new(1, XX([Parachain(source), from.interior }])
	// We need this one for the refunds.
	let from_at_target = append_from_to_target(from_location.clone(), destination.clone())?;

	let xcm = Xcm(vec![
		// Transform origin into Location::new(1, X2([Parachain(SourceParaId), from.interior }])
		DescendOrigin(from_location.interior.clone()),
		// For simplicity, we assume now that the treasury has KSM and pays fees with KSM.
		WithdrawAsset(vec![remote_fee.clone()].into()),
		PayFees { asset: remote_fee },
		SetAppendix(Xcm(vec![
			RefundSurplus,
			DepositAsset { assets: AssetFilter::Wild(WildAsset::All), beneficiary: from_at_target },
		])),
		TransferAsset { beneficiary, assets: (asset_id, amount).into() },
	]);

	Ok(xcm)
}

fn append_from_to_target(from: Location, target: Location) -> Result<Location, Error> {
	let from_at_target = target.appended_with(from).map_err(|_| Error::LocationFull)?;
	Ok(from_at_target)
}
