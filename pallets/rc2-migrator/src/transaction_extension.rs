// Copyright (C) Polkadot Fellows.
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

//! Refuses signed transactions once the migration starts, before they pay a fee.

use crate::{Call, Config, Pallet, RcMigrationStage};
use codec::{Decode, DecodeWithMemTracking, Encode};
use core::marker::PhantomData;
use frame_support::{
	pallet_prelude::{TransactionSource, Weight},
	traits::{Get, IsSubType, OriginTrait},
	DefaultNoBound,
};
use scale_info::TypeInfo;
use sp_runtime::{
	impl_tx_ext_default,
	traits::{DispatchInfoOf, Implication, TransactionExtension, ValidateResult},
	transaction_validity::InvalidTransaction,
};

/// Refuses every signed transaction once the migration has started, so none pays a fee. Only a
/// migrator call from the manager or the admin origin passes. Unsigned transactions and inherents
/// are left alone.
///
/// The call filter refuses a call only when it runs, after its fee is paid, and a fee credited to
/// an account the accounts stage has already passed stays on this chain.
#[derive(Encode, Decode, DecodeWithMemTracking, DefaultNoBound, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct DenySignedOnceMigrationStarts<T>(PhantomData<T>);

impl<T> DenySignedOnceMigrationStarts<T> {
	pub fn new() -> Self {
		Self(PhantomData)
	}
}

impl<T> core::fmt::Debug for DenySignedOnceMigrationStarts<T> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		write!(f, "DenySignedOnceMigrationStarts")
	}
}

type RuntimeCallOf<T> = <T as frame_system::Config>::RuntimeCall;

impl<T> TransactionExtension<RuntimeCallOf<T>> for DenySignedOnceMigrationStarts<T>
where
	T: Config + Send + Sync,
	RuntimeCallOf<T>: IsSubType<Call<T>>,
{
	const IDENTIFIER: &'static str = "DenySignedOnceMigrationStarts";
	type Implicit = ();
	type Val = ();
	type Pre = ();

	fn weight(&self, _: &RuntimeCallOf<T>) -> Weight {
		// the stage, and the manager for a migrator call
		T::DbWeight::get().reads(2)
	}

	fn validate(
		&self,
		origin: <T as frame_system::Config>::RuntimeOrigin,
		call: &RuntimeCallOf<T>,
		_info: &DispatchInfoOf<RuntimeCallOf<T>>,
		_len: usize,
		_self_implicit: Self::Implicit,
		_inherited_implication: &impl Implication,
		_source: TransactionSource,
	) -> ValidateResult<Self::Val, RuntimeCallOf<T>> {
		if origin.as_signer().is_some() &&
			RcMigrationStage::<T>::get().has_started() &&
			!(call.is_sub_type().is_some() &&
				Pallet::<T>::ensure_admin_or_manager(origin.clone()).is_ok())
		{
			return Err(InvalidTransaction::Call.into());
		}
		Ok((Default::default(), (), origin))
	}

	impl_tx_ext_default!(RuntimeCallOf<T>; prepare);
}
