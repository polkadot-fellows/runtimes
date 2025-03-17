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

use crate::collectives_send_whitelist;
use codec::Encode;
use emulated_integration_tests_common::xcm_emulator::{Chain, TestExt};
use frame_support::dispatch::DispatchResultWithPostInfo;
use frame_support::{assert_err, assert_ok};
use polkadot_runtime::governance::pallet_custom_origins::Origin;
use polkadot_system_emulated_network::{
	polkadot_emulated_chain::PolkadotRelayPallet as PolkadotPallet, PolkadotRelay as Polkadot,
};
use sp_core::H256;
use sp_runtime::traits::Dispatchable;
use sp_runtime::traits::Hash;
use sp_runtime::DispatchError;
use xcm::latest::Location;

fn store_preimage(call: polkadot_runtime::RuntimeCall) -> H256 {
	Polkadot::execute_with(|| {
		type Runtime = <Polkadot as Chain>::Runtime;
		type Preimage = <Polkadot as PolkadotPallet>::Preimage;

		// get hash and store preimage
		let call_hash = <Runtime as frame_system::Config>::Hashing::hash(call.encode().as_ref());
		// Root is not important here, we could have an account with enough balance also.
		assert_ok!(Preimage::note_preimage(polkadot_runtime::RuntimeOrigin::root(), call.encode()));

		call_hash
	})
}

fn dispatch_whitelisted_call_with_preimage(
	call: polkadot_runtime::RuntimeCall,
	origin: polkadot_runtime::RuntimeOrigin,
) -> DispatchResultWithPostInfo {
	Polkadot::execute_with(|| {
		type Runtime = <Polkadot as Chain>::Runtime;

		// wrap with whitelist call
		let whitelist_call = polkadot_runtime::RuntimeCall::Whitelist(
			pallet_whitelist::Call::<Runtime>::dispatch_whitelisted_call_with_preimage {
				call: Box::new(call),
			},
		);

		whitelist_call.dispatch(origin)
	})
}

#[test]
fn can_authorize_upgrade_for_relaychain() {
	let code_hash = [1u8; 32].into();
	type Runtime = <Polkadot as Chain>::Runtime;

	let authorize_upgrade =
		polkadot_runtime::RuntimeCall::Utility(pallet_utility::Call::<Runtime>::force_batch {
			calls: vec![
				// upgrade the relaychain
				polkadot_runtime::RuntimeCall::System(frame_system::Call::authorize_upgrade {
					code_hash,
				}),
			],
		});

	// bad origin
	let invalid_origin: polkadot_runtime::RuntimeOrigin = Origin::StakingAdmin.into();
	// ok origin
	let ok_origin: polkadot_runtime::RuntimeOrigin = Origin::WhitelistedCaller.into();

	// store preimage
	let call_hash = store_preimage(authorize_upgrade.clone());

	// Err - when dispatch non-whitelisted
	assert_err!(
		dispatch_whitelisted_call_with_preimage(authorize_upgrade.clone(), ok_origin.clone()),
		DispatchError::Module(sp_runtime::ModuleError {
			index: 23,
			error: [3, 0, 0, 0],
			message: Some("CallIsNotWhitelisted")
		})
	);

	// whitelist
	collectives_send_whitelist(Location::parent(), || {
		polkadot_runtime::RuntimeCall::Whitelist(
			pallet_whitelist::Call::<Runtime>::whitelist_call { call_hash },
		)
		.encode()
	});

	// Err - when dispatch wrong origin
	assert_err!(
		dispatch_whitelisted_call_with_preimage(authorize_upgrade.clone(), invalid_origin),
		DispatchError::BadOrigin
	);

	// check before
	Polkadot::execute_with(|| assert!(polkadot_runtime::System::authorized_upgrade().is_none()));

	// ok - authorized
	assert_ok!(dispatch_whitelisted_call_with_preimage(authorize_upgrade, ok_origin));

	// check after - authorized
	Polkadot::execute_with(|| assert!(polkadot_runtime::System::authorized_upgrade().is_some()));
}

#[test]
fn can_authorize_upgrade_for_system_chains() {
	// TODO: upgrage AssetHub
	// TODO: upgrage Collectives
	// TODO: upgrage BridgeHub
}
