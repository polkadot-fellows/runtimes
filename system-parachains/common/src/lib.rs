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

//! Shared types between system-parachains runtimes.
#![cfg_attr(not(feature = "std"), no_std)]

pub mod randomness;

/// Extra runtime APIs.
pub mod apis {
	/// Information about the current issuance rate of the system.
	///
	/// Both fields should be treated as best-effort, given that the issuance rate might not be
	/// fully predict-able.
	#[derive(scale_info::TypeInfo, codec::Encode, codec::Decode, Eq, PartialEq)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct InflationInfo {
		/// The rate of issuance estimated per annum, represented as a `Perquintill`.
		pub issuance: sp_runtime::Perquintill,
		/// Next amount that we anticipate to mint.
		///
		/// First item is the amount that goes to stakers, second is the leftover that is usually
		/// forwarded to the treasury.
		pub next_mint: (polkadot_primitives::Balance, polkadot_primitives::Balance),
	}

	sp_api::decl_runtime_apis! {
		pub trait Inflation {
			/// Return the current estimates of the issuance amount.
			///
			/// This is marked as experimental in light of RFC#89. Nonetheless, its usage is highly
			/// recommended over trying to read-storage, or re-create the onchain logic.
			fn experimental_issuance_prediction_info() -> InflationInfo;
		}
	}
}
