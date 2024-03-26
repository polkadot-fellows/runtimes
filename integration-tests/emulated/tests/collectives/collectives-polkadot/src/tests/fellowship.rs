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
use codec::Encode;
use collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::Fellows as FellowsOrigin;
use frame_support::{assert_ok, sp_runtime::traits::Dispatchable};

#[test]
fn fellows_whitelist_call() {
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;
		type PolkadotCall = <Polkadot as Chain>::RuntimeCall;
		type PolkadotRuntime = <Polkadot as Chain>::Runtime;

		let call_hash = [1u8; 32].into();

		let whitelist_call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::V4(Parent.into())),
			message: bx!(VersionedXcm::V4(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					require_weight_at_most: Weight::from_parts(5_000_000_000, 500_000),
					call: PolkadotCall::Whitelist(
						pallet_whitelist::Call::<PolkadotRuntime>::whitelist_call { call_hash }
					)
					.encode()
					.into(),
				}
			]))),
		});

		let fellows_origin: RuntimeOrigin = FellowsOrigin.into();

		assert_ok!(whitelist_call.dispatch(fellows_origin));

		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});

	Polkadot::execute_with(|| {
		type RuntimeEvent = <Polkadot as Chain>::RuntimeEvent;

		assert_expected_events!(
			Polkadot,
			vec![
				RuntimeEvent::Whitelist(pallet_whitelist::Event::CallWhitelisted { .. }) => {},
				RuntimeEvent::MessageQueue(pallet_message_queue::Event::Processed { success: true, .. }) => {},
			]
		);
	});
}
