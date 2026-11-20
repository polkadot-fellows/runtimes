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

//! Runtime tests for the kill switch and the tx-pause instance behind it.

use asset_hub_polkadot_runtime::{
	kill_switch::{set_suggested_deposits, KillSwitchGroup},
	xcm_config::{FellowshipLocation, XcmConfig},
	AllPalletsWithoutSystem, Balances, KillSwitch, Runtime, RuntimeCall, RuntimeHoldReason,
	RuntimeOrigin, SessionKeys, TxPause,
};
use asset_test_utils::{ExtBuilder, RuntimeHelper};
use frame_support::{
	assert_noop, assert_ok,
	traits::{
		fungible::{InspectHold, Mutate},
		Contains,
	},
};
use pallet_kill_switch::{Target, TargetOf};
use parachains_common::{AccountId, AssetHubPolkadotAuraId as AuraId};
use sp_runtime::{traits::Dispatchable, DispatchError};
use system_parachains_constants::polkadot::currency::UNITS;
use xcm::latest::prelude::*;

const ALICE: [u8; 32] = [1u8; 32];
const BOB: [u8; 32] = [2u8; 32];

type TestRuntimeHelper = RuntimeHelper<Runtime, AllPalletsWithoutSystem>;

fn test_ext() -> sp_io::TestExternalities {
	ExtBuilder::<Runtime>::default()
		.with_tracing()
		.with_collators(vec![AccountId::from(ALICE)])
		.with_session_keys(vec![(
			AccountId::from(ALICE),
			AccountId::from(ALICE),
			SessionKeys { aura: AuraId::from(sp_core::ed25519::Public::from_raw(ALICE)) },
		)])
		.build()
}

/// Alice with enough for any deposit, with the kill switch turned on.
fn funded_alice() -> AccountId {
	set_suggested_deposits();
	let alice = AccountId::from(ALICE);
	assert_ok!(Balances::mint_into(&alice, 200_000 * UNITS));
	alice
}

fn asset_transfer() -> RuntimeCall {
	RuntimeCall::Assets(pallet_assets::Call::transfer {
		id: 1u32.into(),
		target: AccountId::from(BOB).into(),
		amount: 1,
	})
}

fn balance_transfer() -> RuntimeCall {
	RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death {
		dest: AccountId::from(BOB).into(),
		value: UNITS,
	})
}

/// Whether the call filter stops `call` from `who`.
fn is_filtered(call: RuntimeCall, who: &AccountId) -> bool {
	match call.dispatch(TestRuntimeHelper::origin_of(who.clone())) {
		Ok(_) => false,
		Err(e) => e.error == frame_system::Error::<Runtime>::CallFiltered.into(),
	}
}

fn fellowship() -> RuntimeOrigin {
	let voice = FellowshipLocation::get()
		.pushed_with_interior(Plurality { id: BodyId::Technical, part: BodyPart::Voice })
		.unwrap();
	pallet_xcm::Origin::Xcm(voice).into()
}

fn group(group: KillSwitchGroup) -> TargetOf<Runtime> {
	Target::Group(group)
}

fn held(who: &AccountId) -> u128 {
	Balances::balance_on_hold(
		&RuntimeHoldReason::KillSwitch(pallet_kill_switch::HoldReason::Pause),
		who,
	)
}

#[test]
fn pausing_blocks_the_target_calls() {
	test_ext().execute_with(|| {
		let alice = funded_alice();
		assert!(!is_filtered(asset_transfer(), &alice));

		assert_ok!(KillSwitch::pause(
			TestRuntimeHelper::origin_of(alice.clone()),
			group(KillSwitchGroup::AssetTransfers)
		));

		assert!(is_filtered(asset_transfer(), &alice));
		// Wrapping it doesn't help.
		let batch =
			RuntimeCall::Utility(pallet_utility::Call::batch_all { calls: vec![asset_transfer()] });
		assert!(is_filtered(batch, &alice));
		// Other calls still go through.
		assert!(!is_filtered(balance_transfer(), &alice));
	});
}

#[test]
fn xcm_transact_respects_pauses() {
	test_ext().execute_with(|| {
		let alice = funded_alice();
		type SafeCallFilter = <XcmConfig as xcm_executor::Config>::SafeCallFilter;
		assert!(SafeCallFilter::contains(&asset_transfer()));

		assert_ok!(KillSwitch::pause(
			TestRuntimeHelper::origin_of(alice),
			group(KillSwitchGroup::AssetTransfers)
		));

		assert!(!SafeCallFilter::contains(&asset_transfer()));
		assert!(SafeCallFilter::contains(&balance_transfer()));
	});
}

#[test]
fn fellowship_can_slash_over_xcm() {
	test_ext().execute_with(|| {
		let alice = funded_alice();
		let free = Balances::free_balance(&alice);
		assert_ok!(KillSwitch::pause(
			TestRuntimeHelper::origin_of(alice.clone()),
			group(KillSwitchGroup::Psm)
		));
		assert_eq!(held(&alice), 10_000 * UNITS);

		// Not just anyone.
		assert_noop!(
			KillSwitch::unpause(
				TestRuntimeHelper::origin_of(AccountId::from(BOB)),
				group(KillSwitchGroup::Psm),
				true
			),
			DispatchError::BadOrigin
		);

		assert_ok!(KillSwitch::unpause(fellowship(), group(KillSwitchGroup::Psm), true));
		assert_eq!(held(&alice), 0);
		assert_eq!(Balances::free_balance(&alice), free - 10_000 * UNITS);
	});
}

#[test]
fn only_root_can_force_pause() {
	test_ext().execute_with(|| {
		assert_noop!(
			KillSwitch::force_pause(fellowship(), group(KillSwitchGroup::Psm), None),
			DispatchError::BadOrigin
		);
		assert_ok!(KillSwitch::force_pause(
			TestRuntimeHelper::root_origin(),
			group(KillSwitchGroup::Psm),
			None
		));
		// And the Fellowship can't lift it.
		assert_noop!(
			KillSwitch::unpause(fellowship(), group(KillSwitchGroup::Psm), false),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn only_the_kill_switch_drives_tx_pause() {
	test_ext().execute_with(|| {
		let name = (
			b"Balances".to_vec().try_into().unwrap(),
			b"transfer_allow_death".to_vec().try_into().unwrap(),
		);
		assert_noop!(
			TxPause::pause(TestRuntimeHelper::root_origin(), name),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn can_pause_a_single_call() {
	test_ext().execute_with(|| {
		let alice = funded_alice();
		let call = Target::Call((
			b"Balances".to_vec().try_into().unwrap(),
			b"transfer_allow_death".to_vec().try_into().unwrap(),
		));
		assert_ok!(KillSwitch::pause(TestRuntimeHelper::origin_of(alice.clone()), call));

		// Not in any group, so the plain call deposit.
		assert_eq!(held(&alice), 10_000 * UNITS);
		assert!(is_filtered(balance_transfer(), &alice));
		assert!(!is_filtered(asset_transfer(), &alice));
	});
}

#[test]
fn single_calls_cost_at_least_their_group() {
	test_ext().execute_with(|| {
		set_suggested_deposits();
		let name = |pallet: &str, call: &str| {
			(
				pallet.as_bytes().to_vec().try_into().unwrap(),
				call.as_bytes().to_vec().try_into().unwrap(),
			)
		};
		assert_eq!(KillSwitch::call_deposit(&name("Psm", "mint")), Some(10_000 * UNITS));
		assert_eq!(KillSwitch::call_deposit(&name("PolkadotXcm", "execute")), Some(50_000 * UNITS));
		assert_eq!(KillSwitch::call_deposit(&name("Assets", "transfer")), Some(50_000 * UNITS));
	});
}

#[test]
fn lists_every_group() {
	test_ext().execute_with(|| {
		set_suggested_deposits();
		let groups = KillSwitch::pause_groups();
		assert_eq!(groups.len(), 4);
		assert!(groups.iter().all(|info| !info.calls.is_empty() && info.deposit.is_some()));
	});
}

#[test]
fn ships_switched_off() {
	test_ext().execute_with(|| {
		let alice = AccountId::from(ALICE);
		assert_ok!(Balances::mint_into(&alice, 200_000 * UNITS));

		assert!(KillSwitch::pause_groups().iter().all(|info| info.deposit.is_none()));
		assert_noop!(
			KillSwitch::pause(TestRuntimeHelper::origin_of(alice), group(KillSwitchGroup::Psm)),
			pallet_kill_switch::Error::<Runtime>::NotPausable
		);
		// Root still can.
		assert_ok!(KillSwitch::force_pause(
			TestRuntimeHelper::root_origin(),
			group(KillSwitchGroup::Psm),
			None
		));
	});
}

#[test]
fn only_root_sets_the_deposits() {
	use asset_hub_polkadot_runtime::{dynamic_params::kill_switch, Parameters, RuntimeParameters};
	test_ext().execute_with(|| {
		let param = RuntimeParameters::KillSwitch(kill_switch::Parameters::CallDeposit(
			kill_switch::CallDeposit,
			Some(1),
		));
		assert_noop!(
			Parameters::set_parameter(fellowship(), param.clone()),
			DispatchError::BadOrigin
		);
		assert_ok!(Parameters::set_parameter(TestRuntimeHelper::root_origin(), param));
	});
}
