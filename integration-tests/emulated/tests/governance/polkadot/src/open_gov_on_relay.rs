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
use emulated_integration_tests_common::{
	impls::RelayChain,
	xcm_emulator::{Chain, Parachain, TestExt},
};
use frame_support::assert_err;
use polkadot_runtime::governance::pallet_custom_origins::Origin;
use polkadot_system_emulated_network::{
	AssetHubPolkadotPara as AssetHubPolkadot, BridgeHubPolkadotPara as BridgeHubPolkadot,
	CoretimePolkadotPara as CoretimePolkadot, PeoplePolkadotPara as PeoplePolkadot,
	PolkadotRelay as Polkadot,
};
use sp_runtime::DispatchError;

#[test]
fn relaychain_can_authorize_upgrade_for_itself() {
	let code_hash = [1u8; 32].into();
	type PolkadotRuntime = <Polkadot as Chain>::Runtime;
	type PolkadotRuntimeCall = <Polkadot as Chain>::RuntimeCall;
	type PolkadotRuntimeOrigin = <Polkadot as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		PolkadotRuntimeCall::Utility(pallet_utility::Call::<PolkadotRuntime>::force_batch {
			calls: vec![
				// upgrade the relaychain
				PolkadotRuntimeCall::System(frame_system::Call::authorize_upgrade { code_hash }),
			],
		});

	// bad origin
	let invalid_origin: PolkadotRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: PolkadotRuntimeOrigin = Origin::WhitelistedCaller.into();

	// store preimage
	let call_hash = dispatch_note_preimage_call::<Polkadot>(authorize_upgrade.clone());

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
	Polkadot::execute_with(|| assert!(<Polkadot as Chain>::System::authorized_upgrade().is_some()));
}

#[test]
fn relaychain_can_authorize_upgrade_for_system_chains() {
	type PolkadotRuntime = <Polkadot as Chain>::Runtime;
	type PolkadotRuntimeCall = <Polkadot as Chain>::RuntimeCall;
	type PolkadotRuntimeOrigin = <Polkadot as Chain>::RuntimeOrigin;

	let authorize_upgrade =
		PolkadotRuntimeCall::Utility(pallet_utility::Call::<PolkadotRuntime>::force_batch {
			calls: vec![
				build_xcm_send_authorize_upgrade_call::<Polkadot, AssetHubPolkadot>(
					Polkadot::child_location_of(AssetHubPolkadot::para_id()),
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, BridgeHubPolkadot>(
					Polkadot::child_location_of(BridgeHubPolkadot::para_id()),
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, CollectivesPolkadot>(
					Polkadot::child_location_of(CollectivesPolkadot::para_id()),
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, CoretimePolkadot>(
					Polkadot::child_location_of(CoretimePolkadot::para_id()),
				),
				build_xcm_send_authorize_upgrade_call::<Polkadot, PeoplePolkadot>(
					Polkadot::child_location_of(PeoplePolkadot::para_id()),
				),
			],
		});

	// bad origin
	let invalid_origin: PolkadotRuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: PolkadotRuntimeOrigin = Origin::WhitelistedCaller.into();

	// store preimage
	let call_hash = dispatch_note_preimage_call::<Polkadot>(authorize_upgrade.clone());

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

	AssetHubPolkadot::execute_with(|| {
		assert!(<AssetHubPolkadot as Chain>::System::authorized_upgrade().is_some())
	});
	// check after - authorized
	BridgeHubPolkadot::execute_with(|| {
		assert!(<BridgeHubPolkadot as Chain>::System::authorized_upgrade().is_some())
	});
	CollectivesPolkadot::execute_with(|| {
		assert!(<CollectivesPolkadot as Chain>::System::authorized_upgrade().is_some())
	});
	CoretimePolkadot::execute_with(|| {
		assert!(<CoretimePolkadot as Chain>::System::authorized_upgrade().is_some())
	});
	PeoplePolkadot::execute_with(|| {
		assert!(<PeoplePolkadot as Chain>::System::authorized_upgrade().is_some())
	});
}
