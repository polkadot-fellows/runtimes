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
use emulated_integration_tests_common::accounts::ALICE;

use frame_support::sp_runtime::traits::Dispatchable;
use polkadot_runtime::governance::pallet_custom_origins::Origin::GeneralAdmin as GeneralAdminOrigin;
use people_polkadot_runtime::people::IdentityInfo;

use pallet_identity::Data;

#[test]
fn relay_commands_add_registrar() {
	let origins = vec![
		(OriginKind::Xcm, GeneralAdminOrigin.into()),
		(OriginKind::Superuser, <Polkadot as Chain>::RuntimeOrigin::root()),
	];
	for (origin_kind, origin) in origins {
		let registrar: AccountId = [1; 32].into();
		Polkadot::execute_with(|| {
			type Runtime = <Polkadot as Chain>::Runtime;
			type RuntimeCall = <Polkadot as Chain>::RuntimeCall;
			type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
			type PeopleCall = <PeoplePolkadot as Chain>::RuntimeCall;
			type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;

			let add_registrar_call =
				PeopleCall::Identity(pallet_identity::Call::<PeopleRuntime>::add_registrar {
					account: registrar.into(),
				});

			let xcm_message = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
				dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
				message: bx!(VersionedXcm::from(Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind,
						require_weight_at_most: Weight::from_parts(5_000_000_000, 500_000),
						call: add_registrar_call.encode().into(),
					}
				]))),
			});

			assert_ok!(xcm_message.dispatch(origin));

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
}

#[test]
fn relay_commands_kill_identity() {

	// To kill an identity, first one must be set
	PeoplePolkadot::execute_with(|| {
		type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;
		type PeopleRuntimeEvent = <PeoplePolkadot as Chain>::RuntimeEvent;

		let people_polkadot_bob = <PeoplePolkadot as Chain>::RuntimeOrigin::signed(PeoplePolkadot::account_id_of(ALICE));

		let mut identity_info = <IdentityInfo as Default>::default();
		identity_info.email = Data::Raw(b"test@test.io".to_vec().try_into().unwrap());
		let identity: Box<<PeopleRuntime as pallet_identity::Config>::IdentityInformation> = Box::new(identity_info);

		assert_ok!(<PeoplePolkadot as PeoplePolkadotPallet>::Identity::set_identity(
				people_polkadot_bob,
				identity
		));

		//assert_ok!(add_identity_call.dispatch(people_polkadot_bob));

		assert_expected_events!(
			PeoplePolkadot,
			vec![
				PeopleRuntimeEvent::Identity(pallet_identity::Event::IdentitySet { .. }) => {},
			]
		);
	});

	let (origin_kind, origin) =
		(OriginKind::Superuser, <Polkadot as Chain>::RuntimeOrigin::root());

	Polkadot::execute_with(|| {
		type Runtime = <Polkadot as Chain>::Runtime;
		type RuntimeCall = <Polkadot as Chain>::RuntimeCall;
		type PeopleCall = <PeoplePolkadot as Chain>::RuntimeCall;
		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;
		type PeopleRuntime = <PeoplePolkadot as Chain>::Runtime;

		let kill_identity_call =
			PeopleCall::Identity(pallet_identity::Call::<PeopleRuntime>::kill_identity {
				target: people_polkadot_runtime::MultiAddress::Id(PeoplePolkadot::account_id_of(ALICE)),
			});

		let xcm_message = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind,
					require_weight_at_most: Weight::from_parts(5_000_000_000, 500_000),
					call: kill_identity_call.encode().into(),
				}
			]))),
		});

		assert_ok!(xcm_message.dispatch(origin));

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
				RuntimeEvent::Identity(pallet_identity::Event::IdentityKilled { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}
