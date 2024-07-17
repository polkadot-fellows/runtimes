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
use kusama_runtime::governance::pallet_custom_origins::Origin::GeneralAdmin as GeneralAdminOrigin;

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
				dest: bx!(VersionedLocation::V4(Location::new(0, [Parachain(1004)]))),
				message: bx!(VersionedXcm::V4(Xcm(vec![
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
					RuntimeEvent::Identity(pallet_identity::Event::RegistrarAdded { .. }) => {},
					RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
				]
			);
		});
	}
}
