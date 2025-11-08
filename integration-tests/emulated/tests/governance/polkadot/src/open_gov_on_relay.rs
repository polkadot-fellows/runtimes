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

#[test]
fn relaychain_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type PolkadotRuntime = <Polkadot as Chain>::Runtime;
	type PolkadotRuntimeCall = <Polkadot as Chain>::RuntimeCall;
	type PolkadotRuntimeOrigin = <Polkadot as Chain>::RuntimeOrigin;

	// upgrade the relaychain
	let authorize_upgrade =
		PolkadotRuntimeCall::System(frame_system::Call::authorize_upgrade { code_hash });

	// bad origin
	let invalid_origin: PolkadotRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: PolkadotRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<Polkadot>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Polkadot>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 23,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(Location::parent(), || {
		PolkadotRuntimeCall::Whitelist(pallet_whitelist::Call::<PolkadotRuntime>::whitelist_call {
			call_hash,
		})
		.encode()
	});
	Polkadot::execute_with(|| {
		assert_whitelisted!(Polkadot, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Polkadot>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	Polkadot::execute_with(|| assert!(<Polkadot as Chain>::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<Polkadot>(authorize_upgrade, ok_origin));

	// check after - authorized
	Polkadot::execute_with(|| {
		assert_eq!(
			<Polkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash
		)
	});
}

#[test]
fn relaychain_can_authorize_upgrade_for_system_chains() {
	type PolkadotRuntime = <Polkadot as Chain>::Runtime;
	type PolkadotRuntimeCall = <Polkadot as Chain>::RuntimeCall;
	type PolkadotRuntimeOrigin = <Polkadot as Chain>::RuntimeOrigin;

	let code_hash_asset_hub = [1u8; 32].into();
	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_collectives = [3u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	Polkadot::execute_with(|| {
		Dmp::make_parachain_reachable(AssetHubPolkadot::para_id());
		Dmp::make_parachain_reachable(BridgeHubPolkadot::para_id());
		Dmp::make_parachain_reachable(CollectivesPolkadot::para_id());
		Dmp::make_parachain_reachable(CoretimePolkadot::para_id());
		Dmp::make_parachain_reachable(PeoplePolkadot::para_id());
	});

	let authorize_upgrade =
		PolkadotRuntimeCall::Utility(pallet_utility::Call::<PolkadotRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<Polkadot, AssetHubPolkadot>(
					Polkadot::child_location_of(AssetHubPolkadot::para_id()),
					&code_hash_asset_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, BridgeHubPolkadot>(
					Polkadot::child_location_of(BridgeHubPolkadot::para_id()),
					&code_hash_bridge_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, CollectivesPolkadot>(
					Polkadot::child_location_of(CollectivesPolkadot::para_id()),
					&code_hash_collectives,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, CoretimePolkadot>(
					Polkadot::child_location_of(CoretimePolkadot::para_id()),
					&code_hash_coretime,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, PeoplePolkadot>(
					Polkadot::child_location_of(PeoplePolkadot::para_id()),
					&code_hash_people,
					None,
				),
			],
		});

	// bad origin
	let invalid_origin: PolkadotRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: PolkadotRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<Polkadot>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Polkadot>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 23,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(Location::parent(), || {
		PolkadotRuntimeCall::Whitelist(pallet_whitelist::Call::<PolkadotRuntime>::whitelist_call {
			call_hash,
		})
		.encode()
	});
	Polkadot::execute_with(|| {
		assert_whitelisted!(Polkadot, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Polkadot>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	AssetHubPolkadot::execute_with(|| {
		assert!(<AssetHubPolkadot as Chain>::System::authorized_upgrade().is_none())
	});
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
	assert_ok!(dispatch_whitelisted_call_with_preimage::<Polkadot>(authorize_upgrade, ok_origin));

	// check after - authorized
	AssetHubPolkadot::execute_with(|| {
		assert_eq!(
			<AssetHubPolkadot as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_asset_hub
		)
	});
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
