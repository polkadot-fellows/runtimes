// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Total-issuance correction: burns the audited issuance that no account holds.
//!
//! The relay chain's total issuance exceeds the sum of its account balances by a known amount
//! ("phantom issuance"), measured off-chain and pinned in `Config::TiCorrection`. By this stage
//! the accounts and sweep stages have drained every account to zero except what the caller says
//! is still legitimately held, so whatever issuance the ledger still counts beyond that is held
//! by nobody — no O(accounts) scan is needed. The stage burns `min(expected, measured)`: a
//! remainder above the expectation stays on the books for investigation, a measurement below it
//! is reported as an anomaly. It never burns issuance that an account actually holds.
//!
//! Runs after the sweep, which reactivates inactive issuance and reaps dust — both move total
//! issuance — and before the cool-off.
//!
//! Driving the stage from `on_initialize` and signalling the Coretime chain afterwards is the
//! stage machine's job: [`TiCorrector::correct_total_issuance`] runs once, inside a storage
//! transaction the caller owns, and books what it burned into the conservation ledger.

use crate::{Config, Event, Pallet, RcMigratedBalance};
use core::marker::PhantomData;
use frame_support::traits::Get;

const LOG_TARGET: &str = "runtime::rc2-migrator";

/// Why the correction could not complete. The caller rolls it back.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// The migrated/kept balance bookkeeping would overflow.
	BalanceAccounting,
}

/// What one run of the correction measured and did.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Correction {
	/// The audited phantom, `Config::TiCorrection`.
	pub expected: u128,
	/// Total issuance beyond what is still legitimately held.
	pub unaccounted: u128,
	/// `min(expected, unaccounted)`, written off the total issuance.
	pub burned: u128,
}

pub struct TiCorrector<T>(PhantomData<T>);

impl<T: Config> TiCorrector<T> {
	/// Burn the audited phantom issuance.
	///
	/// `still_held` is the balance that legitimately remains on this chain at this point — the
	/// manager's, which stays funded through `CoolOff` — and is not counted as phantom.
	// TODO(ahm-v2): the stage machine passes the manager's total balance here; `Manager` lands
	// with #1285.
	pub fn correct_total_issuance(still_held: u128) -> Result<Correction, Error> {
		let expected = T::TiCorrection::get();
		let total = pallet_balances::TotalIssuance::<T>::get();
		let unaccounted = total.saturating_sub(still_held);
		let burned = expected.min(unaccounted);

		if unaccounted < expected {
			log::error!(
				target: LOG_TARGET,
				"TI correction anomaly: expected {expected} unaccounted, measured {unaccounted}"
			);
			Pallet::<T>::deposit_event(Event::TiCorrectionAnomaly { expected, unaccounted });
		}

		// No account holds this balance, so there is nothing to burn *from*: the correction is
		// a direct issuance write, mirrored in the migration tracker so the conservation
		// invariant stays exact.
		pallet_balances::TotalIssuance::<T>::put(total.saturating_sub(burned));
		RcMigratedBalance::<T>::try_mutate(|t| {
			t.kept = t.kept.checked_sub(burned).ok_or(Error::BalanceAccounting)?;
			t.ti_corrected = t.ti_corrected.checked_add(burned).ok_or(Error::BalanceAccounting)?;
			Ok::<(), Error>(())
		})?;
		Pallet::<T>::deposit_event(Event::TiCorrected { expected, unaccounted, burned });

		Ok(Correction { expected, unaccounted, burned })
	}
}
