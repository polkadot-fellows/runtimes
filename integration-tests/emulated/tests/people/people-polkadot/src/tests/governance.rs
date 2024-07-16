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

use crate::*;
use frame_support::sp_runtime::traits::Dispatchable;
use polkadot_runtime::governance::pallet_custom_origins::Origin::GeneralAdmin as GeneralAdminOrigin;

#[test]
fn general_admin_add_registrar() {
	let registrar: AccountId = [1; 32].into();
	Polkadot::execute_with(|| {
		type Runtime = <Polkadot as Chain>::Runtime;
		type RuntimeCall = <Polkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
		type RuntimeOrigin = <Polkadot as Chain>::RuntimeOrigin;
		type PeopleCall = <PeoplePolkadot as Chain>::RuntimeCall;
		type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;

		let add_registrar_call =
			PeopleCall::Identity(pallet_identity::Call::<PeopleRuntime>::add_registrar {
				account: registrar.into(),
			});

		let xcm_message = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::V4(Location::new(0, [Parachain(1004)]))),
			message: bx!(VersionedXcm::V4(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					require_weight_at_most: Weight::from_parts(5_000_000_000, 500_000),
					call: add_registrar_call.encode().into(),
				}
			]))),
		});

		let general_admin: RuntimeOrigin = GeneralAdminOrigin.into();

		assert_ok!(xcm_message.dispatch(general_admin));

		assert_expected_events!(
			Polkadot,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeoplePolkadot::execute_with(|| {
		type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::Identity(pallet_identity::Event::RegistrarAdded { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn relay_root_add_registrar() {
	let registrar: AccountId = [1; 32].into();
	Polkadot::execute_with(|| {
		type Runtime = <Polkadot as Chain>::Runtime;
		type RuntimeCall = <Polkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
		type RuntimeOrigin = <Polkadot as Chain>::RuntimeOrigin;
		type PeopleCall = <PeoplePolkadot as Chain>::RuntimeCall;
		type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;

		let add_registrar_call =
			PeopleCall::Identity(pallet_identity::Call::<PeopleRuntime>::add_registrar {
				account: registrar.into(),
			});

		let xcm_message = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::V4(Location::new(0, [Parachain(1004)]))),
			message: bx!(VersionedXcm::V4(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Superuser,
					require_weight_at_most: Weight::from_parts(5_000_000_000, 500_000),
					call: add_registrar_call.encode().into(),
				}
			]))),
		});

		let root: RuntimeOrigin = RuntimeOrigin::root();

		assert_ok!(xcm_message.dispatch(root));

		assert_expected_events!(
			Polkadot,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeoplePolkadot::execute_with(|| {
		type RuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeoplePolkadot,
			vec![
				RuntimeEvent::Identity(pallet_identity::Event::RegistrarAdded { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}
