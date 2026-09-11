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
	mock::*, CtMigratorCall, CtRuntimeCall, Error, Event, MigrationStage, RcMigrationStage,
	CT_MIGRATOR_PALLET_INDEX,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;
use xcm::prelude::*;

type Stage = MigrationStage<AccountId, u64, u64>;

fn stage() -> Stage {
	RcMigrationStage::<Test>::get()
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

		// THEN the machine has not moved, nothing was sent, and no event was emitted. This is
		// what makes the pallet safe to carry in a runtime before anyone commits to a migration.
		assert_stage(Stage::Pending);
		assert_eq!(sent().len(), 0);
		assert_eq!(transitions(), vec![]);
	});
}

#[test]
fn only_root_can_schedule_and_only_into_the_future() {
	new_test_ext().execute_with(|| {
		// WHEN a signed account tries to schedule. THEN it is refused.
		assert_noop!(Rc2Migrator::schedule_migration(RuntimeOrigin::signed(ALICE), 10), BadOrigin);
		// Being the Coretime chain confers no scheduling power either.
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::signed(CORETIME), 10),
			BadOrigin
		);

		// WHEN root schedules the present moment or earlier. THEN it is refused: a start already
		// past would begin the migration on the very next block.
		let now = now_ms();
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now),
			Error::<Test>::StartInPast
		);
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now - 1),
			Error::<Test>::StartInPast
		);

		// WHEN root schedules a future moment. THEN the machine is armed.
		let start = now + 5 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), start));
		assert_stage(Stage::Scheduled { start });

		// WHEN root schedules again. THEN it is refused, so a second governance call cannot
		// silently move a start date that is already committed.
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), start + BLOCK_TIME_MS),
			Error::<Test>::AlreadyScheduled
		);
	});
}

#[test]
fn a_scheduled_migration_starts_once_the_clock_passes_it_and_not_before() {
	// GIVEN a migration scheduled two blocks' worth of time ahead.
	new_test_ext().execute_with(|| {
		let start = now_ms() + 2 * BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), start));

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
fn waiting_for_coretime_never_advances_on_its_own() {
	// GIVEN a machine that has sent its start signal.
	new_test_ext().execute_with(|| {
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::WaitingForCt));

		// WHEN many blocks pass without an answer.
		run_blocks(50);

		// THEN it is still waiting. No timeout is deliberate: nothing has been drained yet, so
		// the right response to silence is a human with `force_set_stage`, not a machine that
		// proceeds into a chain that may not have upgraded.
		assert_stage(Stage::WaitingForCt);
		assert_eq!(sent().len(), 0);
	});
}

#[test]
fn only_the_coretime_chain_can_confirm_readiness() {
	// GIVEN a machine waiting for the Coretime chain.
	new_test_ext().execute_with(|| {
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::WaitingForCt));

		// WHEN anyone else confirms, root included. THEN it is refused.
		assert_noop!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(ALICE)), BadOrigin);
		assert_noop!(Rc2Migrator::ct_ready(RuntimeOrigin::root()), BadOrigin);
		assert_stage(Stage::WaitingForCt);

		// WHEN the Coretime chain confirms. THEN the machine is admitted to the next stage,
		// which with no data stages implemented is the verification window.
		let now = System::block_number();
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		assert_stage(Stage::CoolOff { end_at: now + COOL_OFF });
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

		// Same for a migration that has already finished.
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::MigrationDone));
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
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::CoolOff { end_at }));

		// WHEN the window has not elapsed. THEN nothing happens: the window exists so the end
		// state can be inspected while the migration's filters are still engaged.
		run_blocks(COOL_OFF - 1);
		assert_stage(Stage::CoolOff { end_at });
		assert_eq!(sent().len(), 0);

		// WHEN it elapses. THEN the finish signal is sent and both chains are done.
		run_blocks(1);
		assert_stage(Stage::MigrationDone);
		assert_eq!(sent().len(), 1);
		assert_eq!(sent_call(0), CtRuntimeCall::CtMigrator(CtMigratorCall::FinishMigration));

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
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), start));
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
	});
}

#[test]
fn force_set_stage_is_root_only_and_unconstrained() {
	new_test_ext().execute_with(|| {
		// WHEN a non-root origin forces a stage. THEN it is refused.
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::signed(ALICE), Stage::MigrationDone),
			BadOrigin
		);
		assert_noop!(
			Rc2Migrator::force_set_stage(RuntimeOrigin::signed(CORETIME), Stage::MigrationDone),
			BadOrigin
		);

		// WHEN root forces stages. THEN it may move anywhere, including backwards and into the
		// halt stage.
		for target in [
			Stage::WaitingForCt,
			Stage::Paused,
			Stage::MigrationDone,
			Stage::Scheduled { start: 99 * BLOCK_TIME_MS },
			Stage::Pending,
		] {
			assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), target.clone()));
			assert_stage(target);
		}
	});
}

#[test]
fn a_paused_machine_does_not_advance_but_stays_engaged() {
	// GIVEN a paused migration.
	new_test_ext().execute_with(|| {
		assert_ok!(Rc2Migrator::force_set_stage(RuntimeOrigin::root(), Stage::Paused));

		// WHEN blocks pass. THEN nothing moves — only `force_set_stage` leaves this stage.
		run_blocks(20);
		assert_stage(Stage::Paused);
		assert_eq!(sent().len(), 0);

		// THEN the migration still counts as ongoing.
		assert!(stage().is_ongoing());
		assert!(!stage().is_finished());
	});
}

#[test]
fn the_machine_runs_from_pending_to_done() {
	// GIVEN a scheduled migration and a Coretime chain that answers.
	new_test_ext().execute_with(|| {
		let start = now_ms() + BLOCK_TIME_MS;
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), start));

		// WHEN the start passes.
		run_blocks(2);
		assert_stage(Stage::WaitingForCt);

		// WHEN the Coretime chain confirms.
		assert_ok!(Rc2Migrator::ct_ready(RuntimeOrigin::signed(CORETIME)));
		let end_at = System::block_number() + COOL_OFF;
		assert_stage(Stage::CoolOff { end_at });

		// WHEN the verification window elapses.
		run_blocks(COOL_OFF);

		// THEN the migration is done, and the two signals it sent are the start and the finish,
		// in that order.
		assert_stage(Stage::MigrationDone);
		assert_eq!(
			sent().iter().enumerate().map(|(i, _)| sent_call(i)).collect::<Vec<_>>(),
			vec![
				CtRuntimeCall::CtMigrator(CtMigratorCall::StartMigration),
				CtRuntimeCall::CtMigrator(CtMigratorCall::FinishMigration),
			]
		);
		assert_eq!(
			transitions(),
			vec![
				(Stage::Pending, Stage::Scheduled { start }),
				(Stage::Scheduled { start }, Stage::WaitingForCt),
				(Stage::WaitingForCt, Stage::CoolOff { end_at }),
				(Stage::CoolOff { end_at }, Stage::MigrationDone),
			]
		);
	});
}

#[test]
fn the_stage_predicates_say_what_their_consumers_need() {
	// Exhaustive over the stage enum, so a new stage has to classify itself here rather than
	// inherit whatever the predicates happen to return.
	let cases: [(Stage, bool, bool); 21] = [
		//                                            ongoing, finished
		(Stage::Pending, false, false),
		(Stage::Scheduled { start: 10 }, false, false),
		(Stage::Paused, true, false),
		(Stage::WaitingForCt, true, false),
		(Stage::AccountsInit, true, false),
		(Stage::AccountsOngoing { last_key: None }, true, false),
		(Stage::AccountsDone, true, false),
		(Stage::ProxyInit, true, false),
		(Stage::ProxyOngoing { last_key: None }, true, false),
		(Stage::ProxyDone, true, false),
		(Stage::RegistrarInit, true, false),
		(Stage::RegistrarOngoing { last_key: None }, true, false),
		(Stage::RegistrarDone, true, false),
		(Stage::HrmpInit, true, false),
		(Stage::HrmpOngoing { last_key: None }, true, false),
		(Stage::HrmpDone, true, false),
		(Stage::Sweep, true, false),
		(Stage::SweepDust { last_key: None }, true, false),
		(Stage::TiCorrection, true, false),
		(Stage::CoolOff { end_at: 10 }, true, false),
		(Stage::MigrationDone, false, true),
	];

	for (stage, ongoing, finished) in cases {
		assert_eq!(stage.is_ongoing(), ongoing, "is_ongoing for {stage:?}");
		assert_eq!(stage.is_finished(), finished, "is_finished for {stage:?}");
	}
}

#[test]
fn the_coretime_call_encoding_is_pinned() {
	// Pins the hand-encoded bytes so an accidental renumbering fails here first, with a readable
	// diff. The Coretime runtimes assert the pallet index against the real `construct_runtime!`.
	assert_eq!(
		CtRuntimeCall::CtMigrator(CtMigratorCall::StartMigration).encode(),
		vec![CT_MIGRATOR_PALLET_INDEX, 0]
	);
	assert_eq!(
		CtRuntimeCall::CtMigrator(CtMigratorCall::FinishMigration).encode(),
		vec![CT_MIGRATOR_PALLET_INDEX, 1]
	);
}
