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

use alloc::vec;
use core::marker::PhantomData;
use codec::{Decode, Encode, MaxEncodedLen};
use encointer_balances_tx_payment::ONE_KSM;
use frame_support::pallet_prelude::TypeInfo;
use frame_support::parameter_types;
use frame_support::traits::{
    tokens::{PaymentStatus},
    Get,
};
use xcm::{opaque::lts::Weight, prelude::*};
use xcm_executor::traits::{QueryHandler, QueryResponseStatus};
use pallet_encointer_treasuries::Payout;
use xcm::v5::Junctions::X2;
use crate::xcm_config::KsmLocation;

/// Payout an asset at asset hub.
///
/// The idea is to only support stable coins for now.
pub struct PayoutOverXcmAtAssetHub<
    Router,
    Querier,
    Timeout,
    Beneficiary,
    AssetKind,
>(
    PhantomData<(
        Router,
        Querier,
        Timeout,
        Beneficiary,
        AssetKind,
    )>,
);
impl<
    Router: SendXcm,
    Querier: QueryHandler,
    Timeout: Get<Querier::BlockNumber>,
    AccountId: Clone + Into<[u8; 32]>,
    AssetKind: Clone + Into<Location>,
> Payout
for PayoutOverXcmAtAssetHub<
    Router,
    Querier,
    Timeout,
    AccountId,
    AssetKind,
>
{
    type Balance = u128;
    type AccountId = AccountId;
    type AssetKind = AssetKind;
    type Id = QueryId;
    type Error = xcm::latest::Error;

    fn pay(
        from: &Self::AccountId,
        to: &Self::AccountId,
        asset_kind: Self::AssetKind,
        amount: Self::Balance,
    ) -> Result<Self::Id, Self::Error> {
        let destination = AssetHubLocation::get();
        let query_id = Querier::new_query(asset_kind.clone().into(), Timeout::get(), from.clone().into());

        let message = Xcm(vec![
            DescendOrigin(AccountId32 { network: None, id: from.clone().into() }.into()),
            WithdrawAsset(vec![Asset { id: KsmLocation::get().into(), fun: Fungible(ONE_KSM) }].into()),
            PayFees { asset: (KsmLocation::get(), ONE_KSM).into()},
            SetAppendix(Xcm(vec![
                SetFeesMode { jit_withdraw: true },
                ReportError(QueryResponseInfo {
                    destination: destination.clone(),
                    query_id,
                    max_weight: Weight::zero(),
                }),
            ])),
            TransferAsset {
                beneficiary: AccountId32 { network: None, id: to.clone().into() }.into(),
                assets:(asset_id(asset_kind.clone()), amount).into(),
            },
        ]);

        let (ticket, _) = Router::validate(&mut Some(destination), &mut Some(message))?;
        Router::deliver(ticket)?;
        Ok(query_id.into())
    }

    fn is_asset_supported(_: &Self::AssetKind) -> bool {
        true
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
    fn ensure_successful(_: &Self::AccountId, _: Self::AssetKind, _: Self::Balance) {
        // We cannot generally guarantee this will go through successfully since we don't have any
        // control over the XCM transport layers. We just assume that the benchmark environment
        // will be sending it somewhere sensible.
    }

    #[cfg(feature = "runtime-benchmarks")]
    fn ensure_concluded(id: Self::Id) {
        Querier::expect_response(id, Response::ExecutionResult(None));
    }
}


parameter_types! {
	pub AssetHubLocation: Location = Location::new(1, [Parachain(1000)]);
}

#[derive(Debug, Encode, Decode, TypeInfo, MaxEncodedLen, PartialEq, Eq, Clone, Copy)]
pub enum SupportedPayouts {
    Usdt
}

impl From<SupportedPayouts> for Location {
    fn from(asset: SupportedPayouts) -> Self {
        match asset {
            SupportedPayouts::Usdt => Location {
                parents: 1, interior: X2(
                    [PalletInstance(50), GeneralIndex(1984)].into()
                )
            }
        }
    }
}

pub fn asset_id<T: Into<Location>>(value: T) -> AssetId {
    let location: Location = value.into();
    AssetId::from(location)
}