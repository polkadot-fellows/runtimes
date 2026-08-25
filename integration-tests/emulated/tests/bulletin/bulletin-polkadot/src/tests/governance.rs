// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Governance reaching the Bulletin chain over XCM.
//!
//! `LocationAsSuperuser` grants Root to exactly two locations — the relay chain and the Asset
//! Hub — sending `OriginKind::Superuser`. That is how the chain gets its first storage
//! authorizers and how its storage-backed limits are retuned, so both paths are exercised
//! end to end here.

use crate::*;
use emulated_integration_tests_common::{
	accounts::BOB,
	impls::{assert_expected_events, bx, Encode, RelayChain},
	macros::{pallet_message_queue, pallet_xcm, Dispatchable},
};
use pallet_bulletin_transaction_storage::AuthorizationExtent;
use polkadot_system_emulated_network::polkadot_emulated_chain::polkadot_runtime::Dmp;

const TRANSACTIONS: u32 = 7;
const BYTES: u64 = 256 * 1024;

/// SCALE-encoded `TransactionStorage::authorize_account` for `who`, as the sender has to
/// build it.
fn authorize_account_call(who: &AccountId) -> Vec<u8> {
	type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
	type BulletinRuntimeCall = <BulletinPolkadot as Chain>::RuntimeCall;

	BulletinRuntimeCall::TransactionStorage(pallet_bulletin_transaction_storage::Call::<
		BulletinRuntime,
	>::authorize_account {
		who: who.clone(),
		transactions: TRANSACTIONS,
		bytes: BYTES,
	})
	.encode()
}

/// The granted allowance landed on the Bulletin chain and the message was processed cleanly.
fn assert_authorized(who: AccountId) {
	type RuntimeEvent = <BulletinPolkadot as Chain>::RuntimeEvent;
	type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;

	assert_expected_events!(
		BulletinPolkadot,
		vec![
			RuntimeEvent::TransactionStorage(
				pallet_bulletin_transaction_storage::Event::AccountAuthorized {
					who: authorized,
					transactions,
					bytes,
				}
			) => {
				authorized: *authorized == who,
				transactions: *transactions == TRANSACTIONS,
				bytes: *bytes == BYTES,
			},
			RuntimeEvent::MessageQueue(
				pallet_message_queue::Event::Processed { success: true, .. }
			) => {},
		]
	);

	assert_eq!(
		pallet_bulletin_transaction_storage::Pallet::<BulletinRuntime>::account_authorization_extent(who),
		AuthorizationExtent {
			transactions_allowance: TRANSACTIONS,
			bytes_allowance: BYTES,
			..Default::default()
		},
	);
}

#[test]
fn asset_hub_root_can_authorize_account() {
	let who: AccountId = AssetHubPolkadot::account_id_of(BOB);
	let call = authorize_account_call(&who);

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
					call: call.into(),
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

	BulletinPolkadot::execute_with(|| assert_authorized(who));
}

#[test]
fn relay_root_can_authorize_account() {
	let who: AccountId = AssetHubPolkadot::account_id_of(BOB);
	let call = authorize_account_call(&who);

	Polkadot::execute_with(|| {
		type Runtime = <Polkadot as Chain>::Runtime;
		type RuntimeCall = <Polkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;

		Dmp::make_parachain_reachable(BulletinPolkadot::para_id());

		let send_xcm = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Polkadot::child_location_of(
				BulletinPolkadot::para_id()
			))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Superuser,
					fallback_max_weight: None,
					call: call.into(),
				},
			]))),
		});

		assert_ok!(send_xcm.dispatch(<Polkadot as Chain>::RuntimeOrigin::root()));

		assert_expected_events!(
			Polkadot,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	BulletinPolkadot::execute_with(|| assert_authorized(who));
}

/// `MaxPermanentStorageSize` is a storage-backed parameter rather than a runtime constant so
/// that the chain-wide permanent-storage cap can be retuned by governance without a runtime
/// upgrade. That only holds if governance can actually reach `System::set_storage` here.
#[test]
fn asset_hub_root_can_retune_max_permanent_storage_size() {
	use bulletin_polkadot_runtime::storage::MaxPermanentStorageSize;

	// Half the seeded 1.7 TiB.
	const NEW_CAP: u64 = 17 * 1024 * 1024 * 1024 * 1024 / 20;

	BulletinPolkadot::execute_with(|| {
		assert_ne!(MaxPermanentStorageSize::get(), NEW_CAP);
	});

	let set_storage_call = {
		type BulletinRuntime = <BulletinPolkadot as Chain>::Runtime;
		type BulletinRuntimeCall = <BulletinPolkadot as Chain>::RuntimeCall;

		BulletinRuntimeCall::System(frame_system::Call::<BulletinRuntime>::set_storage {
			items: vec![(MaxPermanentStorageSize::key().to_vec(), NEW_CAP.encode())],
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
					call: set_storage_call.into(),
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
					pallet_message_queue::Event::Processed { success: true, .. }
				) => {},
			]
		);

		assert_eq!(MaxPermanentStorageSize::get(), NEW_CAP);
	});
}
