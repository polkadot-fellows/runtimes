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
	mock::*, CtMigrationStage, Error, Event, Manager, MigrationStage, Rc2MigratorCall,
	Rc2RuntimeCall,
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
fn a_signed_account_drives_nothing() {
	// GIVEN a chain that has not been migrated into.
	new_test_ext().execute_with(|| {
		// WHEN a signed account that is not the manager drives the migration. THEN every call
		// is refused.
		assert_noop!(CtMigrator::start_migration(RuntimeOrigin::signed(ALICE)), BadOrigin);
		assert_noop!(CtMigrator::end_lockdown(RuntimeOrigin::signed(ALICE)), BadOrigin);
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
fn the_manager_drives_the_migration_but_cannot_appoint_one() {
	new_test_ext().execute_with(|| {
		// WHEN a signed account appoints itself. THEN it is refused.
		assert_noop!(CtMigrator::set_manager(RuntimeOrigin::signed(ALICE), Some(ALICE)), BadOrigin);

		// GIVEN Alice appointed manager by the admin origin.
		assert_ok!(CtMigrator::set_manager(RuntimeOrigin::signed(ADMIN), Some(ALICE)));
		assert_eq!(Manager::<Test>::get(), Some(ALICE));
		System::assert_last_event(Event::ManagerSet { old: None, new: Some(ALICE) }.into());

		// WHEN the manager sends the relay chain's two signals and forces a stage. THEN all
		// three are accepted: the manager stands in for a signal that never arrived.
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::signed(ALICE)));
		assert_stage(MigrationStage::DataMigrationOngoing);
		assert_eq!(sent_call(0), Rc2RuntimeCall::Rc2Migrator(Rc2MigratorCall::CtReady));
		assert_ok!(CtMigrator::end_lockdown(RuntimeOrigin::signed(ALICE)));
		assert_stage(MigrationStage::MigrationDone);
		assert_ok!(CtMigrator::force_set_stage(
			RuntimeOrigin::signed(ALICE),
			MigrationStage::Pending
		));
		assert_stage(MigrationStage::Pending);

		// WHEN the manager appoints a manager. THEN it is refused, so the appointment stays with
		// the admin origin alone.
		assert_noop!(CtMigrator::set_manager(RuntimeOrigin::signed(ALICE), None), BadOrigin);

		// WHEN root removes the manager. THEN Alice loses the powers.
		assert_ok!(CtMigrator::set_manager(RuntimeOrigin::root(), None));
		assert_eq!(Manager::<Test>::get(), None);
		System::assert_last_event(Event::ManagerSet { old: Some(ALICE), new: None }.into());
		assert_noop!(CtMigrator::start_migration(RuntimeOrigin::signed(ALICE)), BadOrigin);
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
fn the_readiness_answer_is_the_message_the_relay_chain_expects() {
	new_test_ext().execute_with(|| {
		// WHEN this chain answers the RC.
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));

		// THEN message is what RC expects
		assert_eq!(
			sent(),
			vec![(
				Location::parent(),
				Xcm(vec![
					// unpaid execution so the RC's barrier lets it in
					UnpaidExecution { weight_limit: Unlimited, check_origin: None },
					Transact {
						// arrives with this chain's parachain origin
						origin_kind: OriginKind::Xcm,
						fallback_max_weight: None,
						call: Rc2RuntimeCall::Rc2Migrator(Rc2MigratorCall::CtReady).encode().into(),
					},
					// so a refused call fails the message rather than vanishing
					ExpectTransactStatus(MaybeErrorCode::Success),
				]),
			)]
		);
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
		assert_ok!(CtMigrator::end_lockdown(RuntimeOrigin::root()));

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
fn a_completion_closes_the_migration() {
	// GIVEN a migration under way.
	new_test_ext().execute_with(|| {
		assert_ok!(CtMigrator::start_migration(RuntimeOrigin::root()));

		// WHEN the relay chain closes its verification window.
		assert_ok!(CtMigrator::end_lockdown(RuntimeOrigin::root()));

		// THEN this chain is done, and the only message sent is still the start's answer: the
		// completion is one-directional.
		assert_stage(MigrationStage::MigrationDone);
		assert_eq!(sent().len(), 1);

		// WHEN the same signal arrives again. THEN it is accepted as a no-op — a duplicated
		// message is not worth failing an XCM over.
		assert_ok!(CtMigrator::end_lockdown(RuntimeOrigin::root()));
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
fn a_completion_for_a_migration_that_never_started_is_rejected() {
	new_test_ext().execute_with(|| {
		// WHEN a completion arrives first. THEN it is refused, so a stray or reordered message
		// cannot mark this chain migrated without it ever having received anything.
		assert_noop!(CtMigrator::end_lockdown(RuntimeOrigin::root()), Error::<Test>::NotStarted);
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
	fn expected(stage: &MigrationStage) -> (bool, bool) {
		match stage {
			//                                 ongoing, finished
			MigrationStage::Pending => (false, false),
			MigrationStage::DataMigrationOngoing => (true, false),
			MigrationStage::MigrationDone => (false, true),
		}
	}

	let cases = [
		MigrationStage::Pending,
		MigrationStage::DataMigrationOngoing,
		MigrationStage::MigrationDone,
	];

	for stage in cases {
		let (ongoing, finished) = expected(&stage);
		assert_eq!(stage.is_ongoing(), ongoing, "is_ongoing for {stage:?}");
		assert_eq!(stage.is_finished(), finished, "is_finished for {stage:?}");
	}
}
