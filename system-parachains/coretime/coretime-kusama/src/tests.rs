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

use crate::{
	coretime::{BrokerPalletId, CoretimeBurnAccount, PriceAdapter},
	*,
};
use frame_support::{
	assert_ok,
	traits::{
		fungible::{Inspect, Mutate},
		OnInitialize,
	},
};
use pallet_broker::{AdaptPrice, ConfigRecordOf, SaleInfo};
use parachains_runtimes_test_utils::ExtBuilder;
use sp_runtime::{
	traits::{AccountIdConversion, One},
	FixedU64,
};

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

fn advance_to(b: BlockNumber) {
	while System::block_number() < b {
		let block_number = System::block_number() + 1;
		System::set_block_number(block_number);
		Broker::on_initialize(block_number);
	}
}

#[test]
fn bulk_revenue_is_burnt() {
	const ALICE: [u8; 32] = [1u8; 32];

	ExtBuilder::<Runtime>::default()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::sr25519::Public::from_raw(ALICE)) },
		)])
		.build()
		.execute_with(|| {
			// Configure broker and start sales
			let config = ConfigRecordOf::<Runtime> {
				advance_notice: 1,
				interlude_length: 1,
				leadin_length: 2,
				region_length: 1,
				ideal_bulk_proportion: Perbill::from_percent(100),
				limit_cores_offered: None,
				renewal_bump: Perbill::from_percent(3),
				contribution_timeout: 1,
			};
			let ed = ExistentialDeposit::get();
			assert_ok!(Broker::configure(RuntimeOrigin::root(), config.clone()));
			assert_ok!(Broker::start_sales(RuntimeOrigin::root(), ed, 1));

			let sale_start = SaleInfo::<Runtime>::get().unwrap().sale_start;
			advance_to(sale_start + config.interlude_length);

			// Check and set initial balances.
			let broker_account = BrokerPalletId::get().into_account_truncating();
			let coretime_burn_account = CoretimeBurnAccount::get();
			let treasury_account = xcm_config::RelayTreasuryPalletAccount::get();
			assert_ok!(Balances::mint_into(&AccountId::from(ALICE), 10 * ed));
			let alice_balance_before = Balances::balance(&AccountId::from(ALICE));
			let treasury_balance_before = Balances::balance(&treasury_account);
			let broker_balance_before = Balances::balance(&broker_account);
			let burn_balance_before = Balances::balance(&coretime_burn_account);

			// Purchase coretime.
			assert_ok!(Broker::purchase(RuntimeOrigin::signed(AccountId::from(ALICE)), 5 * ed));

			// Alice decreases.
			assert!(Balances::balance(&AccountId::from(ALICE)) < alice_balance_before);
			// Treasury balance does not increase.
			assert_eq!(Balances::balance(&treasury_account), treasury_balance_before);
			// Broker pallet account does not increase.
			assert_eq!(Balances::balance(&broker_account), broker_balance_before);
			// Coretime burn pot gets the funds.
			assert!(Balances::balance(&coretime_burn_account) > burn_balance_before);

			// They're burnt at the end of the sale. TODO
			// advance_to(sale_start + timeslice_period * config.region_length + 1);
			// assert_eq!(Balances::balance(coretime_burn_account), 0);
		});
}
