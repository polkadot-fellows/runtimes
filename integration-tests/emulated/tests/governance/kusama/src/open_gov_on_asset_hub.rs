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
use asset_hub_kusama_runtime::governance::pallet_custom_origins::Origin;
use emulated_integration_tests_common::xcm_emulator::{Chain, Parachain, TestExt};
use frame_support::{assert_ok, assert_err};
use kusama_system_emulated_network::{
	AssetHubKusamaPara as AssetHubKusama, BridgeHubKusamaPara as BridgeHubKusama,
	CoretimeKusamaPara as CoretimeKusama, PeopleKusamaPara as PeopleKusama,
	KusamaRelay as Kusama,
};
use sp_runtime::DispatchError;
use integration_tests_helpers::{call_hash_of, dispatch_whitelisted_call_with_preimage, build_xcm_send_authorize_upgrade_call};


#[test]
fn assethub_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![AssetHubRuntimeCall::System(frame_system::Call::authorize_upgrade {
				code_hash,
			})],
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
			index: 64,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	AssetHubKusama::execute_with(|| {
		let whitelist_call =
			AssetHubRuntimeCall::Whitelist(pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call {
				call_hash,
			});
		use kusama_runtime::governance::pallet_custom_origins::Origin::Fellows as FellowsOrigin;
		let fellows_origin: AssetHubRuntimeOrigin = FellowsOrigin.into();
		assert_ok!(whitelist_call.dispatch(fellows_origin));
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
	AssetHubKusama::execute_with(|| {
		assert!(<AssetHubKusama as Chain>::System::authorized_upgrade().is_none())
	});

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	AssetHubKusama::execute_with(|| {
		assert!(<AssetHubKusama as Chain>::System::authorized_upgrade().is_some())
	});
}

#[test]
fn assethub_can_authorize_upgrade_for_relay_chain() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![build_xcm_send_authorize_upgrade_call::<AssetHubKusama, Kusama>(
				AssetHubKusama::parent_location(),
			)],
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
			index: 64,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	AssetHubKusama::execute_with(|| {
		let whitelist_call =
			AssetHubRuntimeCall::Whitelist(pallet_whitelist::Call::<AssetHubRuntime>::whitelist_call {
				call_hash,
			});
		use kusama_runtime::governance::pallet_custom_origins::Origin::Fellows as FellowsOrigin;
		let fellows_origin: AssetHubRuntimeOrigin = FellowsOrigin.into();
		assert_ok!(whitelist_call.dispatch(fellows_origin));
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
	Kusama::execute_with(|| assert!(<Kusama as Chain>::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<AssetHubKusama>(
		authorize_upgrade,
		ok_origin
	));

	// check after - authorized
	Kusama::execute_with(|| assert!(<Kusama as Chain>::System::authorized_upgrade().is_some()));
}

#[test]
fn assethub_can_authorize_upgrade_for_system_chains() {
	type AssetHubRuntime = <AssetHubKusama as Chain>::Runtime;
	type AssetHubRuntimeCall = <AssetHubKusama as Chain>::RuntimeCall;
	type AssetHubRuntimeOrigin = <AssetHubKusama as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		AssetHubRuntimeCall::Utility(pallet_utility::Call::<AssetHubRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, BridgeHubKusama>(
					AssetHubKusama::sibling_location_of(BridgeHubKusama::para_id()),
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, CoretimeKusama>(
					AssetHubKusama::sibling_location_of(CoretimeKusama::para_id()),
				),
				build_xcm_send_authorize_upgrade_call::<AssetHubKusama, PeopleKusama>(
					AssetHubKusama::sibling_location_of(PeopleKusama::para_id()),
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
			index: 64,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	Kusama::execute_with(|| {
		let whitelist_call =
			KusamaRuntimeCall::Whitelist(pallet_whitelist::Call::<KusamaRuntime>::whitelist_call {
				call_hash,
			});
		use kusama_runtime::governance::pallet_custom_origins::Origin::Fellows as FellowsOrigin;
		let fellows_origin: KusamaRuntimeOrigin = FellowsOrigin.into();
		assert_ok!(whitelist_call.dispatch(fellows_origin));
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

	// check after - authorized
	BridgeHubKusama::execute_with(|| {
		assert!(<BridgeHubKusama as Chain>::System::authorized_upgrade().is_some())
	});
	CoretimeKusama::execute_with(|| {
		assert!(<CoretimeKusama as Chain>::System::authorized_upgrade().is_some())
	});
	PeopleKusama::execute_with(|| {
		assert!(<PeopleKusama as Chain>::System::authorized_upgrade().is_some())
	});
}
