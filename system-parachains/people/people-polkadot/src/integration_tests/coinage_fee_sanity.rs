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

//! Sanity tests for coinage fee calculations.

use super::*;

#[test]
fn paid_unload_token_fee_in_native_is_reasonable() {
	new_test_ext().execute_with(|| {
		let fee: Balance = Coinage::get_paid_unload_token_fee_in_native();

		// Print the actual value for debugging
		println!("paid_unload_token_fee_in_native = {fee}");

		// The fee should be within reasonable bounds.
		// We allow a range of value/2 to value*2 to tolerate weight changes.
		//
		// NOTE: this is much higher than the equivalent Paseo value (~0.0033 PAS): the People
		// Polkadot runtime uses its real benchmarked weights for the ring-VRF-verifying unload
		// calls, which dominate this fee.
		const EXPECTED: Balance = 1_560_000_000; // ~0.156 DOT
		let lower_bound = EXPECTED / 2;
		let upper_bound = EXPECTED * 2;

		assert!(fee >= lower_bound, "Fee {fee} is below minimum expected {lower_bound}");
		assert!(fee <= upper_bound, "Fee {fee} is above maximum expected {upper_bound}");
	});
}
