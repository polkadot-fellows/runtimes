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

//! Tests for coinage token allowance configuration.
//!
//! These tests verify that the configured token allowances provide the expected
//! minimum number of free unload tokens when the fee multiplier is at its base value (1).

use super::*;
use indiv_pallet_coinage::Config as CoinageConfig;

/// Test that verifies the minimum token distribution guarantee at base multiplier.
///
/// This test ensures that when the fee multiplier is at its base value (1), both people and
/// lite people get at least 10 free unload tokens.
/// The calculation is: min(allowance / fee, MaxFreeUnloadTokensPerTimePeriod) >= 10.
#[test]
fn people_get_at_least_10_free_tokens_at_base_multiplier() {
	new_test_ext().execute_with(|| {
		// The fee multiplier starts at 1, so this is the base fee
		// Ensure test starts from the expected base multiplier.
		let multiplier = TransactionPayment::next_fee_multiplier();
		assert_eq!(
			multiplier,
			FixedU128::from_u32(1),
			"Fee multiplier should start at 1 in this test setup"
		);

		// Get the fee at base multiplier (multiplier = 1)
		let fee = Coinage::get_paid_unload_token_fee_in_native();

		// Test people allowance
		let people_tokens = Coinage::free_unload_token_limit_for_people();
		assert!(
			people_tokens >= 10,
			"People should get at least 10 free unload tokens at base multiplier. \
			Got {people_tokens} tokens (fee={fee})"
		);

		// Test lite people allowance
		let lite_tokens = Coinage::free_unload_token_limit_for_lite_people();
		assert!(
			lite_tokens >= 10,
			"Lite people should get at least 10 free unload tokens at base multiplier. \
			Got {lite_tokens} tokens (fee={fee})"
		);
	});
}

/// Test that verifies the allowance values are correctly configured.
#[test]
fn allowance_values_are_correctly_configured() {
	new_test_ext().execute_with(|| {
		// People allowance should be 20 whole native tokens
		let people_allowance: u128 =
			<Runtime as CoinageConfig>::UnloadTokenAllowancePerTimePeriodForPeople::get();
		assert_eq!(people_allowance, 20 * UNITS, "People allowance should be 20 * UNITS");

		// Lite people allowance should be 10 whole native tokens
		let lite_people_allowance: u128 =
			<Runtime as CoinageConfig>::UnloadTokenAllowancePerTimePeriodForLitePeople::get();
		assert_eq!(lite_people_allowance, 10 * UNITS, "Lite people allowance should be 10 * UNITS");

		// Verify the ratio: people should have 2x the allowance of lite people
		assert_eq!(
			people_allowance / lite_people_allowance,
			2,
			"People allowance should be 2x lite people allowance"
		);
	});
}
