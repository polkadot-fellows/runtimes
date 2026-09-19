// Copyright (C) Polkadot Fellows.
// This file is part of Polkadot.

// Polkadot is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Polkadot is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

use crate::{
	mock::*, CtMigratorCall, CtRuntimeCall, Error, Event, Manager, MigrationStage, Paused,
	RcMigrationStage,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;
use xcm::prelude::*;

type Stage = MigrationStage<AccountId, u64, u64>;

fn stage() -> Stage {
	RcMigrationStage::<Test>::get()
}

/// Put the machine at `stage` directly.
fn set_stage(stage: Stage) {
	RcMigrationStage::<Test>::put(stage);
}

/// The data stages in the order the machine walks them, between the warm-up and the cool-off.
fn data_stages() -> Vec<Stage> {
	vec![
		Stage::AccountsInit,
		Stage::AccountsOngoing { last_key: None },
		Stage::AccountsDone,
		Stage::ProxyInit,
		Stage::ProxyOngoing { last_key: None },
		Stage::ProxyDone,
		Stage::RegistrarInit,
		Stage::RegistrarOngoing { last_key: None },
		Stage::RegistrarDone,
		Stage::HrmpInit,
		Stage::HrmpOngoing { last_key: None },
		Stage::HrmpDone,
		Stage::Sweep,
		Stage::SweepDust { last_key: None },
		Stage::TiCorrection,
	]
}

/// Blocks from the end of the warm-up to the opening of the cool-off: one per data stage.
fn data_stage_blocks() -> u64 {
	data_stages().len() as u64
}

fn assert_stage(expected: Stage) {
	assert_eq!(stage(), expected);
}

/// The stage transitions emitted so far, oldest first.
fn transitions() -> Vec<(Stage, Stage)> {
	System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Rc2Migrator(Event::StageTransition { old, new }) => Some((old, new)),
			_ => None,
		})
		.collect()
}

#[test]
fn a_pending_migration_does_nothing_at_all() {
	// GIVEN a runtime that has never scheduled a migration.
	new_test_ext().execute_with(|| {
		assert_stage(Stage::Pending);

		// WHEN blocks pass.
		run_blocks(20);

		// THEN the machine has not moved, nothing was sent, and no event was emitted.
		assert_stage(Stage::Pending);
		assert_eq!(sent().len(), 0);
		assert_eq!(transitions(), vec![]);
	});
}

#[test]
fn the_admin_origin_or_the_manager_schedules_and_only_into_the_future() {
	new_test_ext().execute_with(|| {
		// WHEN a signed account with no appointment tries to schedule. THEN it is refused.
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::signed(ALICE), 10, WARM_UP, COOL_OFF),
			BadOrigin
		);
		// Being the Coretime chain confers no scheduling power either.
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::signed(CORETIME), 10, WARM_UP, COOL_OFF),
			BadOrigin
		);

		// WHEN root schedules the present moment or earlier. THEN it is refused: a start already
		// past would begin the migration on the very next block.
		let now = now_ms();
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now, WARM_UP, COOL_OFF),
			Error::<Test>::StartInPast
		);
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now - 1, WARM_UP, COOL_OFF),
			Error::<Test>::StartInPast
		);

		// WHEN root schedules a future moment. THEN the machine is armed.
		let start = now + 5 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));
		assert_stage(Stage::Scheduled { start });

		// WHEN root schedules again. THEN it is refused, so a second governance call cannot
		// silently move a start date that is already committed.
		assert_noop!(
			Rc2Migrator::schedule_migration(
				RuntimeOrigin::root(),
				start + BLOCK_TIME_MS,
				WARM_UP,
				COOL_OFF
			),
			Error::<Test>::AlreadyScheduled
		);

		// GIVEN the machine pending again and Alice appointed manager.
		assert_ok!(Rc2Migrator::cancel_migration(RuntimeOrigin::root()));
		assert_stage(Stage::Pending);
		assert_ok!(Rc2Migrator::set_manager(RuntimeOrigin::root(), Some(ALICE)));

		// WHEN the manager schedules the present moment. THEN it is refused like it is for root.
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::signed(ALICE), now, WARM_UP, COOL_OFF),
			Error::<Test>::StartInPast
		);

		// WHEN the manager schedules a future moment. THEN the machine is armed: the manager has
		// the same scheduling power as the admin origin.
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::signed(ALICE),
			start,
			WARM_UP,
			COOL_OFF
		));
		assert_stage(Stage::Scheduled { start });
	});
}

#[test]
fn the_manager_drives_the_migration_but_cannot_appoint_one() {
	new_test_ext().execute_with(|| {
		// WHEN a signed account appoints itself. THEN it is refused.
		assert_noop!(
			Rc2Migrator::set_manager(RuntimeOrigin::signed(ALICE), Some(ALICE)),
			BadOrigin
		);

		// GIVEN Alice appointed manager by the admin origin.
		assert_ok!(Rc2Migrator::set_manager(RuntimeOrigin::root(), Some(ALICE)));
		assert_eq!(Manager::<Test>::get(), Some(ALICE));
		System::assert_last_event(Event::ManagerSet { old: None, new: Some(ALICE) }.into());

		// WHEN the manager schedules and cancels. THEN both are accepted.
		let start = now_ms() + 5 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::signed(ALICE),
			start,
			WARM_UP,
			COOL_OFF
		));
		assert_stage(Stage::Scheduled { start });
		assert_ok!(Rc2Migrator::cancel_migration(RuntimeOrigin::signed(ALICE)));
		assert_stage(Stage::Pending);

		// GIVEN a running machine. WHEN the manager pauses, forces and resumes. THEN all three are
		// accepted: the manager has the admin origin's powers over the machine.
		set_stage(Stage::WaitingForCt);
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::signed(ALICE)));
		assert_ok!(Rc2Migrator::force_set_stage(
			RuntimeOrigin::signed(ALICE),
			Stage::MigrationDone
		));
		assert_stage(Stage::MigrationDone);
		assert_ok!(Rc2Migrator::resume_migration(RuntimeOrigin::signed(ALICE)));

		// WHEN the manager appoints a manager. THEN it is refused, so the appointment stays with
		// the admin origin alone.
		assert_noop!(
			Rc2Migrator::set_manager(RuntimeOrigin::signed(ALICE), Some(CORETIME)),
			BadOrigin
		);

		// WHEN the admin origin appoints an account that something else references. THEN it is
		// refused: the migration reaps the manager account at the end.
		frame_system::Pallet::<Test>::inc_providers(&CORETIME);
		assert_ok!(frame_system::Pallet::<Test>::inc_consumers(&CORETIME));
		assert_noop!(
			Rc2Migrator::set_manager(RuntimeOrigin::root(), Some(CORETIME)),
			Error::<Test>::AccountReferenced
		);

		// WHEN the admin origin removes the manager. THEN Alice loses the powers.
		assert_ok!(Rc2Migrator::set_manager(RuntimeOrigin::root(), None));
		assert_eq!(Manager::<Test>::get(), None);
		System::assert_last_event(Event::ManagerSet { old: Some(ALICE), new: None }.into());
		assert_noop!(Rc2Migrator::cancel_migration(RuntimeOrigin::signed(ALICE)), BadOrigin);
	});
}

#[test]
fn a_scheduled_migration_can_be_cancelled_and_scheduled_again() {
	// GIVEN a scheduled migration.
	new_test_ext().execute_with(|| {
		let start = now_ms() + 5 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));

		// WHEN a signed account cancels. THEN it is refused.
		assert_noop!(Rc2Migrator::cancel_migration(RuntimeOrigin::signed(ALICE)), BadOrigin);
		assert_stage(Stage::Scheduled { start });

		// WHEN root cancels. THEN the machine is pending again and a new start can be set.
		assert_ok!(Rc2Migrator::cancel_migration(RuntimeOrigin::root()));
		assert_stage(Stage::Pending);
		let start = now_ms() + 2 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));
		assert_stage(Stage::Scheduled { start });

		// WHEN the start has passed and the Coretime chain has been signalled. THEN cancelling is
		// refused: the two chains have begun a handshake that this call cannot take back.
		run_blocks(3);
		assert_stage(Stage::WaitingForCt);
		assert_noop!(
			Rc2Migrator::cancel_migration(RuntimeOrigin::root()),
			Error::<Test>::NotScheduled
		);
	});
}

#[test]
fn a_scheduled_migration_starts_once_the_clock_passes_it_and_not_before() {
	// GIVEN a migration scheduled two blocks' worth of time ahead.
	new_test_ext().execute_with(|| {
		let start = now_ms() + 2 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));

		// WHEN the blocks before it pass. THEN nothing is sent and the stage holds: the relay
		// chain serves its users normally right up to the start.
		run_blocks(2);
		assert_stage(Stage::Scheduled { start });
		assert_eq!(sent().len(), 0);

		// WHEN a block sees a clock at or past `start`. THEN exactly one start signal goes to
		// the Coretime chain and the machine waits for the answer.
		run_blocks(1);
		assert_stage(Stage::WaitingForCt);
		assert_eq!(sent().len(), 1);
		assert_eq!(sent()[0].0, Location::new(0, [Parachain(CT_PARA_ID)]));
		assert_eq!(sent_call(0), CtRuntimeCall::CtMigrator(CtMigratorCall::StartMigration));
	});
}

#[test]
fn the_start_signal_is_the_message_the_coretime_chain_expects() {
	// GIVEN a migration whose start has just passed.
	new_test_ext().execute_with(|| {
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));

		// WHEN the machine sends its start signal.
		run_blocks(2);

		// THEN the message is what CT expects
		assert_eq!(
			sent(),
			vec![(
				Location::new(0, [Parachain(CT_PARA_ID)]),
				Xcm(vec![
					// unpaid execution so the Coretime chain's barrier lets it in
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						// call arrives with the root origin
						origin_kind: OriginKind::Superuser,
						fallback_max_weight: None,
						call: CtRuntimeCall::CtMigrator(CtMigratorCall::StartMigration)
							.encode()
							.into(),
					},
					// so a refused call fails the message rather than vanishing
					ExpectTransactStatus(MaybeErrorCode::Success),
				]),
			)]
		);
	});
}

#[test]
fn waiting_for_coretime_never_advances_on_its_own() {
	// GIVEN a machine that has sent its start signal.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForCt);

		// WHEN many blocks pass without an answer.
		run_blocks(50);

		// THEN it is still waiting. There is no timeout.
		assert_stage(Stage::WaitingForCt);
		assert_eq!(sent().len(), 0);
	});
}

#[test]
fn the_coretime_chain_or_the_operator_confirms_readiness() {
	// GIVEN a machine waiting for the Coretime chain.
	new_test_ext().execute_with(|| {
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));
		run_blocks(2);
		assert_stage(Stage::WaitingForCt);

		// WHEN an unrelated account confirms. THEN it is refused.
		assert_noop!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(ALICE)), BadOrigin);
		assert_stage(Stage::WaitingForCt);

		// WHEN the Coretime chain confirms. THEN the machine warms up.
		let now = System::block_number();
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		assert_stage(Stage::WarmUp { end_at: now + WARM_UP });

		// WHEN it confirms again, as it does after a re-run handshake. THEN nothing changes: the
		// warm-up keeps its end block and no second transition is recorded.
		let transitions_so_far = transitions().len();
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		assert_stage(Stage::WarmUp { end_at: now + WARM_UP });
		assert_eq!(transitions().len(), transitions_so_far);

		// WHEN the admin origin and the manager confirm, as they may to stand in for a reply that
		// never arrived. THEN both are accepted.
		assert_ok!(Rc2Migrator::set_manager(RuntimeOrigin::root(), Some(ALICE)));
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::root()));
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(ALICE)));
		assert_stage(Stage::WarmUp { end_at: now + WARM_UP });
		assert_eq!(transitions().len(), transitions_so_far);
	});
}

#[test]
fn readiness_outside_the_handshake_is_rejected() {
	new_test_ext().execute_with(|| {
		// WHEN the Coretime chain confirms readiness for a migration that was never started.
		// THEN it is refused, so a stray message cannot start a migration by itself.
		assert_noop!(
			Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)),
			Error::<Test>::NotWaitingForCt
		);
		assert_stage(Stage::Pending);

		// GIVEN a migration that has already finished. THEN the same refusal.
		set_stage(Stage::MigrationDone);
		assert_noop!(
			Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)),
			Error::<Test>::NotWaitingForCt
		);
	});
}

#[test]
fn cool_off_holds_for_its_period_and_then_finishes() {
	// GIVEN a machine in its verification window.
	new_test_ext().execute_with(|| {
		let end_at = System::block_number() + COOL_OFF;
		set_stage(Stage::CoolOff { end_at });

		// WHEN the window has not elapsed. THEN nothing happens: the window exists so the end
		// state can be inspected while the migration's filters are still engaged.
		run_blocks(COOL_OFF - 1);
		assert_stage(Stage::CoolOff { end_at });
		assert_eq!(sent().len(), 0);

		// WHEN it elapses. THEN the completion signal is sent and both chains are done.
		run_blocks(1);
		assert_stage(Stage::MigrationDone);
		assert_eq!(sent().len(), 1);
		assert_eq!(sent_call(0), CtRuntimeCall::CtMigrator(CtMigratorCall::EndLockdown));

		// WHEN more blocks pass. THEN a finished migration stays finished and sends nothing more.
		run_blocks(10);
		assert_stage(Stage::MigrationDone);
		assert_eq!(sent().len(), 1);
	});
}

#[test]
fn a_failed_send_leaves_the_stage_alone_for_a_retry() {
	// GIVEN a scheduled migration and a router that refuses everything.
	new_test_ext().execute_with(|| {
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));
		SendFails::set(true);

		// WHEN the start passes. THEN the machine has not advanced and nothing was sent: a lost
		// start signal must not leave the relay chain believing the handshake is open.
		run_blocks(5);
		assert_stage(Stage::Scheduled { start });
		assert_eq!(sent().len(), 0);

		// WHEN the router recovers. THEN the same step runs and the machine advances.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::WaitingForCt);
		assert_eq!(sent().len(), 1);

		// GIVEN a machine in its verification window. WHEN the completion signal is the one that
		// cannot be sent. THEN the window stays open rather than closing on a signal the Coretime
		// chain never received.
		let end_at = System::block_number() + COOL_OFF;
		set_stage(Stage::CoolOff { end_at });
		SendFails::set(true);
		run_blocks(COOL_OFF + 5);
		assert_stage(Stage::CoolOff { end_at });
		assert_eq!(sent().len(), 1);

		// WHEN the router recovers. THEN the completion goes out and the machine closes.
		SendFails::set(false);
		run_blocks(1);
		assert_stage(Stage::MigrationDone);
		assert_eq!(sent_call(1), CtRuntimeCall::CtMigrator(CtMigratorCall::EndLockdown));
	});
}

#[test]
fn force_set_stage_needs_a_pause_and_the_admins_powers() {
	// GIVEN a machine waiting on the Coretime chain, not paused.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForCt);

		// WHEN root forces a stage while the machine runs. THEN it is refused: the machine is
		// moved by hand only while it is paused.
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::MigrationDone),
			Error::<Test>::NotPaused
		);

		// GIVEN it is paused.
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::root()));

		// WHEN an origin without the admin's powers forces a stage. THEN it is refused.
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::signed(ALICE), Stage::MigrationDone),
			BadOrigin
		);
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::signed(CORETIME), Stage::MigrationDone),
			BadOrigin
		);

		// WHEN root forces stages. THEN it may move anywhere, including backwards
		for target in [
			Stage::AccountsOngoing { last_key: None },
			Stage::MigrationDone,
			Stage::Scheduled { start: 99 * BLOCK_TIME_MS },
			Stage::Pending,
			Stage::WaitingForCt,
		] {
			assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), target.clone()));
			assert_stage(target);
		}

		// WHEN it is resumed. THEN the machine continues from the forced stage and the hatch
		// closes again.
		assert_ok!(Rc2Migrator::resume_migration(RuntimeOrigin::root()));
		assert_stage(Stage::WaitingForCt);
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::MigrationDone),
			Error::<Test>::NotPaused
		);
	});
}

#[test]
fn a_running_migration_can_be_paused_and_resumed_where_it_stopped() {
	new_test_ext().execute_with(|| {
		let start = now_ms() + BLOCK_TIME_MS;

		// WHEN a migration that is not running is paused. THEN it is refused at every such stage:
		// nothing to halt before the start, cancel is the tool while scheduled, nothing left after.
		for not_running in [Stage::Pending, Stage::Scheduled { start }, Stage::MigrationDone] {
			set_stage(not_running);
			assert_noop!(
				Rc2Migrator::pause_migration(RuntimeOrigin::root()),
				Error::<Test>::NotRunning
			);
		}
		// WHEN a migration that is not paused is resumed. THEN it is refused.
		assert_noop!(
			Rc2Migrator::resume_migration(RuntimeOrigin::root()),
			Error::<Test>::NotPaused
		);

		// GIVEN a scheduled migration that has reached the handshake.
		set_stage(Stage::Pending);
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));
		run_blocks(2);
		assert_stage(Stage::WaitingForCt);
		let sent_so_far = sent().len();

		// WHEN an origin without the admin's powers pauses it. THEN it is refused.
		assert_noop!(Rc2Migrator::pause_migration(RuntimeOrigin::signed(ALICE)), BadOrigin);

		// WHEN the manager pauses it. THEN the machine halts and the stage is left where it was.
		assert_ok!(Rc2Migrator::set_manager(RuntimeOrigin::root(), Some(ALICE)));
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::signed(ALICE)));
		assert!(Paused::<Test>::get());
		assert_stage(Stage::WaitingForCt);
		System::assert_last_event(Event::MigrationPaused { stage: Stage::WaitingForCt }.into());

		// WHEN the Coretime chain confirms during the pause. THEN the fact is recorded — the
		// reply is not lost — but the machine still does not move: the warm-up never counts down.
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		let end_at = System::block_number() + WARM_UP;
		assert_stage(Stage::WarmUp { end_at });
		run_blocks(WARM_UP + 5);
		assert_stage(Stage::WarmUp { end_at });
		assert_eq!(sent().len(), sent_so_far);
		assert!(stage().is_ongoing());
		assert!(!stage().is_finished());

		// WHEN it is paused again. THEN that is refused.
		assert_noop!(
			Rc2Migrator::pause_migration(RuntimeOrigin::root()),
			Error::<Test>::AlreadyPaused
		);

		// WHEN the manager resumes. THEN the machine continues from where it stood: the warm-up
		// has already expired, so the very next block moves on.
		assert_ok!(Rc2Migrator::resume_migration(RuntimeOrigin::signed(ALICE)));
		assert!(!Paused::<Test>::get());
		System::assert_last_event(
			Event::MigrationResumed { stage: Stage::WarmUp { end_at } }.into(),
		);
		run_blocks(1);
		assert_stage(Stage::AccountsInit);
	});
}

#[test]
fn the_machine_runs_from_pending_to_done() {
	// GIVEN a scheduled migration and a Coretime chain that answers.
	new_test_ext().execute_with(|| {
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));

		// WHEN the start passes.
		run_blocks(2);
		assert_stage(Stage::WaitingForCt);

		// WHEN the Coretime chain confirms. THEN the machine warms up: both chains are locked
		// down and nothing is sent yet.
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		let warm_up_end = System::block_number() + WARM_UP;
		assert_stage(Stage::WarmUp { end_at: warm_up_end });

		// WHEN the warm-up elapses. THEN the data stages run, one block each, and nothing is sent
		// because none of them carries data yet.
		run_blocks(WARM_UP);
		assert_stage(Stage::AccountsInit);
		run_blocks(data_stage_blocks());
		assert_eq!(sent().len(), 1, "no data stage may send before it is filled in");

		// THEN the verification window opens.
		let end_at = System::block_number() + COOL_OFF;
		assert_stage(Stage::CoolOff { end_at });

		// WHEN the verification window elapses.
		run_blocks(COOL_OFF);

		// THEN the migration is done, and the two signals it sent are the start and the
		// completion, in that order.
		assert_stage(Stage::MigrationDone);
		assert_eq!(
			(0..sent().len()).map(sent_call).collect::<Vec<_>>(),
			vec![
				CtRuntimeCall::CtMigrator(CtMigratorCall::StartMigration),
				CtRuntimeCall::CtMigrator(CtMigratorCall::EndLockdown),
			]
		);

		// THEN every stage was walked, in this order and no other.
		let mut path = vec![
			Stage::Pending,
			Stage::Scheduled { start },
			Stage::WaitingForCt,
			Stage::WarmUp { end_at: warm_up_end },
		];
		path.extend(data_stages());
		path.extend([Stage::CoolOff { end_at }, Stage::MigrationDone]);
		let expected: Vec<(Stage, Stage)> =
			path.windows(2).map(|w| (w[0].clone(), w[1].clone())).collect();
		assert_eq!(transitions(), expected);
	});
}

#[test]
fn the_scheduled_windows_are_the_ones_the_machine_holds_for() {
	// GIVEN a migration scheduled with windows shorter than the ones the other tests use.
	new_test_ext().execute_with(|| {
		let short_warm_up = WARM_UP - 2;
		let short_cool_off = COOL_OFF - 5;
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			short_warm_up,
			short_cool_off
		));

		// WHEN the handshake completes. THEN the warm-up runs for the scheduled window.
		run_blocks(2);
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		assert_stage(Stage::WarmUp { end_at: System::block_number() + short_warm_up });

		// WHEN it and the data stages elapse. THEN the cool-off runs for the scheduled window too.
		run_blocks(short_warm_up + data_stage_blocks());
		assert_stage(Stage::CoolOff { end_at: System::block_number() + short_cool_off });

		// WHEN it elapses. THEN the machine is done.
		run_blocks(short_cool_off);
		assert_stage(Stage::MigrationDone);
	});
}

#[test]
fn the_warm_up_holds_for_its_period_and_can_be_halted() {
	// GIVEN a machine the Coretime chain has just confirmed.
	new_test_ext().execute_with(|| {
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			start,
			WARM_UP,
			COOL_OFF
		));
		run_blocks(2);
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		let end_at = System::block_number() + WARM_UP;
		let sent_so_far = sent().len();

		// WHEN the blocks before the warm-up ends pass. THEN the stage holds and nothing is sent:
		// the window exists so the queues drain before any data moves.
		run_blocks(WARM_UP - 1);
		assert_stage(Stage::WarmUp { end_at });
		assert_eq!(sent().len(), sent_so_far);

		// WHEN an operator halts inside the window. THEN the machine stops there, having sent
		// nothing.
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::root()));
		run_blocks(WARM_UP);
		assert_stage(Stage::WarmUp { end_at });
		assert_eq!(sent().len(), sent_so_far);
	});
}

#[test]
fn the_stage_predicates_say_what_their_consumers_need() {
	fn expected(stage: &Stage) -> (bool, bool, bool) {
		match stage {
			//                              ongoing, started, finished
			Stage::Pending => (false, false, false),
			Stage::Scheduled { .. } => (false, false, false),
			Stage::WaitingForCt => (true, true, false),
			Stage::WarmUp { .. } => (true, true, false),
			Stage::AccountsInit => (true, true, false),
			Stage::AccountsOngoing { .. } => (true, true, false),
			Stage::AccountsDone => (true, true, false),
			Stage::ProxyInit => (true, true, false),
			Stage::ProxyOngoing { .. } => (true, true, false),
			Stage::ProxyDone => (true, true, false),
			Stage::RegistrarInit => (true, true, false),
			Stage::RegistrarOngoing { .. } => (true, true, false),
			Stage::RegistrarDone => (true, true, false),
			Stage::HrmpInit => (true, true, false),
			Stage::HrmpOngoing { .. } => (true, true, false),
			Stage::HrmpDone => (true, true, false),
			Stage::Sweep => (true, true, false),
			Stage::SweepDust { .. } => (true, true, false),
			Stage::TiCorrection => (true, true, false),
			Stage::CoolOff { .. } => (true, true, false),
			// `has_started` stays true after the end: what closed when it started must not reopen.
			Stage::MigrationDone => (false, true, true),
		}
	}

	let cases = [
		Stage::Pending,
		Stage::Scheduled { start: 10 },
		Stage::WaitingForCt,
		Stage::WarmUp { end_at: 10 },
		Stage::AccountsInit,
		Stage::AccountsOngoing { last_key: None },
		Stage::AccountsDone,
		Stage::ProxyInit,
		Stage::ProxyOngoing { last_key: None },
		Stage::ProxyDone,
		Stage::RegistrarInit,
		Stage::RegistrarOngoing { last_key: None },
		Stage::RegistrarDone,
		Stage::HrmpInit,
		Stage::HrmpOngoing { last_key: None },
		Stage::HrmpDone,
		Stage::Sweep,
		Stage::SweepDust { last_key: None },
		Stage::TiCorrection,
		Stage::CoolOff { end_at: 10 },
		Stage::MigrationDone,
	];

	for stage in cases {
		let (ongoing, started, finished) = expected(&stage);
		assert_eq!(stage.is_ongoing(), ongoing, "is_ongoing for {stage:?}");
		assert_eq!(stage.has_started(), started, "has_started for {stage:?}");
		assert_eq!(stage.is_finished(), finished, "is_finished for {stage:?}");
	}
}
