// Copyright (C) Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

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
