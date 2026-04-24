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

//! Shim for `pallet-dap` items not yet published on crates.io.
//!
//! ---- TODO: Dev scaffolding for the py/trsry wind-down PR. The PR is NOT merged (and this code
//! NOT shipped to chains) until we bump pallet-dap to SDK 2604 release.
//! At bump time, delete this module and re-export from `pallet_dap` directly
//! (`pallet_dap::DapLegacyAdapter`, SDK PR #11716).
use core::marker::PhantomData;
use frame_support::{
	traits::{Currency, OnUnbalanced},
	PalletId,
};
use sp_runtime::traits::AccountIdConversion;

const LOG_TARGET: &str = "runtime::dap-shim";

// TODO: temporary hardcoded derivation of the DAP buffer account. The seed bytes mirror
// `sp_dap::DAP_PALLET_ID` in SDK master. Like the whole shim, it will be discarded once we bump
// SDK with 2604 crates.
const DAP_PALLET_ID: PalletId = PalletId(*b"dap/buff");

/// Adapter that redirects `NegativeImbalance` from the legacy `Currency` trait to the local
/// `pallet_dap` buffer.
///
/// Mirrors upstream `pallet_dap::DapLegacyAdapter`. Use on pallets that still expose
/// `type Slash/Slashed: OnUnbalanced<NegativeImbalance>` (e.g. `pallet_referenda`).
pub struct DapLegacyAdapter<T, C>(PhantomData<(T, C)>);

impl<T, C> OnUnbalanced<<C as Currency<T::AccountId>>::NegativeImbalance> for DapLegacyAdapter<T, C>
where
	T: frame_system::Config,
	C: Currency<T::AccountId>,
{
	fn on_nonzero_unbalanced(amount: <C as Currency<T::AccountId>>::NegativeImbalance) {
		let buffer: T::AccountId = DAP_PALLET_ID.into_account_truncating();
		C::resolve_creating(&buffer, amount);
		log::debug!(
			target: LOG_TARGET,
			"💸 Deposited (legacy) to DAP buffer"
		);
	}
}
