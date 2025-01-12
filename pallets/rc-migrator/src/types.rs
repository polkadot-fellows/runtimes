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

//! Types

use super::*;

pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;

/// Relay Chain Freeze Reason
#[derive(Encode, Decode)]
pub enum AssetHubPalletConfig<T: Config> {
	#[codec(index = 255)]
	AhmController(AhMigratorCall<T>),
}

/// Call encoding for the calls needed from the Broker pallet.
#[derive(Encode, Decode)]
pub enum AhMigratorCall<T: Config> {
	#[codec(index = 0)]
	ReceiveAccounts { accounts: Vec<accounts::AccountFor<T>> },
	#[codec(index = 1)]
	ReceiveMultisigs { multisigs: Vec<multisig::RcMultisigOf<T>> },
}

/// Copy of `ParaInfo` type from `paras_registrar` pallet.
///
/// From: https://github.com/paritytech/polkadot-sdk/blob/b7afe48ed0bfef30836e7ca6359c2d8bb594d16e/polkadot/runtime/common/src/paras_registrar/mod.rs#L50-L59
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, RuntimeDebug, TypeInfo)]
pub struct ParaInfo<AccountId, Balance> {
	/// The account that has placed a deposit for registering this para.
	pub manager: AccountId,
	/// The amount reserved by the `manager` account for the registration.
	pub deposit: Balance,
	/// Whether the para registration should be locked from being controlled by the manager.
	/// None means the lock had not been explicitly set, and should be treated as false.
	pub locked: Option<bool>,
}

/// Weight information for the processing the packages from this pallet on the Asset Hub.
pub trait AhWeightInfo {
	/// Weight for processing a single account on AH.
	fn migrate_account() -> Weight;
}

impl AhWeightInfo for () {
	fn migrate_account() -> Weight {
		Weight::from_all(1)
	}
}

pub trait PalletMigration {
	type Key;
	type Error;

	fn first_key(weight: &mut WeightMeter) -> Result<Option<Self::Key>, Self::Error>;
	fn migrate_many(
		maybe_last_key: Self::Key,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error>;
}
