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

use crate::coretime::PriceAdapter;
use pallet_broker::AdaptPrice;
use sp_runtime::{traits::One, FixedU64};

#[test]
fn adapt_price_no_panic() {
	for limit in 0..10 {
		for target in 1..10 {
			for sold in 0..=limit {
				let price = PriceAdapter::adapt_price(sold, target, limit);

				if sold > target {
					assert!(price > FixedU64::one());
				} else {
					assert!(price <= FixedU64::one());
				}
			}
		}
	}
}

#[test]
fn adapt_price_bound_check() {
	// Using assumptions from pallet implementation i.e. `limit >= sold`.
	// Check extremes
	let limit = 10;
	let target = 5;

	// Maximally sold: `sold == limit`
	assert_eq!(PriceAdapter::adapt_price(limit, target, limit), FixedU64::from_float(1.2));
	// Ideally sold: `sold == target`
	assert_eq!(PriceAdapter::adapt_price(target, target, limit), FixedU64::one());
	// Minimally sold: `sold == 0`
	assert_eq!(PriceAdapter::adapt_price(0, target, limit), FixedU64::from_float(0.5));
	// Optimistic target: `target == limit`
	assert_eq!(PriceAdapter::adapt_price(limit, limit, limit), FixedU64::one());
	// Pessimistic target: `target == 0`
	assert_eq!(PriceAdapter::adapt_price(limit, 0, limit), FixedU64::from_float(1.2));
}

#[test]
fn leadin_price_bound_check() {
	// Using assumptions from pallet implementation i.e. `when` in range [0, 1].
	// Linear, therefore we need three points to fully test it

	// Start of leadin: 5x
	assert_eq!(PriceAdapter::leadin_factor_at(FixedU64::from(0)), FixedU64::from(5));
	// Midway through leadin: 3x
	assert_eq!(PriceAdapter::leadin_factor_at(FixedU64::from_float(0.5)), FixedU64::from(3));
	// End of leadin: 1x
	assert_eq!(PriceAdapter::leadin_factor_at(FixedU64::one()), FixedU64::one());
}
