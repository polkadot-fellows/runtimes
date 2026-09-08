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
	mock::*, CtMigrationStage, Error, Event, MigrationStage, Rc2MigratorCall, Rc2RuntimeCall,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::DispatchError::BadOrigin;
use xcm::prelude::*;

fn stage() -> MigrationStage {
	CtMigrationStage::<Test>::get()
}

fn assert_stage(expected: MigrationStage) {
	assert_eq!(stage(), expected);
}

/// The stage transitions emitted so far, oldest first.
fn transitions() -> Vec<(MigrationStage, MigrationStage)> {
	System::events()
		.into_iter()
		.filter_map(|record| match record.event {
			RuntimeEvent::CtMigrator(Event::StageTransition { old, new }) => Some((old, new)),
			_ => None,
		})
		.collect()
}

#[test]
fn the_migration_calls_are_root_only() {
	// GIVEN a chain that has not been migrated into.
	new_test_ext().execute_with(|| {
		// WHEN a signed account drives the migration. THEN every call is refused. On the real
		// chain Root is what the relay-chain location converts to, so this is the check that
		// keeps a local account from driving a migration.
		assert_noop!(CtMigrator::start_migration(RuntimeOrigin::signed(ALICE)), BadOrigin);
		assert_noop!(CtMigrator::finish_migration(RuntimeOrigin::signed(ALICE)), BadOrigin);
		assert_noop!(
			CtMigrator::force_set_stage(
				RuntimeOrigin::signed(ALICE),
				MigrationStage::MigrationDone
			),
			BadOrigin
		);
		assert_stage(MigrationStage::Pending);
	});
}

#[test]
fn a_start_opens_the_migration_and_answers_the_relay_chain() {
	// GIVEN a chain that has not been migrated into.
	new_test_ext().execute_with(|| {
		assert_stage(MigrationStage::Pending);

		// WHEN the relay chain asks whether this chain can receive state.
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));

		// THEN this chain opens the migration and answers upwards. The answer is what unblocks
		// the relay chain's first data stage.
		assert_stage(MigrationStage::DataMigrationOngoing);
		assert_eq!(sent().len(), 1);
		assert_eq!(sent()[0].0, Location::parent());
		assert_eq!(sent_call(0), Rc2RuntimeCall::Rc2Migrator(Rc2MigratorCall::CtReady));
	});
}

#[test]
fn a_repeated_start_answers_again_without_reopening() {
	// GIVEN a migration already under way.
	new_test_ext().execute_with(|| {
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));

		// WHEN the relay chain asks again — which is what happens when governance rewinds it to
		// `Scheduled` and lets the handshake re-run.
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));

		// THEN the answer is re-sent but the stage is untouched, so a rewind on one chain does
		// not need a matching rewind on the other.
		assert_stage(MigrationStage::DataMigrationOngoing);
		assert_eq!(sent().len(), 2);
		assert_eq!(sent_call(1), Rc2RuntimeCall::Rc2Migrator(Rc2MigratorCall::CtReady));
		assert_eq!(
			transitions(),
			vec![(MigrationStage::Pending, MigrationStage::DataMigrationOngoing)]
		);
	});
}

#[test]
fn a_start_after_the_migration_finished_is_rejected() {
	// GIVEN a chain that has already been migrated into.
	new_test_ext().execute_with(|| {
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));
		assert_ok!(CtMigrator::finish_migration(RuntimeOrigin::root()));

		// WHEN a start arrives. THEN it is refused: re-opening a finished migration would
		// re-arm the filters on a chain that is already serving its new control plane.
		assert_noop!(
			CtMigrator::start_migration(RuntimeOrigin::root()),
			Error::<Test>::AlreadyFinished
		);
		assert_stage(MigrationStage::MigrationDone);
	});
}

#[test]
fn a_start_this_chain_cannot_answer_changes_nothing() {
	// GIVEN a router that refuses everything.
	new_test_ext().execute_with(|| {
		SendFails::set(true);

		// WHEN the relay chain asks. THEN the call fails and the stage is untouched, so this
		// chain never believes a migration is under way that the relay chain will not continue.
		assert_noop!(
			CtMigrator::start_migration(RuntimeOrigin::root()),
			Error::<Test>::XcmSendFailed
		);
		assert_stage(MigrationStage::Pending);
		assert_eq!(sent().len(), 0);

		// WHEN the router recovers. THEN the handshake completes on the relay chain's retry.
		SendFails::set(false);
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));
		assert_stage(MigrationStage::DataMigrationOngoing);
	});
}

#[test]
fn a_finish_closes_the_migration() {
	// GIVEN a migration under way.
	new_test_ext().execute_with(|| {
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));

		// WHEN the relay chain signals the end.
		assert_ok!(CtMigrator::finish_migration(RuntimeOrigin::root()));

		// THEN this chain is done and nothing further was sent: the finish is one-directional.
		assert_stage(MigrationStage::MigrationDone);
		assert_eq!(sent().len(), 1);

		// WHEN the same signal arrives again. THEN it is accepted as a no-op — a duplicated
		// message is not worth failing an XCM over.
		assert_ok!(CtMigrator::finish_migration(RuntimeOrigin::root()));
		assert_stage(MigrationStage::MigrationDone);
		assert_eq!(
			transitions(),
			vec![
				(MigrationStage::Pending, MigrationStage::DataMigrationOngoing),
				(MigrationStage::DataMigrationOngoing, MigrationStage::MigrationDone),
			]
		);
	});
}

#[test]
fn a_finish_for_a_migration_that_never_started_is_rejected() {
	new_test_ext().execute_with(|| {
		// WHEN a finish arrives first. THEN it is refused, so a stray or reordered message
		// cannot mark this chain migrated without it ever having received anything.
		assert_noop!(
			CtMigrator::finish_migration(RuntimeOrigin::root()),
			Error::<Test>::NotStarted
		);
		assert_stage(MigrationStage::Pending);
	});
}

#[test]
fn force_set_stage_moves_anywhere() {
	new_test_ext().execute_with(|| {
		// WHEN root forces stages. THEN it may move anywhere, including backwards — the escape
		// hatch for a handshake that cannot complete on its own.
		for target in [
			MigrationStage::DataMigrationOngoing,
			MigrationStage::MigrationDone,
			MigrationStage::Pending,
		] {
			assert_ok!(CtMigrator::force_set_stage(RuntimeOrigin::root(), target.clone()));
			assert_stage(target);
		}
	});
}

#[test]
fn the_stage_predicates_say_what_their_consumers_need() {
	// `is_finished` is what the control-plane call filter on this chain keys off: the pallets
	// that take over from the relay chain stay closed until the handover is complete.
	let cases: [(MigrationStage, bool, bool); 3] = [
		//                                   ongoing, finished
		(MigrationStage::Pending, false, false),
		(MigrationStage::DataMigrationOngoing, true, false),
		(MigrationStage::MigrationDone, false, true),
	];

	for (stage, ongoing, finished) in cases {
		assert_eq!(stage.is_ongoing(), ongoing, "is_ongoing for {stage:?}");
		assert_eq!(stage.is_finished(), finished, "is_finished for {stage:?}");
	}
}

#[test]
fn the_relay_call_encoding_is_pinned() {
	// The relay-chain call is hand-encoded, so nothing in the compiler checks these indices. The
	// integration tests decode them with the real relay runtime; this pins the bytes so an
	// accidental renumbering fails here first, with a readable diff.
	assert_eq!(Rc2RuntimeCall::Rc2Migrator(Rc2MigratorCall::CtReady).encode(), vec![254, 2]);
}
