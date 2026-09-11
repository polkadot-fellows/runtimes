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

//! The People-to-Bulletin long-term-storage grant.

use crate::*;
use emulated_integration_tests_common::{
	impls::assert_expected_events, macros::pallet_message_queue,
};
use indiv_pallet_resources::{
	types::MembershipCollection, Origin as ResourcesOrigin, SpentLongTermStorageAliases,
};
use indiv_support::utils::BigEndianU32;

/// A long-term-storage claim on People authorizes the same account on Bulletin.
///
/// People authorizes the allocation locally and sends an XCM `Transact` to Bulletin's
/// `pallet-transaction-storage`. Bulletin's XCM barrier grants the People chain free execution for
/// Proof-of-Personhood authorizations, so the `Transact` dispatches `authorize_account` and records
/// the account authorization.
///
/// Asserts People recorded the claim (`SpentLongTermStorageAliases`, `LongTermStorageClaimed`) and
/// Bulletin authorized the account (`TransactionStorage::AccountAuthorized`).
#[test]
fn people_long_term_storage_grant_is_applied_on_bulletin() {
	let who: AccountId = [42; 32].into();

	PeoplePolkadot::execute_with(|| {
		type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;

		let alias = [7; 32];
		assert_ok!(people_polkadot_runtime::Resources::claim_long_term_storage(
			people_polkadot_runtime::RuntimeOrigin::from(
				people_polkadot_runtime::OriginCaller::Resources(
					ResourcesOrigin::LongTermStorageClaim(alias, MembershipCollection::People),
				),
			),
			0,
			0,
			who.clone(),
		));
		assert!(SpentLongTermStorageAliases::<people_polkadot_runtime::Runtime>::contains_key(
			BigEndianU32::from(0),
			alias,
		));
		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::Resources(
					indiv_pallet_resources::Event::LongTermStorageClaimed { account, .. }
				) => {
					account: *account == who,
				},
			]
		);
	});

	BulletinPolkadot::execute_with(|| {
		type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::TransactionStorage(
					pallet_bulletin_transaction_storage::Event::AccountAuthorized {
						who: authorized,
						..
					}
				) => {
					authorized: *authorized == who,
				},
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);
	});
}
