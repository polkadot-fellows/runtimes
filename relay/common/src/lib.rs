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

//! Shared code between the Kusama nd Polkadot RC Runtimes.
#![cfg_attr(not(feature = "std"), no_std)]

use polkadot_primitives::Balance;
use sp_runtime::{Perquintill, Saturating};

/// Extra runtime APIs for kusama runtime.
pub mod apis {
	/// Information about the current inflation rate of the system.
	///
	/// Both fields should be treated as best-effort, given that the inflation rate might not be
	/// fully predict-able.
	#[derive(scale_info::TypeInfo, codec::Encode, codec::Decode)]
	#[cfg_attr(feature = "std", derive(Debug))]
	pub struct InflationInfo {
		/// The rate of inflation estimated per annum.
		pub inflation: sp_runtime::Perquintill,
		/// Next amount that we anticipate to mint.
		///
		/// First item is the amount that goes to stakers, second is the leftover that is usually
		/// forwarded to the treasury.
		pub next_mint: (polkadot_primitives::Balance, polkadot_primitives::Balance),
	}

	sp_api::decl_runtime_apis! {
		pub trait Inflation {
			/// Return the current estimates of the inflation amount.
			///
			/// This is marked as experimental in light of RFC#89. Nonetheless, its usage is highly
			/// recommended over trying to read-storage, or re-create the onchain logic.
			fn experimental_inflation_prediction_info() -> InflationInfo;
		}
	}
}

// ---- TODO: Below is copy pasted from sdk, remove once we pull the version containing
// https://github.com/paritytech/polkadot-sdk/pull/4938

#[derive(Debug, Clone)]
/// Parameters passed into [`relay_era_payout`] function.
pub struct EraPayoutParams {
	/// Total staked amount.
	pub total_staked: Balance,
	/// Total stakable amount.
	///
	/// Usually, this is equal to the total issuance, except if a large part of the issuance is
	/// locked in another sub-system.
	pub total_stakable: Balance,
	/// Ideal stake ratio, which is reduced by `legacy_auction_proportion` if not `None`.
	pub ideal_stake: Perquintill,
	/// Maximum inflation rate.
	pub max_annual_inflation: Perquintill,
	/// Minimum inflation rate.
	pub min_annual_inflation: Perquintill,
	/// Falloff used to calculate era payouts.
	pub falloff: Perquintill,
	/// Fraction of the era period used to calculate era payouts.
	pub period_fraction: Perquintill,
	/// Legacy auction proportion, which, if not `None`, is subtracted from `ideal_stake`.
	pub legacy_auction_proportion: Option<Perquintill>,
}

/// A specialized function to compute the inflation of the staking system, tailored for Polkadot
/// Relay Chains, such as Polkadot, Kusama, and Westend.
pub fn relay_era_payout(params: EraPayoutParams) -> (Balance, Balance) {
	let EraPayoutParams {
		total_staked,
		total_stakable,
		ideal_stake,
		max_annual_inflation,
		min_annual_inflation,
		falloff,
		period_fraction,
		legacy_auction_proportion,
	} = params;

	let delta_annual_inflation = max_annual_inflation.saturating_sub(min_annual_inflation);

	let ideal_stake = ideal_stake.saturating_sub(legacy_auction_proportion.unwrap_or_default());

	let stake = Perquintill::from_rational(total_staked, total_stakable);
	let adjustment = pallet_staking_reward_fn::compute_inflation(stake, ideal_stake, falloff);
	let staking_inflation =
		min_annual_inflation.saturating_add(delta_annual_inflation * adjustment);

	let max_payout = period_fraction * max_annual_inflation * total_stakable;
	let staking_payout = (period_fraction * staking_inflation) * total_stakable;
	let rest = max_payout.saturating_sub(staking_payout);

	let other_issuance = total_stakable.saturating_sub(total_staked);
	if total_staked > other_issuance {
		let _cap_rest = Perquintill::from_rational(other_issuance, total_staked) * staking_payout;
		// We don't do anything with this, but if we wanted to, we could introduce a cap on the
		// treasury amount with: `rest = rest.min(cap_rest);`
	}
	(staking_payout, rest)
}

// ---- TODO: Above is copy pasted from sdk, remove once we pull the version containing
// https://github.com/paritytech/polkadot-sdk/pull/4938
