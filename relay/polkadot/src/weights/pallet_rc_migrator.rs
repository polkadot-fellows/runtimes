// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_rc_migrator`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_rc_migrator::WeightInfo for WeightInfo<T> {
	/// Storage: `System::Account` (r:2 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `RcMigrator::RcAccounts` (r:1 w:0)
	/// Proof: `RcMigrator::RcAccounts` (`max_values`: None, `max_size`: Some(57), added: 2532, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Freezes` (r:1 w:0)
	/// Proof: `Balances::Freezes` (`max_values`: None, `max_size`: Some(193), added: 2668, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Holds` (r:1 w:0)
	/// Proof: `Balances::Holds` (`max_values`: None, `max_size`: Some(103), added: 2578, mode: `MaxEncodedLen`)
	/// Storage: `Balances::Locks` (r:1 w:0)
	/// Proof: `Balances::Locks` (`max_values`: None, `max_size`: Some(1299), added: 3774, mode: `MaxEncodedLen`)
	/// Storage: `RcMigrator::RcMigratedBalance` (r:1 w:1)
	/// Proof: `RcMigrator::RcMigratedBalance` (`max_values`: Some(1), `max_size`: Some(32), added: 527, mode: `MaxEncodedLen`)
	fn withdraw_account() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `503`
		//  Estimated: `6196`
		// Minimum execution time: 51_000_000 picoseconds.
		Weight::from_parts(59_000_000, 6196)
			.saturating_add(T::DbWeight::get().reads(7_u64))
			.saturating_add(T::DbWeight::get().writes(2_u64))
	}
}
