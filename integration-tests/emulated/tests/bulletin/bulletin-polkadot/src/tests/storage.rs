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

//! Tests for storage authorization via XCM from the People chain.

use crate::*;
use emulated_integration_tests_common::{
	impls::{assert_expected_events, bx, Encode},
	macros::{pallet_message_queue, pallet_xcm, Dispatchable},
};
use emulated_integration_tests_common::accounts::BOB;
use pallet_bulletin_transaction_storage::AuthorizationExtent;

/// The People chain authorizes an account on the Bulletin chain via XCM Transact.
///
/// `EnsureXcm<Equals<PeopleLocation>>` accepts the sibling origin when `OriginKind::Xcm` is used,
/// so the call should succeed and the account's authorization extent should be updated.
#[test]
fn people_chain_can_authorize_account_on_bulletin() {
	let who: AccountId = PeoplePolkadot::account_id_of(BOB);
	let transactions: u32 = 5;
	let bytes: u64 = 512 * 1024;

	// Encode the `authorize_account` call for the Bulletin runtime.
	let authorize_call = {
		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
		type BulletinRuntimeCall = <BulletinPolkadot as Chain>::RuntimeCall;
		BulletinRuntimeCall::TransactionStorage(
			pallet_bulletin_transaction_storage::Call::<BulletinRuntime>::authorize_account {
				who: who.clone(),
				transactions,
				bytes,
			},
		)
		.encode()
	};

	// Have the People chain send the XCM to the Bulletin chain.
	PeoplePolkadot::execute_with(|| {
		type Runtime = <PeoplePolkadot as Chain>::Runtime;
		type RuntimeCall = <PeoplePolkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;

		let send_xcm = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(PeoplePolkadot::sibling_location_of(
				BulletinPolkadot::para_id()
			))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: authorize_call.into(),
				},
			]))),
		});

		assert_ok!(send_xcm.dispatch(<PeoplePolkadot as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// Verify the Bulletin chain processed the message and created the authorization.
	BulletinPolkadot::execute_with(|| {
		type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::TransactionStorage(
					pallet_bulletin_transaction_storage::Event::AccountAuthorized { .. }
				) => {},
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);

		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
		assert_eq!(
			pallet_bulletin_transaction_storage::Pallet::<BulletinRuntime>::account_authorization_extent(who),
			AuthorizationExtent { transactions, bytes },
		);
	});
}
