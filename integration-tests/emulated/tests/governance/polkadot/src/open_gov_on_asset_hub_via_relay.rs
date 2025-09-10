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
use crate::{common::*, imports::*};

use asset_hub_polkadot_runtime::governance::pallet_custom_origins::Origin;

#[test]
fn assethub_can_authorize_upgrade_for_system_chains_via_relay_chain() {
	type AssetHubRuntime = <AssetHubPolkadot as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubPolkadot as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin;

	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_collectives = [3u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	Polkadot::execute_with(|| {
		Dmp::make_parachain_reachable(BridgeHubPolkadot::para_id());
		Dmp::make_parachain_reachable(CollectivesPolkadot::para_id());
		Dmp::make_parachain_reachable(CoretimePolkadot::para_id());
		Dmp::make_parachain_reachable(PeoplePolkadot::para_id());
	});

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![
				build_xcm_send_call::<AssetHubPolkadot, Polkadot>(
					AssetHubPolkadot::parent_location(),
					None,
					build_xcm_send_authorize_upgrade_call::<Polkadot, BridgeHubPolkadot>(
						Polkadot::child_location_of(BridgeHubPolkadot::para_id()),
						&code_hash_bridge_hub,
						None,
					),
					OriginKind::Superuser,
				),
				build_xcm_send_call::<AssetHubPolkadot, Polkadot>(
					AssetHubPolkadot::parent_location(),
					None,
					build_xcm_send_authorize_upgrade_call::<Polkadot, CollectivesPolkadot>(
						Polkadot::child_location_of(CollectivesPolkadot::para_id()),
						&code_hash_collectives,
						None,
					),
					OriginKind::Superuser,
				),
				build_xcm_send_call::<AssetHubPolkadot, Polkadot>(
					AssetHubPolkadot::parent_location(),
					None,
					build_xcm_send_authorize_upgrade_call::<Polkadot, CoretimePolkadot>(
						Polkadot::child_location_of(CoretimePolkadot::para_id()),
						&code_hash_coretime,
						None,
					),
					OriginKind::Superuser,
				),
				build_xcm_send_call::<AssetHubPolkadot, Polkadot>(
					AssetHubPolkadot::parent_location(),
					None,
					build_xcm_send_authorize_upgrade_call::<Polkadot, PeoplePolkadot>(
						Polkadot::child_location_of(PeoplePolkadot::para_id()),
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

	let call_hash = call_hash_of::<AssetHubPolkadot>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 64,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(
		CollectivesPolkadot::sibling_location_of(<AssetHubPolkadot as Parachain>::para_id()),
		|| {
			AssetHubRuntimeCall::Whitelist(
				pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call { call_hash },
			)
			.encode()
		},
	);
	AssetHubPolkadot::execute_with(|| {
		assert_whitelisted!(AssetHubPolkadot, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	BridgeHubPolkadot::execute_with(|| {
		assert!(<BridgeHubPolkadot as Chain>::System::authorized_upgrade().is_none())
	});
	CollectivesPolkadot::execute_with(|| {
		assert!(<CollectivesPolkadot as Chain>::System::authorized_upgrade().is_none())
	});
	CoretimePolkadot::execute_with(|| {
		assert!(<CoretimePolkadot as Chain>::System::authorized_upgrade().is_none())
	});
	PeoplePolkadot::execute_with(|| {
		assert!(<PeoplePolkadot as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubPolkadot>(
		authorize_upgrade,
		ok_origin
	));

	Polkadot::execute_with(|| {});

	// check after - authorized
	BridgeHubPolkadot::execute_with(|| {
		assert_eq!(
			<BridgeHubPolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_bridge_hub
		)
	});
	CollectivesPolkadot::execute_with(|| {
		assert_eq!(
			<CollectivesPolkadot as Chain>::System::authorized_upgrade()
				.unwrap()
				.code_hash(),
			&code_hash_collectives
		)
	});
	CoretimePolkadot::execute_with(|| {
		assert_eq!(
			<CoretimePolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_coretime
		)
	});
	PeoplePolkadot::execute_with(|| {
		assert_eq!(
			<PeoplePolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_people
		)
	});
}
