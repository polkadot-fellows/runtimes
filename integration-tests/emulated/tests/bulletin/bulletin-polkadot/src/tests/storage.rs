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

//! The storage pallets' XCM surface: the People chain can authorize accounts, a sibling
//! outside the barrier's unpaid-execution list cannot get a message executed at all, the
//! Asset Hub cannot authorize as a plain `OriginKind::Xcm` origin, and calls that commit
//! data are blocked by the `SafeCallFilter`.

use crate::*;
use emulated_integration_tests_common::{
	accounts::BOB,
	impls::{assert_expected_events, bx, Encode},
	macros::{pallet_message_queue, pallet_xcm, Dispatchable},
};
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
		BulletinRuntimeCall::TransactionStorage(pallet_bulletin_transaction_storage::Call::<
			BulletinRuntime,
		>::authorize_account {
			who: who.clone(),
			transactions,
			bytes,
		})
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
					pallet_bulletin_transaction_storage::Event::AccountAuthorized {
						who: authorized,
						transactions: granted_transactions,
						bytes: granted_bytes,
					}
				) => {
					authorized: *authorized == who,
					granted_transactions: *granted_transactions == transactions,
					granted_bytes: *granted_bytes == bytes,
				},
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);

		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
		assert_eq!(
			pallet_bulletin_transaction_storage::Pallet::<BulletinRuntime>::account_authorization_extent(who.clone()),
			AuthorizationExtent {
				transactions_allowance: transactions,
				bytes_allowance: bytes,
				..Default::default()
			},
		);
		// The grant is usable, not just recorded.
		assert!(pallet_bulletin_transaction_storage::Pallet::<BulletinRuntime>::can_store(
			&who,
			bytes as u32,
		));
	});
}

/// A system parachain outside the barrier's unpaid-execution list gets no further than the
/// barrier, whatever it asks for. The message is delivered and then dropped as unprocessable.
#[test]
fn unlisted_sibling_is_rejected_by_the_barrier() {
	let who: AccountId = PeoplePolkadot::account_id_of(BOB);

	let authorize_call = {
		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
		type BulletinRuntimeCall = <BulletinPolkadot as Chain>::RuntimeCall;
		BulletinRuntimeCall::TransactionStorage(pallet_bulletin_transaction_storage::Call::<
			BulletinRuntime,
		>::authorize_account {
			who: who.clone(),
			transactions: 5,
			bytes: 512 * 1024,
		})
		.encode()
	};

	BridgeHubPolkadot::execute_with(|| {
		type Runtime = <BridgeHubPolkadot as Chain>::Runtime;
		type RuntimeCall = <BridgeHubPolkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <BridgeHubPolkadot as Chain>::RuntimeEvent;

		let send_xcm = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(BridgeHubPolkadot::sibling_location_of(
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

		assert_ok!(send_xcm.dispatch(<BridgeHubPolkadot as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			BridgeHubPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BulletinPolkadot::execute_with(|| {
		type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;
		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;

		// A barrier rejection stops execution at instruction 0, which the queue reports as an
		// unsuccessful message rather than a processing failure.
		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: false, .. }
				) => {},
			]
		);

		assert_eq!(
			pallet_bulletin_transaction_storage::Pallet::<BulletinRuntime>::account_authorization_extent(who),
			AuthorizationExtent::default(),
		);
	});
}

/// The Asset Hub clears the barrier, so what stops it from authorizing is the `Authorizer`
/// config: only Root — reached with `OriginKind::Superuser` — or the People chain's XCM origin
/// qualifies. As a plain `OriginKind::Xcm` origin the Asset Hub is neither.
///
/// `Transact` reports an inner dispatch failure through the transact-status register instead of
/// failing the message, so `ExpectTransactStatus` is what turns the rejected dispatch into an
/// unsuccessful message.
#[test]
fn asset_hub_cannot_authorize_as_a_plain_xcm_origin() {
	let who: AccountId = AssetHubPolkadot::account_id_of(BOB);

	let authorize_call = {
		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
		type BulletinRuntimeCall = <BulletinPolkadot as Chain>::RuntimeCall;
		BulletinRuntimeCall::TransactionStorage(pallet_bulletin_transaction_storage::Call::<
			BulletinRuntime,
		>::authorize_account {
			who: who.clone(),
			transactions: 5,
			bytes: 512 * 1024,
		})
		.encode()
	};

	AssetHubPolkadot::execute_with(|| {
		type Runtime = <AssetHubPolkadot as Chain>::Runtime;
		type RuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		let send_xcm = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(AssetHubPolkadot::sibling_location_of(
				BulletinPolkadot::para_id()
			))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: authorize_call.into(),
				},
				ExpectTransactStatus(MaybeErrorCode::Success),
			]))),
		});

		assert_ok!(send_xcm.dispatch(<AssetHubPolkadot as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BulletinPolkadot::execute_with(|| {
		type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;
		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;

		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: false, .. }
				) => {},
			]
		);

		assert_eq!(
			pallet_bulletin_transaction_storage::Pallet::<BulletinRuntime>::account_authorization_extent(who),
			AuthorizationExtent::default(),
		);
	});
}

/// Calls that commit data are unreachable over XCM. The Asset Hub is a superuser on the
/// Bulletin chain, so `SafeCallFilter` is the only thing standing between it and a free
/// `store`.
#[test]
fn store_over_xcm_is_blocked() {
	let store_call = {
		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
		type BulletinRuntimeCall = <BulletinPolkadot as Chain>::RuntimeCall;
		BulletinRuntimeCall::TransactionStorage(pallet_bulletin_transaction_storage::Call::<
			BulletinRuntime,
		>::store {
			data: vec![42u8; 100],
		})
		.encode()
	};

	AssetHubPolkadot::execute_with(|| {
		type Runtime = <AssetHubPolkadot as Chain>::Runtime;
		type RuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <AssetHubPolkadot as Chain>::RuntimeEvent;

		let send_xcm = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(AssetHubPolkadot::sibling_location_of(
				BulletinPolkadot::para_id()
			))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Superuser,
					fallback_max_weight: None,
					call: store_call.into(),
				},
			]))),
		});

		assert_ok!(send_xcm.dispatch(<AssetHubPolkadot as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			AssetHubPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BulletinPolkadot::execute_with(|| {
		type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			BulletinPolkadot,
			vec![
				RuntimeEvent::MessageQueue(
					pallet_message_queue::Event::Processed { success: false, .. }
				) => {},
			]
		);
	});
}
