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

use crate::{common::collectives_send_whitelist, imports::*};
use asset_hub_kusama_runtime::governance::pallet_custom_origins::Origin;

#[test]
fn assethub_can_authorize_upgrade_for_system_chains_via_relay_chain() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	Kusama::execute_with(|| {
		Dmp::make_parachain_reachable(BridgeHubKusama::para_id());
		Dmp::make_parachain_reachable(CoretimeKusama::para_id());
		Dmp::make_parachain_reachable(PeopleKusama::para_id());
	});

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![
				build_xcm_send_call::<AssetHubKusama, Kusama>(
					AssetHubKusama::parent_location(),
					None,
					build_xcm_send_authorize_upgrade_call::<Kusama, BridgeHubKusama>(
						Kusama::child_location_of(BridgeHubKusama::para_id()),
						&code_hash_bridge_hub,
						None,
					),
					OriginKind::Superuser,
				),
				build_xcm_send_call::<AssetHubKusama, Kusama>(
					AssetHubKusama::parent_location(),
					None,
					build_xcm_send_authorize_upgrade_call::<Kusama, CoretimeKusama>(
						Kusama::child_location_of(CoretimeKusama::para_id()),
						&code_hash_coretime,
						None,
					),
					OriginKind::Superuser,
				),
				build_xcm_send_call::<AssetHubKusama, Kusama>(
					AssetHubKusama::parent_location(),
					None,
					build_xcm_send_authorize_upgrade_call::<Kusama, PeopleKusama>(
						Kusama::child_location_of(PeopleKusama::para_id()),
						&code_hash_people,
						None,
					),
					OriginKind::Superuser,
				),
			],
		});

	// bad origin
	let invalid_origin: AssetHubRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: AssetHubRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<AssetHubKusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 94,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist from Kusama
	collectives_send_whitelist(
		Kusama::child_location_of(<AssetHubKusama as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubKusama::execute_with(|| {
		assert_whitelisted!(AssetHubKusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	BridgeHubKusama::execute_with(|| {
		assert!(<BridgeHubKusama as Chain>::System::authorized_upgrade().is_none())
	});
	CoretimeKusama::execute_with(|| {
		assert!(<CoretimeKusama as Chain>::System::authorized_upgrade().is_none())
	});
	PeopleKusama::execute_with(|| {
		assert!(<PeopleKusama as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	Kusama::execute_with(|| {});

	// check after - authorized
	BridgeHubKusama::execute_with(|| {
		assert_eq!(
			<BridgeHubKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_bridge_hub
		)
	});
	CoretimeKusama::execute_with(|| {
		assert_eq!(
			<CoretimeKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_coretime
		)
	});
	PeopleKusama::execute_with(|| {
		assert_eq!(
			<PeopleKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_people
		)
	});
}
