// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{mock::*, *};
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungible::InspectHold, Hooks},
};
use sp_runtime::{traits::Dispatchable, DispatchError};

fn group(group: Group) -> TargetOf<Test> {
	Target::Group(group)
}

fn transfer() -> TargetOf<Test> {
	group(Group::Transfer)
}

fn burn() -> TargetOf<Test> {
	group(Group::Burn)
}

/// The expiry list as `(end, id)`.
fn queue() -> Vec<(u64, PauseId)> {
	Expiries::<Test>::get().iter().map(|expiry| (expiry.at, expiry.id)).collect()
}

/// A single call of `Balances`.
fn call(name: &str) -> TargetOf<Test> {
	Target::Call(full_name("Balances", name))
}

/// Whether tx-pause has `call` of `Balances` paused.
fn is_paused(call: &str) -> bool {
	pallet_tx_pause::PausedCalls::<Test>::contains_key(full_name("Balances", call))
}

/// Moves the relay chain to `n` and runs the hook once, like a new block would.
fn run_to_relay(n: u64) {
	RelayBlock::set(n);
	System::set_block_number(System::block_number() + 1);
	KillSwitch::on_poll(System::block_number(), &mut WeightMeter::new());
}

fn held(who: u64) -> u64 {
	Balances::balance_on_hold(&HoldReason::Pause.into(), &who)
}

/// Whether a signed `transfer_allow_death` gets through the call filter.
fn transfer_works() -> bool {
	let call =
		RuntimeCall::Balances(pallet_balances::Call::transfer_allow_death { dest: 5, value: 1 });
	match call.dispatch(RuntimeOrigin::signed(4)) {
		Ok(_) => true,
		Err(e) => {
			assert_eq!(e.error, frame_system::Error::<Test>::CallFiltered.into());
			false
		},
	}
}

fn fellowship() -> RuntimeOrigin {
	RuntimeOrigin::signed(99)
}

#[test]
fn can_pause_call() {
	new_test_ext().execute_with(|| {
		assert!(transfer_works());

		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));

		assert!(!transfer_works());
		assert_eq!(held(1), DEPOSIT);
		let pause = Pauses::<Test>::get(transfer()).unwrap();
		assert_eq!(pause.until, Some(1 + PAUSE_DURATION));
		assert_eq!(pause.deposits.to_vec(), vec![Deposit { who: 1, amount: DEPOSIT }]);
		System::assert_has_event(
			Event::DepositHeld { target: transfer(), who: 1, amount: DEPOSIT }.into(),
		);
		System::assert_last_event(
			Event::Paused { target: transfer(), who: Some(1), until: Some(1 + PAUSE_DURATION) }
				.into(),
		);
		// Other calls still work.
		assert_ok!(RuntimeCall::Balances(pallet_balances::Call::transfer_keep_alive {
			dest: 5,
			value: 1
		})
		.dispatch(RuntimeOrigin::signed(4)));
	});
}

#[test]
fn fails_to_pause() {
	new_test_ext().execute_with(|| {
		// Not a target.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), group(Group::Unknown)),
			Error::<Test>::NotPausable
		);
		// A target, but tx-pause refuses its call.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), group(Group::Whitelisted)),
			Error::<Test>::NotPausable
		);
		// The kill switch can never be paused.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), group(Group::KillSwitchCall)),
			Error::<Test>::NotPausable
		);
		assert_noop!(
			KillSwitch::force_pause(RuntimeOrigin::root(), group(Group::KillSwitchCall), None),
			Error::<Test>::NotPausable
		);
		// No deposit, so only Root can pause it.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), group(Group::RootOnly)),
			Error::<Test>::NotPausable
		);
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), group(Group::RootOnly), None));
		// Can't afford it.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(42), transfer()),
			DispatchError::Token(sp_runtime::TokenError::FundsUnavailable)
		);
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::none(), transfer()),
			DispatchError::BadOrigin
		);

		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(2), transfer()),
			Error::<Test>::AlreadyPaused
		);
	});
}

#[test]
fn tx_pause_origins_are_disabled() {
	new_test_ext().execute_with(|| {
		let call = full_name("Balances", "transfer_allow_death");
		assert_noop!(TxPause::pause(RuntimeOrigin::root(), call.clone()), DispatchError::BadOrigin);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_noop!(TxPause::unpause(RuntimeOrigin::root(), call), DispatchError::BadOrigin);
	});
}

#[test]
fn pause_expires_and_deposit_is_released() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));

		run_to_relay(PAUSE_DURATION);
		assert!(!transfer_works());

		run_to_relay(1 + PAUSE_DURATION);
		assert!(transfer_works());
		assert!(Pauses::<Test>::get(transfer()).is_none());
		assert!(Expiries::<Test>::get().is_empty());
		System::assert_has_event(Event::Expired { target: transfer() }.into());

		// Nobody stepped in, so the deposit comes back right away.
		assert_eq!(held(1), 0);
		assert_eq!(Balances::free_balance(1), 1_000);
		System::assert_has_event(
			Event::DepositReleased { target: transfer(), who: 1, amount: DEPOSIT }.into(),
		);
		// And it can't be slashed after the fact.
		assert_noop!(KillSwitch::unpause(fellowship(), transfer(), true), Error::<Test>::NotPaused);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn expires_when_relay_block_skips() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		// Due at 11. Jump straight from 1 to 13.
		run_to_relay(13);
		assert!(transfer_works());
		assert!(Pauses::<Test>::get(transfer()).is_none());
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn every_group_can_be_paused_at_once() {
	new_test_ext().execute_with(|| {
		// Single calls take every slot they can get first.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), call("transfer_allow_death")));
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), call("burn")));
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(2), call("force_transfer")),
			Error::<Test>::TooManyPauses
		);

		for g in <Test as Config>::Groups::all() {
			if g == Group::RootOnly {
				assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), group(g), Some(100)));
			} else {
				assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), group(g)));
			}
		}
		assert_eq!(Expiries::<Test>::get().len() as u32, MAX_ACTIVE_PAUSES);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn shared_calls_stay_paused() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		// `Money` also pauses `transfer_allow_death`, which is already paused.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), group(Group::Money)));
		assert!(is_paused("burn"));
		assert_eq!(PausedCalls::<Test>::get(full_name("Balances", "transfer_allow_death")), 2);

		// `Money` still holds it.
		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), false));
		assert!(!transfer_works());
		assert_ok!(KillSwitch::do_try_state());

		assert_ok!(KillSwitch::unpause(fellowship(), group(Group::Money), false));
		assert!(transfer_works());
		assert!(!is_paused("burn"));
		assert_eq!(PausedCalls::<Test>::iter().count(), 0);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn pause_lifts_what_it_paused_after_an_upgrade() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), group(Group::Money)));
		// The runtime now says `Money` is `transfer_allow_death` and `force_transfer`.
		MoneyUpgraded::set(true);

		assert_ok!(KillSwitch::unpause(fellowship(), group(Group::Money), false));
		assert!(transfer_works());
		assert!(!is_paused("burn"));
		assert!(!is_paused("force_transfer"));
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn lists_pause_groups() {
	new_test_ext().execute_with(|| {
		let groups = KillSwitch::pause_groups();
		assert_eq!(groups.len(), <Test as Config>::Groups::all().len());
		let money = groups.iter().find(|info| info.group == Group::Money).unwrap();
		assert_eq!(
			money.calls,
			vec![full_name("Balances", "transfer_allow_death"), full_name("Balances", "burn")]
		);
		assert_eq!(money.deposit, Some(DEPOSIT));
		let root_only = groups.iter().find(|info| info.group == Group::RootOnly).unwrap();
		assert_eq!(root_only.deposit, None);
	});
}

#[test]
fn on_poll_spreads_work_over_blocks() {
	new_test_ext().execute_with(|| {
		for target in [group(Group::Transfer), group(Group::ForceTransfer), group(Group::Burn)] {
			assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), target));
		}
		RelayBlock::set(20);

		// Room for two of the three.
		let weight = <() as WeightInfo>::on_poll_base()
			.saturating_add(<() as WeightInfo>::on_poll_expire().saturating_mul(2));
		KillSwitch::on_poll(2, &mut WeightMeter::with_limit(weight));
		assert_eq!(Pauses::<Test>::iter().count(), 1);
		assert_eq!(Expiries::<Test>::get().len(), 1);
		assert_ok!(KillSwitch::do_try_state());

		KillSwitch::on_poll(3, &mut WeightMeter::new());
		assert_eq!(Pauses::<Test>::iter().count(), 0);
		assert!(Expiries::<Test>::get().is_empty());
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn on_poll_respects_weight_limit() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		RelayBlock::set(20);

		KillSwitch::on_poll(2, &mut WeightMeter::with_limit(Weight::zero()));
		assert!(Pauses::<Test>::get(transfer()).is_some());

		KillSwitch::on_poll(3, &mut WeightMeter::new());
		assert!(Pauses::<Test>::get(transfer()).is_none());
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn can_slash_pause_the_hook_has_not_ended_yet() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		// The relay chain moves past the end, but the hook hasn't run yet.
		RelayBlock::set(1 + PAUSE_DURATION);

		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), true));
		assert!(transfer_works());
		assert_eq!(held(1), 0);
		assert_eq!(Balances::free_balance(TREASURY), DEPOSIT);
		System::assert_has_event(
			Event::DepositSlashed { target: transfer(), who: 1, amount: DEPOSIT }.into(),
		);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn can_extend() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(2), transfer()));
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(3), transfer()));

		let pause = Pauses::<Test>::get(transfer()).unwrap();
		assert_eq!(pause.until, Some(1 + 3 * PAUSE_DURATION));
		let depositors: Vec<_> = pause.deposits.iter().map(|d| d.who).collect();
		assert_eq!(depositors, vec![1, 2, 3]);
		assert_eq!((held(1), held(2), held(3)), (DEPOSIT, DEPOSIT, DEPOSIT));
		assert_eq!(Expiries::<Test>::get().len(), 1);

		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(4), transfer()),
			Error::<Test>::TooManyDeposits
		);

		// The old end doesn't do anything anymore.
		run_to_relay(1 + PAUSE_DURATION);
		assert!(!transfer_works());
		run_to_relay(1 + 3 * PAUSE_DURATION);
		assert!(transfer_works());
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn fails_to_extend() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(2), transfer()),
			Error::<Test>::NotPaused
		);
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), None));
		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(2), transfer()),
			Error::<Test>::ForcedPause
		);
	});
}

#[test]
fn unpause_with_slash_starts_cooldown() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(2), transfer()));

		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), true));

		assert!(transfer_works());
		assert_eq!((held(1), held(2)), (0, 0));
		assert_eq!(Balances::free_balance(1), 1_000 - DEPOSIT);
		assert_eq!(Balances::free_balance(TREASURY), 2 * DEPOSIT);
		assert!(Expiries::<Test>::get().is_empty());
		System::assert_has_event(
			Event::CooldownStarted { target: transfer(), until: 1 + COOLDOWN }.into(),
		);
		System::assert_has_event(Event::Unpaused { target: transfer(), slashed: true }.into());

		// Nobody can pause it again until the cooldown is over, not even its call on its own.
		for target in [transfer(), call("transfer_allow_death")] {
			assert_noop!(
				KillSwitch::pause(RuntimeOrigin::signed(3), target),
				Error::<Test>::InCooldown
			);
		}
		// Another group with the same call is still fine.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(3), group(Group::Money)));
		RelayBlock::set(1 + COOLDOWN);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(3), transfer()));
		assert!(!Cooldowns::<Test>::contains_key(transfer()));
	});
}

#[test]
fn root_can_pause_during_cooldown() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), true));
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), Some(5)));
		assert!(!transfer_works());
	});
}

#[test]
fn unpause_with_refund_has_no_cooldown() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_ok!(KillSwitch::unpause(RuntimeOrigin::root(), transfer(), false));

		assert!(transfer_works());
		assert_eq!(held(1), 0);
		assert_eq!(Balances::free_balance(1), 1_000);
		assert!(!Cooldowns::<Test>::contains_key(transfer()));

		// If the problem comes back, anyone can pause right away.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), transfer()));
	});
}

#[test]
fn fails_to_unpause() {
	new_test_ext().execute_with(|| {
		assert_noop!(KillSwitch::unpause(fellowship(), transfer(), true), Error::<Test>::NotPaused);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_noop!(
			KillSwitch::unpause(RuntimeOrigin::signed(1), transfer(), false),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn fellowship_can_extend() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		let until = 1 + PAUSE_DURATION;

		assert_noop!(
			KillSwitch::force_extend(fellowship(), transfer(), until),
			Error::<Test>::NotLater
		);
		assert_ok!(KillSwitch::force_extend(fellowship(), transfer(), until + 1_000));
		assert_eq!(Pauses::<Test>::get(transfer()).unwrap().until, Some(until + 1_000));
		assert_ok!(KillSwitch::force_extend(RuntimeOrigin::root(), transfer(), until + 2_000));
		assert_eq!(Pauses::<Test>::get(transfer()).unwrap().until, Some(until + 2_000));
		assert_eq!(Expiries::<Test>::get().len(), 1);

		assert_noop!(
			KillSwitch::force_extend(RuntimeOrigin::signed(1), transfer(), until + 3_000),
			DispatchError::BadOrigin
		);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn force_pause_without_end() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			KillSwitch::force_pause(fellowship(), transfer(), None),
			DispatchError::BadOrigin
		);
		assert_noop!(
			KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), Some(1)),
			Error::<Test>::InPast
		);
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), None));
		assert!(Expiries::<Test>::get().is_empty());

		run_to_relay(1_000);
		assert!(!transfer_works());
		assert_noop!(
			KillSwitch::force_extend(RuntimeOrigin::root(), transfer(), 2_000),
			Error::<Test>::OpenEnded
		);

		// Only Root can lift its own pause.
		assert_noop!(
			KillSwitch::unpause(fellowship(), transfer(), false),
			DispatchError::BadOrigin
		);
		assert_ok!(KillSwitch::unpause(RuntimeOrigin::root(), transfer(), false));
		assert!(transfer_works());
	});
}

#[test]
fn new_pause_only_settles_its_own_deposits() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		run_to_relay(1 + PAUSE_DURATION);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), transfer()));

		// The first pause ran out and was refunded, slashing the new one only hits its depositor.
		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), true));
		assert_eq!(Balances::free_balance(1), 1_000);
		assert_eq!(Balances::free_balance(2), 1_000 - DEPOSIT);
		assert_eq!(Balances::free_balance(TREASURY), DEPOSIT);
	});
}

#[test]
fn expiry_queue_is_cleaned_up() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		RelayBlock::set(5);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), burn()));
		assert_eq!(queue(), vec![(1 + PAUSE_DURATION, 0), (5 + PAUSE_DURATION, 1)]);

		// Extending moves it to its new place in the list.
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(3), transfer()));
		assert_eq!(queue(), vec![(5 + PAUSE_DURATION, 1), (1 + 2 * PAUSE_DURATION, 0)]);

		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), false));
		assert_eq!(queue(), vec![(5 + PAUSE_DURATION, 1)]);
		assert_ok!(KillSwitch::unpause(fellowship(), burn(), false));
		assert!(Expiries::<Test>::get().is_empty());
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn fails_to_extend_pause_that_ran_out() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		// The relay chain moves past the end, but the hook hasn't run yet.
		RelayBlock::set(5 + PAUSE_DURATION);

		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(2), transfer()),
			Error::<Test>::AlreadyExpired
		);
		// The Fellowship can still keep it going, but not to a block that already passed.
		assert_noop!(
			KillSwitch::force_extend(fellowship(), transfer(), 3 + PAUSE_DURATION),
			Error::<Test>::InPast
		);
		assert_ok!(KillSwitch::force_extend(fellowship(), transfer(), 20));
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn fellowship_cannot_touch_root_pause() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			KillSwitch::force_pause(fellowship(), transfer(), Some(50)),
			DispatchError::BadOrigin
		);
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), Some(50)));
		// Nobody else can stretch it either, or the Fellowship would be stuck with it.
		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(1), transfer()),
			Error::<Test>::ForcedPause
		);

		assert_noop!(KillSwitch::unpause(fellowship(), transfer(), true), DispatchError::BadOrigin);
		assert_noop!(
			KillSwitch::force_extend(fellowship(), transfer(), 55),
			DispatchError::BadOrigin
		);

		assert_ok!(KillSwitch::unpause(RuntimeOrigin::root(), transfer(), false));
		assert!(transfer_works());
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn can_pause_single_call() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), call("transfer_allow_death")));
		assert!(!transfer_works());
		// It's in groups, so it costs what they cost.
		assert_eq!(held(1), DEPOSIT);
		System::assert_has_event(
			Event::Paused {
				target: call("transfer_allow_death"),
				who: Some(1),
				until: Some(1 + PAUSE_DURATION),
			}
			.into(),
		);

		// The group holds it too, and it stays paused until both are gone.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), transfer()));
		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), false));
		assert!(!transfer_works());

		run_to_relay(1 + PAUSE_DURATION);
		assert!(transfer_works());
		assert_eq!(held(1), 0);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn single_call_price() {
	new_test_ext().execute_with(|| {
		let name = |call: &str| full_name("Balances", call);
		// In no group.
		assert_eq!(KillSwitch::call_deposit(&name("upgrade_accounts")), Some(CALL_DEPOSIT));
		// In groups, at least what they cost.
		assert_eq!(KillSwitch::call_deposit(&name("burn")), Some(DEPOSIT));
		// In a group only Root can pause, so only Root can pause it.
		assert_eq!(KillSwitch::call_deposit(&name("force_set_balance")), None);
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), call("force_set_balance")),
			Error::<Test>::NotPausable
		);
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), call("force_set_balance"), None));
		// Doesn't exist.
		assert_eq!(KillSwitch::call_deposit(&name("no_such_call")), None);

		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), call("upgrade_accounts")));
		assert_eq!(held(1), CALL_DEPOSIT);
	});
}

#[test]
fn slashed_call_does_not_block_its_groups() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), call("transfer_allow_death")));
		assert_ok!(KillSwitch::unpause(fellowship(), call("transfer_allow_death"), true));

		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(2), call("transfer_allow_death")),
			Error::<Test>::InCooldown
		);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), transfer()));
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), group(Group::Money)));
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn calls_can_extend_with_every_group_paused() {
	new_test_ext().execute_with(|| {
		// Groups first, then single calls in what's left.
		for g in <Test as Config>::Groups::all() {
			if g == Group::RootOnly {
				assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), group(g), Some(100)));
			} else {
				assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), group(g)));
			}
		}
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), call("upgrade_accounts")));
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), call("transfer_all")));
		assert_eq!(Expiries::<Test>::get().len() as u32, MAX_ACTIVE_PAUSES);

		// A full list doesn't stop a call pause from moving.
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(3), call("upgrade_accounts")));
		assert_ok!(KillSwitch::force_extend(fellowship(), call("transfer_all"), 500));
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), call("transfer_all"), Some(600)));
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn fails_to_pause_single_call() {
	new_test_ext().execute_with(|| {
		// Doesn't exist.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), call("no_such_call")),
			Error::<Test>::NotPausable
		);
		assert_noop!(
			KillSwitch::pause(
				RuntimeOrigin::signed(1),
				Target::Call(full_name("NoSuchPallet", "transfer"))
			),
			Error::<Test>::NotPausable
		);
		// Exists, but tx-pause never pauses it.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), call("transfer_keep_alive")),
			Error::<Test>::NotPausable
		);
		assert_noop!(
			KillSwitch::pause(
				RuntimeOrigin::signed(1),
				Target::Call(full_name("KillSwitch", "unpause"))
			),
			Error::<Test>::NotPausable
		);
	});
}

#[test]
fn root_takes_over_a_paid_pause() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(2), transfer()));

		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), None));
		System::assert_has_event(Event::Unpaused { target: transfer(), slashed: false }.into());

		// Still paused, now for good, and the deposits are back.
		assert!(!transfer_works());
		assert_eq!((held(1), held(2)), (0, 0));
		let pause = Pauses::<Test>::get(transfer()).unwrap();
		assert!(pause.forced);
		assert_eq!(pause.until, None);
		assert!(Expiries::<Test>::get().is_empty());
		assert_eq!(PausedCalls::<Test>::get(full_name("Balances", "transfer_allow_death")), 1);

		// But not twice.
		assert_noop!(
			KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), Some(50)),
			Error::<Test>::AlreadyPaused
		);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn expiry_survives_ids_moving_on() {
	new_test_ext().execute_with(|| {
		// A pause that ends early, between two that end later.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		RelayBlock::set(3);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), call("burn")));
		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), false));
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		// The new pause of `transfer` got a new id.
		assert_eq!(Pauses::<Test>::get(transfer()).unwrap().id, 2);

		run_to_relay(3 + PAUSE_DURATION);
		assert!(transfer_works());
		assert!(!is_paused("burn"));
		assert!(Pauses::<Test>::iter().next().is_none());
		assert_eq!(PauseTargets::<Test>::iter().count(), 0);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn unpausable_calls_have_no_price() {
	new_test_ext().execute_with(|| {
		// tx-pause never pauses these.
		assert_eq!(KillSwitch::call_deposit(&full_name("System", "remark")), None);
		assert_eq!(KillSwitch::call_deposit(&full_name("KillSwitch", "unpause")), None);
		assert_eq!(KillSwitch::call_deposit(&full_name("Balances", "transfer_keep_alive")), None);

		// A call that's already paused still has its price, so the pause can be extended.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), call("upgrade_accounts")));
		assert_eq!(
			KillSwitch::call_deposit(&full_name("Balances", "upgrade_accounts")),
			Some(CALL_DEPOSIT)
		);
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(2), call("upgrade_accounts")));
	});
}

#[test]
fn zero_turns_pausing_off() {
	new_test_ext().execute_with(|| {
		CallDepositAmount::set(0);
		assert_eq!(KillSwitch::call_deposit(&full_name("Balances", "upgrade_accounts")), None);
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), call("upgrade_accounts")),
			Error::<Test>::NotPausable
		);
		// Groups still work.
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));

		GroupDeposit::set(0);
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), burn()),
			Error::<Test>::NotPausable
		);
		assert!(KillSwitch::pause_groups().iter().all(|info| info.deposit.is_none()));
		// Paused before it was turned off, still ends normally and gets refunded.
		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(2), transfer()),
			Error::<Test>::NotPausable
		);
		run_to_relay(1 + PAUSE_DURATION);
		assert_eq!(held(1), 0);

		// Root can still pause.
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), burn(), None));

		GroupDeposit::set(DEPOSIT);
		PauseLength::set(0);
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), transfer()),
			Error::<Test>::NotPausable
		);
		assert_ok!(KillSwitch::do_try_state());
	});
}

#[test]
fn slashing_a_forced_pause_does_nothing_extra() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), None));
		assert_ok!(KillSwitch::unpause(RuntimeOrigin::root(), transfer(), true));

		System::assert_has_event(Event::Unpaused { target: transfer(), slashed: false }.into());
		assert_eq!(Cooldowns::<Test>::iter().count(), 0);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
	});
}
