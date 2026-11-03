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

fn transfer() -> RuntimeCallNameOf {
	full_name("Balances", "transfer_allow_death")
}

type RuntimeCallNameOf = pallet_tx_pause::RuntimeCallNameOf<Test>;

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
		assert_eq!(pause.deposits.to_vec(), vec![0]);
		assert_eq!(Deposits::<Test>::get(0).unwrap().who, 1);
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
		// No price.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), full_name("System", "remark")),
			Error::<Test>::NotPausable
		);
		// Priced, but tx-pause refuses.
		assert_noop!(
			KillSwitch::pause(
				RuntimeOrigin::signed(1),
				full_name("Balances", "transfer_keep_alive")
			),
			Error::<Test>::NotPausable
		);
		// The kill switch can never be paused.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(1), full_name("KillSwitch", "unpause")),
			Error::<Test>::NotPausable
		);
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
		assert_noop!(TxPause::pause(RuntimeOrigin::root(), transfer()), DispatchError::BadOrigin);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_noop!(TxPause::unpause(RuntimeOrigin::root(), transfer()), DispatchError::BadOrigin);
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
		assert_eq!(ExpiryCount::<Test>::get(), 0);
		assert!(ExpiryCursor::<Test>::get().is_none());
		System::assert_has_event(Event::Expired { target: transfer() }.into());

		// The deposit stays slashable for a while.
		assert_eq!(held(1), DEPOSIT);
		assert_noop!(
			KillSwitch::release_deposit(RuntimeOrigin::signed(3), 0),
			Error::<Test>::TooEarly
		);
		// Counted from when the pause ran out, not from when the deposit was made.
		run_to_relay(1 + PAUSE_DURATION + RELEASE_DELAY - 1);
		assert_noop!(
			KillSwitch::release_deposit(RuntimeOrigin::signed(3), 0),
			Error::<Test>::TooEarly
		);
		run_to_relay(1 + PAUSE_DURATION + RELEASE_DELAY);
		assert_ok!(KillSwitch::release_deposit(RuntimeOrigin::signed(3), 0));
		assert_eq!(held(1), 0);
		assert_eq!(Balances::free_balance(1), 1_000);
		assert_noop!(
			KillSwitch::release_deposit(RuntimeOrigin::signed(3), 0),
			Error::<Test>::UnknownDeposit
		);
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
	});
}

#[test]
fn on_poll_respects_max_steps() {
	new_test_ext().execute_with(|| {
		let calls = [
			"transfer_allow_death",
			"force_transfer",
			"transfer_all",
			"burn",
			"force_unreserve",
			"force_set_balance",
		];
		for call in calls {
			assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), full_name("Balances", call)));
		}
		assert_eq!(ExpiryCount::<Test>::get(), 6);

		// All 6 are due at 11, but one block only gets MAX_POLL_STEPS (5) of them.
		run_to_relay(20);
		assert_eq!(Pauses::<Test>::iter().count(), 1);
		run_to_relay(20);
		assert_eq!(Pauses::<Test>::iter().count(), 0);
		assert_eq!(ExpiryCount::<Test>::get(), 0);
		assert!(ExpiryCursor::<Test>::get().is_none());
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
	});
}

#[test]
fn can_expire_manually() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_noop!(
			KillSwitch::expire(RuntimeOrigin::signed(3), transfer()),
			Error::<Test>::NotExpired
		);

		// The relay chain moves on but the hook doesn't run.
		RelayBlock::set(1 + PAUSE_DURATION);
		assert_ok!(KillSwitch::expire(RuntimeOrigin::signed(3), transfer()));
		assert!(transfer_works());
		assert_eq!(ExpiryCount::<Test>::get(), 0);
		assert_eq!(held(1), DEPOSIT);
		assert_noop!(
			KillSwitch::expire(RuntimeOrigin::signed(3), transfer()),
			Error::<Test>::NotPaused
		);
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
		assert_eq!(pause.deposits.to_vec(), vec![0, 1, 2]);
		assert_eq!((held(1), held(2), held(3)), (DEPOSIT, DEPOSIT, DEPOSIT));
		assert_eq!(ExpiryCount::<Test>::get(), 1);

		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(4), transfer()),
			Error::<Test>::TooManyDeposits
		);

		// The old end doesn't do anything anymore.
		run_to_relay(1 + PAUSE_DURATION);
		assert!(!transfer_works());
		run_to_relay(1 + 3 * PAUSE_DURATION);
		assert!(transfer_works());
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
			Error::<Test>::OpenEnded
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
		assert_eq!(Deposits::<Test>::iter().count(), 0);
		assert_eq!(ExpiryCount::<Test>::get(), 0);
		System::assert_has_event(
			Event::CooldownStarted { target: transfer(), until: 1 + COOLDOWN }.into(),
		);
		System::assert_has_event(Event::Unpaused { target: transfer(), slashed: true }.into());

		// Nobody can pause it again until the cooldown is over.
		assert_noop!(
			KillSwitch::pause(RuntimeOrigin::signed(3), transfer()),
			Error::<Test>::InCooldown
		);
		RelayBlock::set(1 + COOLDOWN);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(3), transfer()));
		assert!(Cooldowns::<Test>::get(transfer()).is_none());
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
		assert!(Cooldowns::<Test>::get(transfer()).is_none());

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
fn fellowship_extension_is_capped() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		let until = 1 + PAUSE_DURATION;

		assert_noop!(
			KillSwitch::force_extend(fellowship(), transfer(), until),
			Error::<Test>::NotLater
		);
		assert_ok!(KillSwitch::force_extend(fellowship(), transfer(), until + 10));
		assert_noop!(
			KillSwitch::force_extend(
				fellowship(),
				transfer(),
				until + MAX_FELLOWSHIP_EXTENSION + 1
			),
			Error::<Test>::ExtensionTooLong
		);
		assert_ok!(KillSwitch::force_extend(
			fellowship(),
			transfer(),
			until + MAX_FELLOWSHIP_EXTENSION
		));

		// Root has no cap.
		assert_ok!(KillSwitch::force_extend(RuntimeOrigin::root(), transfer(), until + 1_000));
		assert_eq!(Pauses::<Test>::get(transfer()).unwrap().until, Some(until + 1_000));
		// And it doesn't count against the Fellowship's cap.
		assert_eq!(
			Pauses::<Test>::get(transfer()).unwrap().fellowship_extension,
			MAX_FELLOWSHIP_EXTENSION
		);
		assert_eq!(ExpiryCount::<Test>::get(), 1);

		assert_noop!(
			KillSwitch::force_extend(RuntimeOrigin::signed(1), transfer(), until + 2_000),
			DispatchError::BadOrigin
		);
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
		assert_eq!(ExpiryCount::<Test>::get(), 0);

		run_to_relay(1_000);
		assert!(!transfer_works());
		assert_noop!(
			KillSwitch::expire(RuntimeOrigin::signed(3), transfer()),
			Error::<Test>::OpenEnded
		);
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
fn can_slash_deposit_of_expired_pause() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_noop!(KillSwitch::slash_deposit(fellowship(), 0), Error::<Test>::StillPaused);
		assert_noop!(
			KillSwitch::release_deposit(RuntimeOrigin::signed(3), 0),
			Error::<Test>::StillPaused
		);

		run_to_relay(1 + PAUSE_DURATION);
		assert_noop!(
			KillSwitch::slash_deposit(RuntimeOrigin::signed(1), 0),
			DispatchError::BadOrigin
		);
		assert_ok!(KillSwitch::slash_deposit(fellowship(), 0));
		assert_eq!(held(1), 0);
		assert_eq!(Balances::free_balance(TREASURY), DEPOSIT);
		assert_noop!(KillSwitch::slash_deposit(fellowship(), 0), Error::<Test>::UnknownDeposit);
		// Slashing an old deposit doesn't start a cooldown.
		assert!(Cooldowns::<Test>::get(transfer()).is_none());
	});
}

#[test]
fn old_deposit_is_not_part_of_new_pause() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		run_to_relay(1 + PAUSE_DURATION);
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), transfer()));

		// Deposit 0 is from the pause that ran out, deposit 1 backs the new one.
		assert_ok!(KillSwitch::slash_deposit(fellowship(), 0));
		assert_noop!(KillSwitch::slash_deposit(fellowship(), 1), Error::<Test>::StillPaused);

		// Refunding the new pause only touches its own deposit.
		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), false));
		assert_eq!(Balances::free_balance(2), 1_000);
		assert_eq!(Balances::free_balance(1), 1_000 - DEPOSIT);
	});
}

#[test]
fn expiry_queue_is_cleaned_up() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(2), full_name("Balances", "burn")));
		assert_eq!(ExpiryCursor::<Test>::get(), Some(1 + PAUSE_DURATION));

		assert_ok!(KillSwitch::unpause(fellowship(), transfer(), false));
		assert_eq!(ExpiryCount::<Test>::get(), 1);
		assert!(ExpiryCursor::<Test>::get().is_some());

		assert_ok!(KillSwitch::unpause(fellowship(), full_name("Balances", "burn"), false));
		assert_eq!(ExpiryCount::<Test>::get(), 0);
		assert!(ExpiryCursor::<Test>::get().is_none());
		assert_eq!(Expiries::<Test>::iter().count(), 0);
	});
}

#[test]
fn release_delay_starts_when_pause_runs_out() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		// Root keeps it going for much longer than the release delay.
		assert_ok!(KillSwitch::force_extend(RuntimeOrigin::root(), transfer(), 500));
		run_to_relay(500);
		assert!(transfer_works());

		// The deposit was made at 1, but it only becomes free 100 blocks after the pause ended.
		assert_noop!(
			KillSwitch::release_deposit(RuntimeOrigin::signed(1), 0),
			Error::<Test>::TooEarly
		);
		assert_ok!(KillSwitch::slash_deposit(fellowship(), 0));
		assert_eq!(Balances::free_balance(TREASURY), DEPOSIT);
	});
}

#[test]
fn fails_to_extend_pause_that_ran_out() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::pause(RuntimeOrigin::signed(1), transfer()));
		// The relay chain moves past the end, but the hook hasn't run yet.
		RelayBlock::set(1 + PAUSE_DURATION);

		assert_noop!(
			KillSwitch::extend(RuntimeOrigin::signed(2), transfer()),
			Error::<Test>::AlreadyExpired
		);
		assert_noop!(
			KillSwitch::force_extend(fellowship(), transfer(), 20),
			Error::<Test>::AlreadyExpired
		);
		// Root can still keep it going.
		assert_ok!(KillSwitch::force_extend(RuntimeOrigin::root(), transfer(), 20));
	});
}

#[test]
fn fellowship_cannot_touch_root_pause() {
	new_test_ext().execute_with(|| {
		assert_ok!(KillSwitch::force_pause(RuntimeOrigin::root(), transfer(), Some(50)));
		// Someone backs it with a deposit, that doesn't make it the Fellowship's to manage.
		assert_ok!(KillSwitch::extend(RuntimeOrigin::signed(1), transfer()));

		assert_noop!(KillSwitch::unpause(fellowship(), transfer(), true), DispatchError::BadOrigin);
		assert_noop!(
			KillSwitch::force_extend(fellowship(), transfer(), 55),
			DispatchError::BadOrigin
		);

		assert_ok!(KillSwitch::unpause(RuntimeOrigin::root(), transfer(), false));
		assert!(transfer_works());
		assert_eq!(Balances::free_balance(1), 1_000);
	});
}
