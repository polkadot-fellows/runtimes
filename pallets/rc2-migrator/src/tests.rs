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

use crate::{
	mock::*, CtMigratorCall, CtRuntimeCall, Error, Event, MigrationStage, RcMigrationStage,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;
use xcm::prelude::*;

type Stage = MigrationStage<u64>;

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

		// WHEN root schedules the current block or earlier. THEN it is refused: a start in the
		// past would begin the migration inside the same block that scheduled it.
		let now = System::block_number();
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now),
			Error::<Test>::StartInPast
		);
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now - 1),
			Error::<Test>::StartInPast
		);

		// WHEN root schedules a future block. THEN the machine is armed.
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now + 5));
		assert_stage(Stage::Scheduled { start: now + 5 });

		// WHEN root schedules again. THEN it is refused, so a second governance call cannot
		// silently move a start date that is already committed.
		assert_noop!(
			Rc2Migrator::schedule_migration(RuntimeOrigin::root(), now + 9),
			Error::<Test>::AlreadyScheduled
		);
	});
}

#[test]
fn a_scheduled_migration_starts_on_its_block_and_not_before() {
	// GIVEN a migration scheduled for block 5.
	new_test_ext().execute_with(|| {
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), 5));

		// WHEN the blocks before it pass. THEN nothing is sent and the stage holds: the relay
		// chain serves its users normally right up to the start block.
		run_blocks(3);
		assert_eq!(System::block_number(), 4);
		assert_stage(Stage::Scheduled { start: 5 });
		assert_eq!(sent().len(), 0);

		// WHEN the start block arrives. THEN exactly one start signal goes to the Coretime
		// chain and the machine waits for the answer.
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

		// WHEN anyone else confirms. THEN it is refused — including root, whose way into the
		// machine is `force_set_stage`, not a forged handshake.
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
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), 3));
		SendFails::set(true);

		// WHEN the start block passes. THEN the machine has not advanced and nothing was sent:
		// a lost start signal must not leave the relay chain believing the handshake is open.
		run_blocks(5);
		assert_stage(Stage::Scheduled { start: 3 });
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
		// halt stage. A machine that second-guesses root here is one that cannot be rescued.
		for target in [
			Stage::WaitingForCt,
			Stage::Paused,
			Stage::MigrationDone,
			Stage::Scheduled { start: 99 },
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

		// THEN the migration still counts as ongoing, so call filters and barriers keyed on the
		// stage stay engaged while a human works out what went wrong.
		assert!(stage().is_ongoing());
		assert!(stage().has_started());
		assert!(!stage().is_finished());
	});
}

#[test]
fn the_machine_runs_from_pending_to_done() {
	// GIVEN a scheduled migration and a Coretime chain that answers.
	new_test_ext().execute_with(|| {
		assert_ok!(Rc2Migrator::schedule_migration(RuntimeOrigin::root(), 2));

		// WHEN the start block passes.
		run_blocks(1);
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
				(Stage::Pending, Stage::Scheduled { start: 2 }),
				(Stage::Scheduled { start: 2 }, Stage::WaitingForCt),
				(Stage::WaitingForCt, Stage::CoolOff { end_at }),
				(Stage::CoolOff { end_at }, Stage::MigrationDone),
			]
		);
	});
}

#[test]
fn the_stage_predicates_say_what_their_consumers_need() {
	// Three predicates with three different consumers: `has_started` gates calls that must stay
	// closed once the migration begins, `is_ongoing` gates what reopens afterwards, and
	// `is_finished` gates the control plane that only exists after the handover.
	let cases: [(Stage, bool, bool, bool); 6] = [
		//                          ongoing, started, finished
		(Stage::Pending, false, false, false),
		(Stage::Scheduled { start: 10 }, false, false, false),
		(Stage::WaitingForCt, true, true, false),
		(Stage::Paused, true, true, false),
		(Stage::CoolOff { end_at: 10 }, true, true, false),
		(Stage::MigrationDone, false, true, true),
	];

	for (stage, ongoing, started, finished) in cases {
		assert_eq!(stage.is_ongoing(), ongoing, "is_ongoing for {stage:?}");
		assert_eq!(stage.has_started(), started, "has_started for {stage:?}");
		assert_eq!(stage.is_finished(), finished, "is_finished for {stage:?}");
	}
}

#[test]
fn the_coretime_call_encoding_is_pinned() {
	// The Coretime calls are hand-encoded, so nothing in the compiler checks these indices. The
	// integration tests decode them with the real Coretime runtime; this pins the bytes so an
	// accidental renumbering fails here first, with a readable diff.
	assert_eq!(CtRuntimeCall::CtMigrator(CtMigratorCall::StartMigration).encode(), vec![100, 0]);
	assert_eq!(CtRuntimeCall::CtMigrator(CtMigratorCall::FinishMigration).encode(), vec![100, 1]);
}
