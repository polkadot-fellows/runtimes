// This file is part of Cumulus.

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

//! Runtime-level tests for safe-mode and tx-pause configuration.
//!

use asset_hub_polkadot_runtime::{
	tx_pause::MaxNameLen, xcm_config::XcmConfig, AllPalletsWithoutSystem, Balances,
	ExistentialDeposit, Runtime, RuntimeCall, RuntimeOrigin, SafeMode, SessionKeys, TxPause,
};
use asset_test_utils::{ExtBuilder, RuntimeHelper};
use frame_support::{
	assert_err, assert_ok,
	traits::{fungible::Mutate, Contains},
};
use sp_runtime::traits::Dispatchable;
use parachains_common::{AccountId, AssetHubPolkadotAuraId as AuraId};
use polkadot_runtime_constants::currency::UNITS;

const ALICE: [u8; 32] = [1u8; 32];
const BOB: [u8; 32] = [2u8; 32];

type TestRuntimeHelper = RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

fn test_ext() -> ExtBuilder<Runtime> {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
}

type CallName = frame_support::BoundedVec<u8, MaxNameLen>;

fn balances_transfer_name() -> (CallName, CallName) {
	(
		b"Balances".to_vec().try_into().unwrap(),
		b"transfer_allow_death".to_vec().try_into().unwrap(),
	)
}

fn balances_transfer_call(dest: AccountId, value: u128) -> RuntimeCall {
	RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
		dest: dest.into(),
		value,
	})
}

#[test]
fn safe_mode_blocks_non_whitelisted_calls() {
	test_ext().build().execute_with(|| {
		let alice = AccountId::from(ALICE);
		let bob = AccountId::from(BOB);
		let signed: RuntimeOrigin = TestRuntimeHelper::origin_of(alice.clone());
		let root: RuntimeOrigin = TestRuntimeHelper::root_origin();

		assert_ok!(Balances::mint_into(&alice, 200_000 * UNITS + ExistentialDeposit::get()));

		assert_ok!(balances_transfer_call(bob.clone(), UNITS).dispatch(signed.clone()));

		assert_ok!(SafeMode::enter(signed.clone()));

		assert_err!(
			balances_transfer_call(bob.clone(), UNITS).dispatch(signed.clone()),
			frame_system::Error::<Runtime>::CallFiltered,
		);

		assert_ok!(SafeMode::force_exit(root));

		assert_ok!(balances_transfer_call(bob, UNITS).dispatch(signed));
	});
}

#[test]
fn tx_pause_blocks_paused_call() {
	test_ext().build().execute_with(|| {
		let alice = AccountId::from(ALICE);
		let bob = AccountId::from(BOB);
		let signed: RuntimeOrigin = TestRuntimeHelper::origin_of(alice.clone());
		let root: RuntimeOrigin = TestRuntimeHelper::root_origin();
		let call_name = balances_transfer_name();

		assert_ok!(Balances::mint_into(&alice, ExistentialDeposit::get() + UNITS));

		assert_ok!(TxPause::pause(root.clone(), call_name.clone()));
		assert_err!(
			balances_transfer_call(bob.clone(), UNITS).dispatch(signed.clone()),
			frame_system::Error::<Runtime>::CallFiltered,
		);

		assert_ok!(TxPause::unpause(root, call_name));
		assert_ok!(balances_transfer_call(bob, UNITS).dispatch(signed));
	});
}

#[test]
fn tx_pause_callable_during_safe_mode() {
	test_ext().build().execute_with(|| {
		let alice = AccountId::from(ALICE);
		let signed: RuntimeOrigin = TestRuntimeHelper::origin_of(alice.clone());
		let root: RuntimeOrigin = TestRuntimeHelper::root_origin();

		assert_ok!(Balances::mint_into(&alice, 200_000 * UNITS + ExistentialDeposit::get()));

		assert_ok!(SafeMode::enter(signed));
		assert_ok!(TxPause::pause(root, balances_transfer_name()));
	});
}

#[test]
fn safe_call_filter_respects_safe_mode() {
	test_ext().build().execute_with(|| {
		let alice = AccountId::from(ALICE);
		let transfer = balances_transfer_call(AccountId::from(BOB), UNITS);

		assert_ok!(Balances::mint_into(&alice, 200_000 * UNITS + ExistentialDeposit::get()));

		let signed: RuntimeOrigin = TestRuntimeHelper::origin_of(alice);

		assert!(<XcmConfig as xcm_executor::Config>::SafeCallFilter::contains(&transfer));
		assert_ok!(SafeMode::enter(signed));
		assert!(!<XcmConfig as xcm_executor::Config>::SafeCallFilter::contains(&transfer));
	});
}
