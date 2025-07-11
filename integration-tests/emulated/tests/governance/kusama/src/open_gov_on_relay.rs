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

use emulated_integration_tests_common::{
	assert_whitelisted,
	impls::RelayChain,
	xcm_emulator::{Chain, Parachain, TestExt},
	xcm_helpers::{
		build_xcm_send_authorize_upgrade_call, call_hash_of,
		dispatch_whitelisted_call_with_preimage,
	},
};
use frame_support::{assert_err, assert_ok};
use kusama_runtime::{governance::pallet_custom_origins::Origin, Dmp};
use kusama_system_emulated_network::{
	AssetHubKusamaPara as AssetHubKusama, BridgeHubKusamaPara as BridgeHubKusama,
	CoretimeKusamaPara as CoretimeKusama, KusamaRelay as Kusama, PeopleKusamaPara as PeopleKusama,
};
use sp_runtime::{traits::Dispatchable, DispatchError};

#[test]
fn relaychain_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type KusamaRuntime = <Kusama as Chain>::Runtime;
	type KusamaRuntimeCall = <Kusama as Chain>::RuntimeCall;
	type KusamaRuntimeOrigin = <Kusama as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		KusamaRuntimeCall::Utility(pallet_utility::Call::<KusamaRuntime>::force_batch {
			calls: vec![
				// upgrade the relaychain
				KusamaRuntimeCall::System(frame_system::Call::authorize_upgrade { code_hash }),
			],
		});

	// bad origin
	let invalid_origin: KusamaRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: KusamaRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<Kusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Kusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 44,
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
		assert_whitelisted!(Kusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Kusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	Kusama::execute_with(|| assert!(<Kusama as Chain>::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage::<Kusama>(authorize_upgrade, ok_origin));

	// check after - authorized
	Kusama::execute_with(|| {
		assert_eq!(<Kusama as Chain>::System::authorized_upgrade().unwrap().code_hash(), &code_hash)
	});
}

#[test]
fn relaychain_can_authorize_upgrade_for_system_chains() {
	type KusamaRuntime = <Kusama as Chain>::Runtime;
	type KusamaRuntimeCall = <Kusama as Chain>::RuntimeCall;
	type KusamaRuntimeOrigin = <Kusama as Chain>::RuntimeOrigin;

	Kusama::execute_with(|| {
		Dmp::make_parachain_reachable(AssetHubKusama::para_id());
		Dmp::make_parachain_reachable(BridgeHubKusama::para_id());
		Dmp::make_parachain_reachable(CoretimeKusama::para_id());
		Dmp::make_parachain_reachable(PeopleKusama::para_id());
	});

	let code_hash_asset_hub = [1u8; 32].into();
	let code_hash_bridge_hub = [2u8; 32].into();
	let code_hash_coretime = [4u8; 32].into();
	let code_hash_people = [5u8; 32].into();

	let authorize_upgrade =
		KusamaRuntimeCall::Utility(pallet_utility::Call::<KusamaRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<Kusama, AssetHubKusama>(
					Kusama::child_location_of(AssetHubKusama::para_id()),
					&code_hash_asset_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<Kusama, BridgeHubKusama>(
					Kusama::child_location_of(BridgeHubKusama::para_id()),
					&code_hash_bridge_hub,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<Kusama, CoretimeKusama>(
					Kusama::child_location_of(CoretimeKusama::para_id()),
					&code_hash_coretime,
					None,
				),
				build_xcm_send_authorize_upgrade_call::<Kusama, PeopleKusama>(
					Kusama::child_location_of(PeopleKusama::para_id()),
					&code_hash_people,
					None,
				),
			],
		});

	// bad origin
	let invalid_origin: KusamaRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: KusamaRuntimeOrigin = Origin::WhitelistedCaller.into();

	let call_hash = call_hash_of::<Kusama>(&authorize_upgrade);

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Kusama>(
			authorize_upgrade.clone(),
			ok_origin.clone()
		),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 44,
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
		assert_whitelisted!(Kusama, call_hash);
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage::<Kusama>(
			authorize_upgrade.clone(),
			invalid_origin
		),
		DispatchError::BadOrigin
	);

	// check before
	AssetHubKusama::execute_with(|| {
		assert!(<AssetHubKusama as Chain>::System::authorized_upgrade().is_none())
	});
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
	assert_ok!(dispatch_whitelisted_call_with_preimage::<Kusama>(authorize_upgrade, ok_origin));

	AssetHubKusama::execute_with(|| {
		assert_eq!(
			<AssetHubKusama as Chain>::System::authorized_upgrade().unwrap().code_hash(),
			&code_hash_asset_hub
		)
	});
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
