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
	mock::*, xcm_config::NothingOnceDataMigrationStarts, AssetTrap, AssetTrapRejection,
	BlockLimits, CtMigratorCall, CtRuntimeCall, Error, Event, Manager, MigratedBalances,
	MigrationPhase, Paused, RcMigratedBalance, RcMigrationStage, TakenAssetTraps,
};
use codec::Encode;
use frame_support::{
	assert_noop, assert_ok,
	traits::{ContainsPair, Everything},
};
use sp_runtime::{traits::Dispatchable, DispatchError::BadOrigin};
use xcm::prelude::*;

fn stage() -> Stage {
	RcMigrationStage::<Test>::get()
}

/// Put the machine at `stage` directly.
fn set_stage(stage: Stage) {
	RcMigrationStage::<Test>::put(stage);
}

/// The data stages in the order the machine walks them, between the wait for the asset traps and
/// the cool-off.
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
		Stage::PayAssetTraps,
		Stage::Sweep,
		Stage::SweepDust { last_key: None },
		Stage::TiCorrection,
	]
}

/// Every stage, in the order the machine walks them.
fn all_stages() -> Vec<Stage> {
	[
		Stage::Pending,
		Stage::Scheduled { start: 10 },
		Stage::WaitingForCt,
		Stage::WarmUp { end_at: 10 },
		Stage::WaitingForAssetTraps,
	]
	.into_iter()
	.chain(data_stages())
	.chain([Stage::CoolOff { end_at: 10 }, Stage::MigrationDone])
	.collect()
}

/// Blocks from the end of the wait for the asset traps to the opening of the cool-off: one per data
/// stage.
fn data_stage_blocks() -> u64 {
	data_stages().len() as u64
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
		assert_ok!(Rc2Migrator::set_manager(RuntimeOrigin::signed(ADMIN), Some(ALICE)));
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
			Stage::WarmUp { end_at: 10 }
		));
		assert_stage(Stage::WarmUp { end_at: 10 });
		assert_ok!(Rc2Migrator::resume_migration(RuntimeOrigin::signed(ALICE)));

		// WHEN the manager appoints a manager. THEN it is refused, so the appointment stays with
		// the admin origin alone.
		assert_noop!(
			Rc2Migrator::set_manager(RuntimeOrigin::signed(ALICE), Some(CORETIME)),
			BadOrigin
		);

		// WHEN root appoints an account that something else references. THEN it is refused: the
		// migration reaps the manager account at the end.
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

		// WHEN root forces stages within the run. THEN it may move anywhere, including backwards,
		// and the pause holds throughout.
		for target in [
			Stage::AccountsOngoing { last_key: None },
			Stage::CoolOff { end_at: 50 },
			Stage::WarmUp { end_at: 20 },
			Stage::WaitingForCt,
		] {
			assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), target.clone()));
			assert_stage(target);
			assert!(Paused::<Test>::get());
		}

		// WHEN it is resumed. THEN the machine continues from the forced stage and the hatch
		// closes again.
		assert_ok!(Rc2Migrator::resume_migration(RuntimeOrigin::root()));
		assert_stage(Stage::WaitingForCt);
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::MigrationDone),
			Error::<Test>::NotPaused
		);

		// WHEN it is paused and forced out of the run, back to pending. THEN the pause goes with
		// it: nothing to resume, and a fresh schedule runs without any further step.
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::root()));
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::Pending));
		assert!(!Paused::<Test>::get());
		assert_noop!(
			Rc2Migrator::resume_migration(RuntimeOrigin::root()),
			Error::<Test>::NotPaused
		);
		assert_ok!(Rc2Migrator::schedule_migration(
			RuntimeOrigin::root(),
			now_ms() + BLOCK_TIME_MS,
			WARM_UP,
			COOL_OFF
		));
		run_blocks(2);
		assert_stage(Stage::WaitingForCt);
		assert_eq!(sent().len(), 1);

		// WHEN it is paused and forced to a schedule whose start has passed. THEN it starts on
		// the next block.
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::root()));
		assert_ok!(Rc2Migrator::force_set_stage(
			RuntimeOrigin::root(),
			Stage::Scheduled { start: now_ms() }
		));
		assert!(!Paused::<Test>::get());
		run_blocks(1);
		assert_stage(Stage::WaitingForCt);
		assert_eq!(sent().len(), 2);

		// WHEN it is paused and forced to done. THEN the pause is gone and neither pause nor
		// resume has anything to act on.
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::root()));
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::MigrationDone));
		assert!(!Paused::<Test>::get());
		assert_noop!(
			Rc2Migrator::resume_migration(RuntimeOrigin::root()),
			Error::<Test>::NotPaused
		);
		assert_noop!(
			Rc2Migrator::pause_migration(RuntimeOrigin::root()),
			Error::<Test>::NotRunning
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
		assert_stage(Stage::WaitingForAssetTraps);
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

		// WHEN the warm-up elapses. THEN the machine waits for the asset traps.
		run_blocks(WARM_UP);
		assert_stage(Stage::WaitingForAssetTraps);

		// WHEN the manager says there are no more. THEN the data stages run, one block each, and
		// nothing is sent because none of them carries data yet.
		assert_ok!(Rc2Migrator::submit_asset_traps(RuntimeOrigin::root(), vec![], true));
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
			Stage::WaitingForAssetTraps,
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

		// WHEN it, the wait for the asset traps and the data stages pass. THEN the cool-off runs
		// for the scheduled window too.
		run_blocks(short_warm_up);
		assert_ok!(Rc2Migrator::submit_asset_traps(RuntimeOrigin::root(), vec![], true));
		run_blocks(data_stage_blocks());
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
	fn expected(stage: &Stage) -> MigrationPhase {
		// Exhaustive
		match stage {
			Stage::Pending | Stage::Scheduled { .. } => MigrationPhase::NotStarted,
			Stage::WaitingForCt | Stage::WarmUp { .. } => MigrationPhase::WarmUp,
			Stage::WaitingForAssetTraps |
			Stage::AccountsInit |
			Stage::AccountsOngoing { .. } |
			Stage::AccountsDone |
			Stage::ProxyInit |
			Stage::ProxyOngoing { .. } |
			Stage::ProxyDone |
			Stage::RegistrarInit |
			Stage::RegistrarOngoing { .. } |
			Stage::RegistrarDone |
			Stage::HrmpInit |
			Stage::HrmpOngoing { .. } |
			Stage::HrmpDone |
			Stage::PayAssetTraps |
			Stage::Sweep |
			Stage::SweepDust { .. } |
			Stage::TiCorrection |
			Stage::CoolOff { .. } => MigrationPhase::DataMigration,
			Stage::MigrationDone => MigrationPhase::Done,
		}
	}

	let cases = [
		Stage::Pending,
		Stage::Scheduled { start: 10 },
		Stage::WaitingForCt,
		Stage::WarmUp { end_at: 10 },
		Stage::WaitingForAssetTraps,
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
		Stage::PayAssetTraps,
		Stage::Sweep,
		Stage::SweepDust { last_key: None },
		Stage::TiCorrection,
		Stage::CoolOff { end_at: 10 },
		Stage::MigrationDone,
	];

	for stage in cases {
		let phase = expected(&stage);
		assert_eq!(stage.phase(), phase, "phase for {stage:?}");
		// What each phase means to the call filter, the barrier and the teleport filter. What
		// closed stays closed after the end.
		let (ongoing, started, data, finished) = match phase {
			MigrationPhase::NotStarted => (false, false, false, false),
			MigrationPhase::WarmUp => (true, true, false, false),
			MigrationPhase::DataMigration => (true, true, true, false),
			MigrationPhase::Done => (false, true, true, true),
		};
		assert_eq!(stage.is_ongoing(), ongoing, "is_ongoing for {stage:?}");
		assert_eq!(stage.has_started(), started, "has_started for {stage:?}");
		assert_eq!(stage.data_migration_started(), data, "data_migration_started for {stage:?}");
		assert_eq!(stage.is_finished(), finished, "is_finished for {stage:?}");
	}
}

#[test]
fn each_stage_allows_only_its_own_call_set() {
	new_test_ext().execute_with(|| {
		for stage in all_stages() {
			let allowed = match stage {
				Stage::Pending | Stage::Scheduled { .. } => PRE_MIGRATION,
				Stage::WaitingForCt |
				Stage::WarmUp { .. } |
				Stage::WaitingForAssetTraps |
				Stage::AccountsInit |
				Stage::AccountsOngoing { .. } |
				Stage::AccountsDone |
				Stage::ProxyInit |
				Stage::ProxyOngoing { .. } |
				Stage::ProxyDone |
				Stage::RegistrarInit |
				Stage::RegistrarOngoing { .. } |
				Stage::RegistrarDone |
				Stage::HrmpInit |
				Stage::HrmpOngoing { .. } |
				Stage::HrmpDone |
				Stage::PayAssetTraps |
				Stage::Sweep |
				Stage::SweepDust { .. } |
				Stage::TiCorrection |
				Stage::CoolOff { .. } => INTRA_MIGRATION,
				Stage::MigrationDone => POST_MIGRATION,
			};

			// GIVEN the machine at `stage`.
			set_stage(stage.clone());

			// THEN only the remark of that stage's call set passes.
			for tag in [PRE_MIGRATION, INTRA_MIGRATION, POST_MIGRATION] {
				let origin = RuntimeOrigin::signed(ALICE);
				if tag == allowed {
					assert_ok!(tagged_remark(tag).dispatch(origin));
				} else {
					assert_noop!(
						tagged_remark(tag).dispatch(origin).map_err(|e| e.error),
						frame_system::Error::<Test>::CallFiltered
					);
				}
			}
		}
	});
}

#[test]
fn teleport_trust_ends_when_the_data_migration_starts() {
	new_test_ext().execute_with(|| {
		let asset: Asset = (Here, 1u128).into();
		let origin = Location::new(0, [Parachain(CT_PARA_ID)]);
		for stage in all_stages() {
			// GIVEN the machine at `stage`.
			set_stage(stage.clone());

			// THEN the inner filter's answer stands until data moves, and nothing is trusted after.
			assert_eq!(
				NothingOnceDataMigrationStarts::<Test, Everything>::contains(&asset, &origin),
				!stage.data_migration_started(),
				"at {stage:?}"
			);
		}
	});
}

/// The events of the asset traps submitted so far, oldest first.
fn trap_events() -> Vec<Event<Test>> {
	System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Rc2Migrator(
				event @ (Event::AssetTrapTaken { .. } | Event::AssetTrapRejected { .. }),
			) => Some(event),
			_ => None,
		})
		.collect()
}

/// Submit `traps` as root, without finishing.
fn submit(traps: Vec<AssetTrap>) -> sp_runtime::DispatchResult {
	Rc2Migrator::submit_asset_traps(RuntimeOrigin::root(), traps, false)
}

#[test]
fn the_wait_for_the_asset_traps_follows_the_warm_up_and_never_ends_on_its_own() {
	// GIVEN a warm-up that ends with the next block.
	new_test_ext().execute_with(|| {
		let end_at = System::block_number() + 1;
		set_stage(Stage::WarmUp { end_at });

		// WHEN it ends. THEN the machine waits for the asset traps.
		run_blocks(1);
		assert_stage(Stage::WaitingForAssetTraps);

		// WHEN many blocks pass without them. THEN it is still waiting, having sent nothing.
		// There is no timeout.
		run_blocks(50);
		assert_stage(Stage::WaitingForAssetTraps);
		assert_eq!(sent().len(), 0);
	});
}

#[test]
fn the_manager_or_the_admin_submits_asset_traps_and_only_while_they_are_awaited() {
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		assert_ok!(Rc2Migrator::set_manager(RuntimeOrigin::root(), Some(ALICE)));

		// WHEN an account that is not the manager submits. THEN it is refused.
		for who in [BOB, CORETIME] {
			assert_noop!(
				Rc2Migrator::submit_asset_traps(RuntimeOrigin::signed(who), vec![], true),
				BadOrigin
			);
		}

		// WHEN the manager, the admin origin and root submit. THEN each is accepted.
		for origin in
			[RuntimeOrigin::signed(ALICE), RuntimeOrigin::signed(ADMIN), RuntimeOrigin::root()]
		{
			assert_ok!(Rc2Migrator::submit_asset_traps(origin, vec![], false));
		}
		assert_stage(Stage::WaitingForAssetTraps);

		// GIVEN a trap.
		let trap = native_trap(Parachain(2000), 10);
		let hash = make_trap(&trap, 1);

		// WHEN it is submitted at any other stage. THEN it is refused, and the trap stays in
		// `pallet-xcm`.
		for stage in all_stages().into_iter().filter(|s| *s != Stage::WaitingForAssetTraps) {
			set_stage(stage.clone());
			assert_noop!(
				Rc2Migrator::submit_asset_traps(RuntimeOrigin::root(), vec![trap.clone()], true),
				Error::<Test>::NotWaitingForAssetTraps
			);
		}
		assert_eq!(XcmAssetTraps::get(hash), 1);
	});
}

#[test]
fn a_submitted_trap_is_taken_whole_out_of_pallet_xcm() {
	// GIVEN the same assets trapped three times for one claimer.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		let trap = native_trap(Parachain(2000), 10);
		let hash = make_trap(&trap, 3);

		// WHEN the manager submits the trap once, without finishing.
		assert_ok!(submit(vec![trap.clone()]));

		// THEN `pallet-xcm` holds none of the three, the migrator holds all three under the same
		// key, and the machine still waits.
		assert!(!XcmAssetTraps::contains_key(hash));
		assert_eq!(TakenAssetTraps::<Test>::get(&trap), 3);
		assert_eq!(trap_events(), vec![Event::AssetTrapTaken { hash, trap, count: 3 }]);
		assert_stage(Stage::WaitingForAssetTraps);
	});
}

#[test]
fn a_trap_pallet_xcm_does_not_hold_is_rejected_and_the_rest_go_through() {
	// GIVEN one trap, and one that was never made.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		let held = native_trap(Parachain(2000), 10);
		let hash = make_trap(&held, 1);
		let never_made = native_trap(Parachain(2001), 10);

		// WHEN the manager submits the one never made, then the real one twice.
		assert_ok!(submit(vec![never_made.clone(), held.clone(), held.clone()]));

		// THEN the call succeeds and each submission has its own event: the real trap is taken
		// the first time, and the other two are rejected.
		assert_eq!(
			trap_events(),
			vec![
				Event::AssetTrapRejected { trap: never_made, reason: AssetTrapRejection::NotHeld },
				Event::AssetTrapTaken { hash, trap: held.clone(), count: 1 },
				Event::AssetTrapRejected {
					trap: held.clone(),
					reason: AssetTrapRejection::NotHeld
				},
			]
		);
		assert_eq!(TakenAssetTraps::<Test>::iter().collect::<Vec<_>>(), vec![(held, 1)]);
	});
}

#[test]
fn a_trap_is_found_only_with_the_assets_in_the_version_they_were_trapped_in() {
	// GIVEN assets trapped while XCM v4 was current.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		let current = native_trap(Parachain(2000), 10);
		let v4 = AssetTrap {
			claimer: current.claimer.clone().into_version(4).unwrap(),
			assets: current.assets.clone().into_version(4).unwrap(),
		};
		let hash = make_trap(&v4, 1);

		// WHEN the manager submits the same assets in the current version. THEN no trap matches.
		assert_ok!(submit(vec![current.clone()]));

		// WHEN the manager submits them in v4, as `pallet-xcm` reported them. THEN the trap is
		// taken. The claimer may be in any version, since `pallet-xcm` keys it in the current one.
		assert_ok!(submit(vec![v4.clone()]));
		assert_eq!(
			trap_events(),
			vec![
				Event::AssetTrapRejected { trap: current, reason: AssetTrapRejection::NotHeld },
				Event::AssetTrapTaken { hash, trap: v4, count: 1 },
			]
		);
	});
}

#[test]
fn assets_trapped_again_after_they_were_taken_add_to_the_count() {
	// GIVEN a trap of Asset Hub's that was taken.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		let trap = native_trap(Parachain(AH_PARA_ID), 10);
		let hash = make_trap(&trap, 1);
		assert_ok!(submit(vec![trap.clone()]));

		// GIVEN Asset Hub, which can still message this chain, trapped the same assets twice
		// more.
		make_trap(&trap, 2);

		// WHEN the manager submits the trap again.
		assert_ok!(submit(vec![trap.clone()]));

		// THEN the two new ones are taken and added to the one taken before.
		assert_eq!(
			trap_events().last(),
			Some(&Event::AssetTrapTaken { hash, trap: trap.clone(), count: 2 })
		);
		assert_eq!(TakenAssetTraps::<Test>::get(&trap), 3);
	});
}

#[test]
fn finishing_takes_the_last_traps_and_moves_on_to_the_accounts_stage() {
	// GIVEN a trap.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		let trap = native_trap(Parachain(2000), 10);
		make_trap(&trap, 1);

		// WHEN the manager submits it and says it is the last.
		assert_ok!(Rc2Migrator::submit_asset_traps(
			RuntimeOrigin::root(),
			vec![trap.clone()],
			true
		));

		// THEN it is taken, and the machine moves on to the accounts stage.
		assert_eq!(TakenAssetTraps::<Test>::get(&trap), 1);
		assert_stage(Stage::AccountsInit);
		assert_eq!(transitions(), vec![(Stage::WaitingForAssetTraps, Stage::AccountsInit)]);

		// WHEN the manager submits again. THEN it is refused: the wait is over.
		assert_noop!(
			Rc2Migrator::submit_asset_traps(RuntimeOrigin::root(), vec![trap], true),
			Error::<Test>::NotWaitingForAssetTraps
		);
	});
}

/// The asset trap payouts so far, oldest first.
fn paid_events() -> Vec<(AssetTrap, Location, u128)> {
	System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::Rc2Migrator(Event::AssetTrapPaid { trap, beneficiary, amount }) =>
				Some((trap, beneficiary, amount)),
			_ => None,
		})
		.collect()
}

/// The deposits of every message sent to Asset Hub so far, sorted, after checking that each
/// teleports in exactly what it deposits.
fn ah_deposits() -> Vec<(Location, u128)> {
	let mut deposits = Vec::new();
	for (dest, Xcm(message)) in sent() {
		assert_eq!(dest, ah_dest());
		let [UnpaidExecution { weight_limit: Unlimited, check_origin: None }, ReceiveTeleportedAsset(total), rest @ ..] =
			message.as_slice()
		else {
			panic!("not a teleport: {message:?}");
		};
		let mut sum = 0;
		for instruction in rest {
			let DepositAsset { assets: AssetFilter::Definite(assets), beneficiary } = instruction
			else {
				panic!("not a deposit: {instruction:?}");
			};
			let [Asset { id, fun: Fungible(amount) }] = assets.inner().as_slice() else {
				panic!("not one fungible asset: {assets:?}");
			};
			assert_eq!(*id, AssetId(Location::parent()));
			sum += amount;
			deposits.push((beneficiary.clone(), *amount));
		}
		assert_eq!(*total, (Location::parent(), sum).into());
	}
	deposits.sort_by_key(|deposit| deposit.encode());
	deposits
}

/// Take `traps`, each made `count` times, and put the machine at the payout with the ledger the
/// accounts stage seeds.
fn take_for_payout(traps: &[(AssetTrap, u32)]) {
	set_stage(Stage::WaitingForAssetTraps);
	for (trap, count) in traps {
		make_trap(trap, *count);
	}
	assert_ok!(submit(traps.iter().map(|(trap, _)| trap.clone()).collect()));
	assert_eq!(TakenAssetTraps::<Test>::iter().count(), traps.len(), "every trap was taken");
	RcMigratedBalance::<Test>::put(MigratedBalances { kept: 1_000, ..Default::default() });
	set_stage(Stage::PayAssetTraps);
}

#[test]
fn a_trap_the_payout_could_not_pay_is_rejected_and_left_in_pallet_xcm() {
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		let claimer = Location::new(0, [Parachain(2000)]);
		let rejected = |assets: Assets, claimer: Location, reason| {
			let trap = AssetTrap { claimer: claimer.into(), assets: assets.into() };
			let hash = make_trap(&trap, 1);
			assert_ok!(submit(vec![trap.clone()]));
			assert_eq!(trap_events().last(), Some(&Event::AssetTrapRejected { trap, reason }));
			assert_eq!(XcmAssetTraps::get(hash), 1, "the trap stays in `pallet-xcm`");
		};

		// WHEN a trap holds a token other than this chain's, alone or beside it. THEN it is
		// rejected.
		rejected((Parent, 10).into(), claimer.clone(), AssetTrapRejection::NotNative);
		rejected(
			Assets::from(vec![Asset::from((Here, 10)), Asset::from((Parent, 10))]),
			claimer.clone(),
			AssetTrapRejection::NotNative,
		);

		// WHEN its claimer is not a relay account, a parachain, or an account or body on one.
		// THEN it is rejected.
		for claimer in [
			Location::new(0, [Plurality { id: BodyId::Technical, part: BodyPart::Voice }]),
			Location::new(0, [Parachain(2000), Parachain(2001)]),
			Location::new(0, [Parachain(2000), PalletInstance(50)]),
			Location::new(1, [Parachain(2000)]),
		] {
			rejected((Here, 10).into(), claimer, AssetTrapRejection::ClaimerNotOnAssetHub);
		}

		// WHEN it is worth less than Asset Hub's existential deposit. THEN it is rejected.
		rejected((Here, AH_ED - 1).into(), claimer, AssetTrapRejection::BelowExistentialDeposit);
		assert!(TakenAssetTraps::<Test>::iter().next().is_none());
	});
}

#[test]
fn a_trap_is_worth_what_it_holds_times_how_many_times_it_was_trapped() {
	// GIVEN assets worth less than Asset Hub's existential deposit, trapped twice.
	new_test_ext().execute_with(|| {
		set_stage(Stage::WaitingForAssetTraps);
		let trap = native_trap(Parachain(2000), AH_ED - 1);
		let hash = make_trap(&trap, 2);

		// WHEN the manager submits them. THEN the two together reach the deposit, so they are
		// taken.
		assert_ok!(submit(vec![trap.clone()]));
		assert_eq!(
			trap_events(),
			vec![Event::AssetTrapTaken { hash, trap: trap.clone(), count: 2 }]
		);
		assert_eq!(TakenAssetTraps::<Test>::get(&trap), 2);
	});
}

#[test]
fn the_taken_traps_are_paid_to_their_claimers_on_asset_hub() {
	new_test_ext().execute_with(|| {
		// GIVEN a trap of every claimer Asset Hub can pay, one of them trapped three times.
		let relay_account =
			native_trap(AccountId32 { network: Some(NetworkId::Polkadot), id: ALICE.into() }, 10);
		let para = native_trap(Parachain(2000), 6);
		let para_account = native_trap(
			Location::new(0, [Parachain(2001), AccountId32 { network: None, id: BOB.into() }]),
			7,
		);
		let para_key = native_trap(
			Location::new(0, [Parachain(2002), AccountKey20 { network: None, key: [9; 20] }]),
			8,
		);
		let para_body = native_trap(
			Location::new(
				0,
				[Parachain(2003), Plurality { id: BodyId::Index(3), part: BodyPart::Voice }],
			),
			9,
		);
		take_for_payout(&[
			(relay_account.clone(), 1),
			(para.clone(), 3),
			(para_account, 1),
			(para_key, 1),
			(para_body, 1),
		]);
		let issuance = total_issuance();

		// WHEN the payout runs.
		run_blocks(1);

		// THEN one message pays each claimer what its trap held times how many times it was
		// trapped: a relay account as the same account, the rest as Asset Hub sees them.
		let alice = Location::new(0, [AccountId32 { network: None, id: ALICE.into() }]);
		let mut expected = vec![
			(alice.clone(), 10),
			(Location::new(1, [Parachain(2000)]), 18),
			(Location::new(1, [Parachain(2001), AccountId32 { network: None, id: BOB.into() }]), 7),
			(Location::new(1, [Parachain(2002), AccountKey20 { network: None, key: [9; 20] }]), 8),
			(
				Location::new(
					1,
					[Parachain(2003), Plurality { id: BodyId::Index(3), part: BodyPart::Voice }],
				),
				9,
			),
		];
		expected.sort_by_key(|deposit| deposit.encode());
		assert_eq!(sent().len(), 1);
		assert_eq!(ah_deposits(), expected);
		assert!(paid_events().contains(&(relay_account, alice, 10)));
		assert!(paid_events().contains(&(para, Location::new(1, [Parachain(2000)]), 18)));
		assert_eq!(paid_events().len(), 5);

		// THEN the migrator holds no trap, the machine moves on, and the ledger books the payout
		// without taking it from what is kept, since nothing was burned here.
		assert!(TakenAssetTraps::<Test>::iter().next().is_none());
		assert_stage(Stage::Sweep);
		assert_eq!(
			RcMigratedBalance::<Test>::get(),
			MigratedBalances { kept: 1_000, ah_asset_traps: 52, ..Default::default() }
		);
		assert_eq!(total_issuance(), issuance);
	});
}

#[test]
fn the_payout_takes_as_many_messages_and_blocks_as_the_limits_need() {
	new_test_ext().execute_with(|| {
		// GIVEN five traps, two to a message and two messages to a block.
		let traps: Vec<_> = (0..5).map(|i| (native_trap(Parachain(2000 + i), 10), 1)).collect();
		take_for_payout(&traps);
		assert_ok!(Rc2Migrator::set_limits(
			RuntimeOrigin::root(),
			BlockLimits { items_per_xcm: 2, xcm_per_block: 2 }
		));

		// WHEN the first block runs. THEN it pays four in two messages and the stage holds.
		run_blocks(1);
		assert_eq!(sent().len(), 2);
		assert_eq!(ah_deposits().len(), 4);
		assert_stage(Stage::PayAssetTraps);

		// WHEN the next block runs. THEN it pays the last one and the machine moves on.
		run_blocks(1);
		assert_eq!(sent().len(), 3);
		assert_eq!(ah_deposits().len(), 5);
		assert_stage(Stage::Sweep);
		assert_eq!(RcMigratedBalance::<Test>::get().ah_asset_traps, 50);
	});
}

#[test]
fn a_failed_send_leaves_the_payout_for_a_retry() {
	new_test_ext().execute_with(|| {
		// GIVEN a taken trap and a router that refuses everything.
		let trap = native_trap(Parachain(2000), 10);
		take_for_payout(&[(trap.clone(), 1)]);
		SendFails::set(true);

		// WHEN the payout runs. THEN nothing changed: the trap is still held, nothing is booked,
		// and the stage holds.
		run_blocks(3);
		assert_eq!(TakenAssetTraps::<Test>::get(&trap), 1);
		assert_eq!(RcMigratedBalance::<Test>::get().ah_asset_traps, 0);
		assert!(paid_events().is_empty());
		assert_stage(Stage::PayAssetTraps);

		// WHEN the router recovers. THEN the trap is paid.
		SendFails::set(false);
		run_blocks(1);
		assert_eq!(ah_deposits(), vec![(Location::new(1, [Parachain(2000)]), 10)]);
		assert_stage(Stage::Sweep);
	});
}

#[test]
fn a_rewind_does_not_pay_a_trap_twice() {
	new_test_ext().execute_with(|| {
		// GIVEN a trap already paid.
		take_for_payout(&[(native_trap(Parachain(2000), 10), 1)]);
		run_blocks(1);
		assert_eq!(sent().len(), 1);

		// WHEN the machine is rewound to the payout and runs it again.
		assert_ok!(Rc2Migrator::pause_migration(RuntimeOrigin::root()));
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::PayAssetTraps));
		assert_ok!(Rc2Migrator::resume_migration(RuntimeOrigin::root()));
		run_blocks(1);

		// THEN nothing more is sent or booked, and the machine moves on.
		assert_eq!(sent().len(), 1);
		assert_eq!(RcMigratedBalance::<Test>::get().ah_asset_traps, 10);
		assert_stage(Stage::Sweep);
	});
}
