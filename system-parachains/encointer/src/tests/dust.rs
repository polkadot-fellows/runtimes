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

use crate::Runtime;
use parachains_runtimes_test_utils::ExtBuilder;

#[test]
fn dust_accumulates_instead_of_being_burned() {
	ExtBuilder::<Runtime>::default().build().execute_with(
		system_parachains_test_utils::dust_accumulates_instead_of_being_burned::<Runtime>,
	);
}

#[test]
fn forward_period_counts_relay_blocks() {
	ExtBuilder::<Runtime>::default()
		.build()
		.execute_with(system_parachains_test_utils::forward_period_counts_relay_blocks::<Runtime>);
}
