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
use asset_hub_polkadot_runtime::{
	tx_pause::MaxNameLen, RuntimeCall, RuntimeOrigin, SafeMode, TxPause,
};
use emulated_integration_tests_common::accounts::{ALICE, BOB};
use frame_support::traits::Contains;
use polkadot_runtime_constants::currency::UNITS;
use sp_runtime::traits::Dispatchable;

type CallName = frame_support::BoundedVec<u8, MaxNameLen>;

fn balances_transfer_name() -> (CallName, CallName) {
	(b"Balances".to_vec().try_into().unwrap(), b"transfer_allow_death".to_vec().try_into().unwrap())
}

fn balances_transfer_call(dest: AccountId, value: u128) -> RuntimeCall {
	RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death { dest: dest.into(), value })
}

#[test]
fn safe_mode_blocks_non_whitelisted_calls() {
	let alice = AssetHubPolkadot::account_id_of(ALICE);
	let bob = AssetHubPolkadot::account_id_of(BOB);

	AssetHubPolkadot::execute_with(|| {
		let signed: RuntimeOrigin =
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(alice.clone());
		let root: RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin::root();

		assert_ok!(balances_transfer_call(bob.clone(), UNITS).dispatch(signed.clone()));

		assert_ok!(SafeMode::enter(signed.clone()));

		assert_err!(
			balances_transfer_call(bob.clone(), UNITS).dispatch(signed.clone()),
			frame_system::Error::<asset_hub_polkadot_runtime::Runtime>::CallFiltered,
		);

		assert_ok!(SafeMode::force_exit(root));

		assert_ok!(balances_transfer_call(bob, UNITS).dispatch(signed));
	});
}

#[test]
fn tx_pause_blocks_paused_call() {
	let alice = AssetHubPolkadot::account_id_of(ALICE);
	let bob = AssetHubPolkadot::account_id_of(BOB);

	AssetHubPolkadot::execute_with(|| {
		let signed: RuntimeOrigin =
			<AssetHubPolkadot as Chain>::RuntimeOrigin::signed(alice.clone());
		let root: RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin::root();
		let call_name = balances_transfer_name();

		assert_ok!(TxPause::pause(root.clone(), call_name.clone()));
		assert_err!(
			balances_transfer_call(bob.clone(), UNITS).dispatch(signed.clone()),
			frame_system::Error::<asset_hub_polkadot_runtime::Runtime>::CallFiltered,
		);

		assert_ok!(TxPause::unpause(root, call_name));
		assert_ok!(balances_transfer_call(bob, UNITS).dispatch(signed));
	});
}

#[test]
fn tx_pause_callable_during_safe_mode() {
	let alice = AssetHubPolkadot::account_id_of(ALICE);

	AssetHubPolkadot::execute_with(|| {
		let signed: RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(alice);
		let root: RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin::root();

		assert_ok!(SafeMode::enter(signed));
		assert_ok!(TxPause::pause(root, balances_transfer_name()));
	});
}

#[test]
fn safe_call_filter_respects_safe_mode() {
	let alice = AssetHubPolkadot::account_id_of(ALICE);
	let bob = AssetHubPolkadot::account_id_of(BOB);

	AssetHubPolkadot::execute_with(|| {
		use asset_hub_polkadot_runtime::xcm_config::XcmConfig;

		let signed: RuntimeOrigin = <AssetHubPolkadot as Chain>::RuntimeOrigin::signed(alice);
		let transfer = balances_transfer_call(bob, UNITS);

		assert!(<XcmConfig as xcm_executor::Config>::SafeCallFilter::contains(&transfer));
		assert_ok!(SafeMode::enter(signed));
		assert!(!<XcmConfig as xcm_executor::Config>::SafeCallFilter::contains(&transfer));
	});
}
