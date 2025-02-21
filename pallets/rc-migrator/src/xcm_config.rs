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

//! XCM configurations for the Relay Chain for the AHM migration.

use frame_support::{parameter_types, PalletId};
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use xcm::prelude::*;
use xcm_builder::{FungibleAdapter, IsConcrete, MintLocation};

fn check_account() -> AccountId32 {
	const CHECK_ACCOUNT_ID: PalletId = PalletId(*b"py/xcmch");
	AccountIdConversion::<AccountId32>::into_account_truncating(&CHECK_ACCOUNT_ID)
}

parameter_types! {
	pub const TokenLocation: Location = Here.into_location();
	/// The Checking Account along with the indication that the local chain is able to mint tokens.
	pub TrackingTeleportOut: (AccountId32, MintLocation) = (check_account(), MintLocation::Local);
}

/// Native token asset transactor.
/// Only aware of the Balances pallet, which is mapped to `TokenLocation`.
/// Tracks teleports of native token to keep total issuance consistent.
pub type AssetTransactorBefore<T, AccountIdConverter> = FungibleAdapter<
	// Use this currency:
	pallet_balances::Pallet<T>,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<TokenLocation>,
	// We can convert the `Location`s with our converter above:
	AccountIdConverter,
	// Our chain's account ID type
	AccountId32,
	// We track our teleports in/out to keep total issuance correct.
	TrackingTeleportOut,
>;

/// Native token asset transactor.
/// Only aware of the Balances pallet, which is mapped to `TokenLocation`.
/// Does not track teleports of native token. Total issuance tracking migrated to AH.
pub type AssetTransactorDuringAfter<T, AccountIdConverter> = FungibleAdapter<
	// Use this currency:
	pallet_balances::Pallet<T>,
	// Use this currency when it is a fungible asset matching the given location or name:
	IsConcrete<TokenLocation>,
	// We can convert the `Location`s with our converter above:
	AccountIdConverter,
	// Our chain's account ID type
	AccountId32,
	// No tracking of teleports.
	(),
>;
