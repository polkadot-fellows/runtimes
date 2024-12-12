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
use emulated_integration_tests_common::accounts::{ALICE, BOB};

use frame_support::{sp_runtime::traits::Dispatchable, traits::ProcessMessageError};
use kusama_runtime::governance::pallet_custom_origins::Origin::GeneralAdmin as GeneralAdminOrigin;
use people_kusama_runtime::people::IdentityInfo;

use pallet_identity::Data;

#[test]
fn relay_commands_add_registrar() {
	let origins = vec![
		(OriginKind::Xcm, GeneralAdminOrigin.into()),
		(OriginKind::Superuser, <Kusama as Chain>::RuntimeOrigin::root()),
	];
	for (origin_kind, origin) in origins {
		let registrar: AccountId = [1; 32].into();
		Kusama::execute_with(|| {
			type Runtime = <Kusama as Chain>::Runtime;
			type RuntimeCall = <Kusama as Chain>::RuntimeCall;
			type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
			type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
			type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

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
						// TODO:
						// This and the below weight data in the XCM can be removed once XCMv5 is
						// used.
						require_weight_at_most: Weight::from_parts(5_000_000_000, 500_000),
						call: add_registrar_call.encode().into(),
					}
				]))),
			});

			assert_ok!(xcm_message.dispatch(origin));

			assert_expected_events!(
				Kusama,
				vec![
					RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
				]
			);
		});

		PeopleKusama::execute_with(|| {
			type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

			assert_expected_events!(
				PeopleKusama,
				vec![
					RuntimeEvent::Identity(pallet_identity::Event::RegistrarAdded { .. }) => {},
					RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				]
			);
		});
	}
}

#[test]
fn relay_commands_add_registrar_wrong_origin() {
	let people_kusama_alice = PeopleKusama::account_id_of(ALICE);

	let (origin_kind, origin) = (
		OriginKind::SovereignAccount,
		<Kusama as Chain>::RuntimeOrigin::signed(people_kusama_alice),
	);

	let registrar: AccountId = [1; 32].into();
	Kusama::execute_with(|| {
		type Runtime = <Kusama as Chain>::Runtime;
		type RuntimeCall = <Kusama as Chain>::RuntimeCall;
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

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
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleKusama::execute_with(|| {
		type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleKusama,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::ProcessingFailed { error: ProcessMessageError::Unsupported, .. }) => {},
			]
		);
	});
}

#[test]
fn relay_commands_kill_identity() {
	// To kill an identity, first one must be set
	PeopleKusama::execute_with(|| {
		type PeopleRuntime = <PeopleKusama as Chain>::Runtime;
		type PeopleRuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

		let people_kusama_alice =
			<PeopleKusama as Chain>::RuntimeOrigin::signed(PeopleKusama::account_id_of(ALICE));

		let identity_info = IdentityInfo {
			email: Data::Raw(b"test@test.io".to_vec().try_into().unwrap()),
			..Default::default()
		};
		let identity: Box<<PeopleRuntime as pallet_identity::Config>::IdentityInformation> =
			Box::new(identity_info);

		assert_ok!(<PeopleKusama as PeopleKusamaPallet>::Identity::set_identity(
			people_kusama_alice,
			identity
		));

		assert_expected_events!(
			PeopleKusama,
			vec![
				PeopleRuntimeEvent::Identity(pallet_identity::Event::IdentitySet { .. }) => {},
			]
		);
	});

	let (origin_kind, origin) = (OriginKind::Superuser, <Kusama as Chain>::RuntimeOrigin::root());

	Kusama::execute_with(|| {
		type Runtime = <Kusama as Chain>::Runtime;
		type RuntimeCall = <Kusama as Chain>::RuntimeCall;
		type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

		let kill_identity_call =
			PeopleCall::Identity(pallet_identity::Call::<PeopleRuntime>::kill_identity {
				target: people_kusama_runtime::MultiAddress::Id(PeopleKusama::account_id_of(ALICE)),
			});

		let xcm_message = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind,
					// Making the weight's ref time any lower will prevent the XCM from triggering
					// execution of the intended extrinsic on the People chain - beware of spurious
					// test failure due to this.
					require_weight_at_most: Weight::from_parts(11_000_000_000, 500_000),
					call: kill_identity_call.encode().into(),
				}
			]))),
		});

		assert_ok!(xcm_message.dispatch(origin));

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleKusama::execute_with(|| {
		type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleKusama,
			vec![
				RuntimeEvent::Identity(pallet_identity::Event::IdentityKilled { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}

#[test]
fn relay_commands_kill_identity_wrong_origin() {
	let people_kusama_alice = PeopleKusama::account_id_of(BOB);

	let (origin_kind, origin) = (
		OriginKind::SovereignAccount,
		<Kusama as Chain>::RuntimeOrigin::signed(people_kusama_alice),
	);

	Kusama::execute_with(|| {
		type Runtime = <Kusama as Chain>::Runtime;
		type RuntimeCall = <Kusama as Chain>::RuntimeCall;
		type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

		let kill_identity_call =
			PeopleCall::Identity(pallet_identity::Call::<PeopleRuntime>::kill_identity {
				target: people_kusama_runtime::MultiAddress::Id(PeopleKusama::account_id_of(ALICE)),
			});

		let xcm_message = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind,
					require_weight_at_most: Weight::from_parts(11_000_000_000, 500_000),
					call: kill_identity_call.encode().into(),
				}
			]))),
		});

		assert_ok!(xcm_message.dispatch(origin));

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleKusama::execute_with(|| {
		type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleKusama,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::ProcessingFailed { error: ProcessMessageError::Unsupported, .. }) => {},
			]
		);
	});
}

#[test]
fn relay_commands_add_remove_username_authority() {
	let people_kusama_alice = PeopleKusama::account_id_of(ALICE);
	let people_kusama_bob = PeopleKusama::account_id_of(BOB);

	let origins = vec![
		(OriginKind::Xcm, GeneralAdminOrigin.into(), "generaladmin"),
		(OriginKind::Superuser, <Kusama as Chain>::RuntimeOrigin::root(), "rootusername"),
	];
	for (origin_kind, origin, usr) in origins {
		// First, add a username authority.
		Kusama::execute_with(|| {
			type Runtime = <Kusama as Chain>::Runtime;
			type RuntimeCall = <Kusama as Chain>::RuntimeCall;
			type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
			type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
			type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

			let add_username_authority = PeopleCall::Identity(pallet_identity::Call::<
				PeopleRuntime,
			>::add_username_authority {
				authority: people_kusama_runtime::MultiAddress::Id(people_kusama_alice.clone()),
				suffix: b"suffix1".into(),
				allocation: 10,
			});

			let add_authority_xcm_msg = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
				dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
				message: bx!(VersionedXcm::from(Xcm(vec![
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						origin_kind,
						require_weight_at_most: Weight::from_parts(500_000_000, 500_000),
						call: add_username_authority.encode().into(),
					}
				]))),
			});

			assert_ok!(add_authority_xcm_msg.dispatch(origin.clone()));

			assert_expected_events!(
				Kusama,
				vec![
					RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
				]
			);
		});

		// Check events system-parachain-side
		PeopleKusama::execute_with(|| {
			type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

			assert_expected_events!(
				PeopleKusama,
				vec![
					RuntimeEvent::Identity(pallet_identity::Event::AuthorityAdded { .. }) => {},
					RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				]
			);
		});

		// Now, use the previously added username authority to concede a username to an account.
		PeopleKusama::execute_with(|| {
			type PeopleRuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

			assert_ok!(<PeopleKusama as PeopleKusamaPallet>::Identity::set_username_for(
				<PeopleKusama as Chain>::RuntimeOrigin::signed(people_kusama_alice.clone()),
				people_kusama_runtime::MultiAddress::Id(people_kusama_bob.clone()),
				usr.to_owned().into_bytes(),
				None,
			));

			assert_expected_events!(
				PeopleKusama,
				vec![
					PeopleRuntimeEvent::Identity(pallet_identity::Event::UsernameQueued { .. }) => {},
				]
			);
		});

		// Accept the given username
		PeopleKusama::execute_with(|| {
			type PeopleRuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;
			let full_username = [usr.to_owned(), ".suffix1".to_owned()].concat().into_bytes();

			assert_ok!(<PeopleKusama as PeopleKusamaPallet>::Identity::accept_username(
				<PeopleKusama as Chain>::RuntimeOrigin::signed(people_kusama_bob.clone()),
				full_username.try_into().unwrap(),
			));

			assert_expected_events!(
				PeopleKusama,
				vec![
					PeopleRuntimeEvent::Identity(pallet_identity::Event::UsernameSet { .. }) => {},
				]
			);
		});

		// Now, remove the username authority with another privileged XCM call.
		Kusama::execute_with(|| {
			type Runtime = <Kusama as Chain>::Runtime;
			type RuntimeCall = <Kusama as Chain>::RuntimeCall;
			type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
			type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
			type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

			let remove_username_authority = PeopleCall::Identity(pallet_identity::Call::<
				PeopleRuntime,
			>::remove_username_authority {
				authority: people_kusama_runtime::MultiAddress::Id(people_kusama_alice.clone()),
			});

			let remove_authority_xcm_msg =
				RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
					dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
					message: bx!(VersionedXcm::from(Xcm(vec![
						UnpaidExecution { weight_limit: Unlimited, check_origin: None },
						Transact {
							origin_kind,
							// TODO:
							// this and all other references to `require_weight_at_most` can be
							// removed once XCMv5 is in use.
							require_weight_at_most: Weight::from_parts(500_000_000, 500_000),
							call: remove_username_authority.encode().into(),
						}
					]))),
				});

			assert_ok!(remove_authority_xcm_msg.dispatch(origin));

			assert_expected_events!(
				Kusama,
				vec![
					RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
				]
			);
		});

		// Final event check.
		PeopleKusama::execute_with(|| {
			type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

			assert_expected_events!(
				PeopleKusama,
				vec![
					RuntimeEvent::Identity(pallet_identity::Event::AuthorityRemoved { .. }) => {},
					RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				]
			);
		});
	}
}

#[test]
fn relay_commands_add_remove_username_authority_wrong_origin() {
	let people_kusama_alice = PeopleKusama::account_id_of(ALICE);

	let (origin_kind, origin) = (
		OriginKind::SovereignAccount,
		<Kusama as Chain>::RuntimeOrigin::signed(people_kusama_alice.clone()),
	);

	Kusama::execute_with(|| {
		type Runtime = <Kusama as Chain>::Runtime;
		type RuntimeCall = <Kusama as Chain>::RuntimeCall;
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

		let add_username_authority =
			PeopleCall::Identity(pallet_identity::Call::<PeopleRuntime>::add_username_authority {
				authority: people_kusama_runtime::MultiAddress::Id(people_kusama_alice.clone()),
				suffix: b"suffix1".into(),
				allocation: 10,
			});

		let add_authority_xcm_msg = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind,
					require_weight_at_most: Weight::from_parts(500_000_000, 500_000),
					call: add_username_authority.encode().into(),
				}
			]))),
		});

		assert_ok!(add_authority_xcm_msg.dispatch(origin));

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	// Check events system-parachain-side
	PeopleKusama::execute_with(|| {
		type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleKusama,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::ProcessingFailed { error: ProcessMessageError::Unsupported, .. }) => {},
			]
		);
	});

	// Since the origin check is the very first instruction in `remove_username_authority`, an
	// authority need not exist to test the safety of the origin check.
	Kusama::execute_with(|| {
		type Runtime = <Kusama as Chain>::Runtime;
		type RuntimeCall = <Kusama as Chain>::RuntimeCall;
		type RuntimeEvent = <Kusama as Chain>::RuntimeEvent;
		type PeopleCall = <PeopleKusama as Chain>::RuntimeCall;
		type PeopleRuntime = <PeopleKusama as Chain>::Runtime;

		let remove_username_authority = PeopleCall::Identity(pallet_identity::Call::<
			PeopleRuntime,
		>::remove_username_authority {
			authority: people_kusama_runtime::MultiAddress::Id(people_kusama_alice.clone()),
		});

		let remove_authority_xcm_msg = RuntimeCall::XcmPallet(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(Location::new(0, [Parachain(1004)]))),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::SovereignAccount,
					require_weight_at_most: Weight::from_parts(500_000_000, 500_000),
					call: remove_username_authority.encode().into(),
				}
			]))),
		});

		assert_ok!(remove_authority_xcm_msg
			.dispatch(<Kusama as Chain>::RuntimeOrigin::signed(people_kusama_alice)));

		assert_expected_events!(
			Kusama,
			vec![
				RuntimeEvent::XcmPallet(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	PeopleKusama::execute_with(|| {
		type RuntimeEvent = <PeopleKusama as Chain>::RuntimeEvent;

		assert_expected_events!(
			PeopleKusama,
			vec![
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::ProcessingFailed { error: ProcessMessageError::Unsupported, .. }) => {},
			]
		);
	});
}
