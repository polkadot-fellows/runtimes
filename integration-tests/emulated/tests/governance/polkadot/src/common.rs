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

use crate::imports::*;

/// CollectivesPolkadot dispatches `pallet_xcm::send` with `OriginKind:Xcm` to the dest with encoded
/// whitelist call.
pub fn collectives_send_whitelist(
	dest: Location,
	encoded_whitelist_call: impl FnOnce() -> Vec<u8>,
) {
	CollectivesPolkadot::execute_with(|| {
		type RuntimeEvent = <CollectivesPolkadot as Chain>::RuntimeEvent;
		type RuntimeCall = <CollectivesPolkadot as Chain>::RuntimeCall;
		type RuntimeOrigin = <CollectivesPolkadot as Chain>::RuntimeOrigin;
		type Runtime = <CollectivesPolkadot as Chain>::Runtime;

		let whitelist_call = RuntimeCall::PolkadotXcm(pallet_xcm::Call::<Runtime>::send {
			dest: bx!(VersionedLocation::from(dest)),
			message: bx!(VersionedXcm::from(Xcm(vec![
				UnpaidExecution { weight_limit: Unlimited, check_origin: None },
				Transact {
					origin_kind: OriginKind::Xcm,
					fallback_max_weight: None,
					call: encoded_whitelist_call().into(),
				}
			]))),
		});

		use collectives_polkadot_runtime::fellowship::pallet_fellowship_origins::Origin::Fellows as FellowsOrigin;
		let fellows_origin: RuntimeOrigin = FellowsOrigin.into();
		assert_ok!(whitelist_call.dispatch(fellows_origin));
		assert_expected_events!(
			CollectivesPolkadot,
			vec![
				RuntimeEvent::PolkadotXcm(pallet_xcm::Event::Sent { .. }) => {},
			]
		);
	});
}
