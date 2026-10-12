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
	mock::*, multisig::ManagerMultisigVoteOf, Call, Error, Event, ManagerMultisigRound,
	ManagerMultisigs, ManagerVotesInCurrentRound, MigrationStage, Paused, RcMigrationStage,
};
use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_core::{sr25519, Pair};
use sp_runtime::{
	transaction_validity::InvalidTransaction, DispatchError, DispatchResult, MultiSignature,
	MultiSigner,
};

/// A member's signed vote for `call` in `round`.
fn vote_in_round(
	pair: &sr25519::Pair,
	call: RuntimeCall,
	round: u32,
) -> (ManagerMultisigVoteOf<Test>, MultiSignature) {
	let payload =
		ManagerMultisigVoteOf::<Test> { who: MultiSigner::Sr25519(pair.public()), call, round };
	let sig = MultiSignature::Sr25519(pair.sign(&payload.encode_with_bytes_wrapper()));
	(payload, sig)
}

/// A member's signed vote for `call` in the current round.
fn vote(pair: &sr25519::Pair, call: RuntimeCall) -> (ManagerMultisigVoteOf<Test>, MultiSignature) {
	vote_in_round(pair, call, ManagerMultisigRound::<Test>::get())
}

/// Submit a vote through the unsigned call.
fn submit(payload: ManagerMultisigVoteOf<Test>, sig: MultiSignature) -> DispatchResult {
	Rc2Migrator::vote_manager_multisig(RuntimeOrigin::none(), Box::new(payload), sig)
}

/// Two members vote for `call`, which meets the mock's threshold.
fn reach_threshold(call: RuntimeCall) {
	for n in [1, 2] {
		let (payload, sig) = vote(&member(n).0, call.clone());
		assert_ok!(submit(payload, sig));
	}
}

/// A call any signed origin may dispatch, distinguishable by its bytes.
fn remark(n: u8) -> RuntimeCall {
	RuntimeCall::System(frame_system::Call::<Test>::remark_with_event { remark: vec![n] })
}

/// Who `remark_with_event` recorded as the sender, for every remark so far.
fn remark_senders() -> Vec<AccountId> {
	System::events()
		.into_iter()
		.filter_map(|r| match r.event {
			RuntimeEvent::System(frame_system::Event::Remarked { sender, .. }) => Some(sender),
			_ => None,
		})
		.collect()
}

#[test]
fn threshold_dispatches_as_the_multisig_and_ends_the_round() {
	new_test_ext().execute_with(|| {
		let (alice, alice_id) = member(1);
		let (bob, _) = member(2);
		assert_eq!(ManagerMultisigRound::<Test>::get(), MultisigStartRound::get());
		let call = remark(1);

		// WHEN one member votes. THEN the vote is recorded and nothing is dispatched.
		let (payload, sig) = vote(&alice, call.clone());
		assert_ok!(submit(payload, sig));
		assert_eq!(ManagerMultisigs::<Test>::get(&call), vec![alice_id]);
		System::assert_has_event(Event::ManagerMultisigVoted { votes: 1 }.into());
		assert_eq!(remark_senders(), vec![]);

		// WHEN the same member votes again for the same call. THEN it is refused.
		let (payload, sig) = vote(&alice, call.clone());
		assert_noop!(submit(payload, sig), Error::<Test>::DuplicateVote);

		// WHEN a second member votes. THEN the call is dispatched as the multisig and the round
		// ends with its bookkeeping cleared.
		let (payload, sig) = vote(&bob, call.clone());
		assert_ok!(submit(payload, sig));
		assert_eq!(remark_senders(), vec![Rc2Migrator::manager_multisig_id()]);
		System::assert_has_event(Event::ManagerMultisigDispatched { res: Ok(()) }.into());
		assert_eq!(ManagerMultisigRound::<Test>::get(), MultisigStartRound::get() + 1);
		assert_eq!(ManagerMultisigs::<Test>::iter().count(), 0);
		assert_eq!(ManagerVotesInCurrentRound::<Test>::iter().count(), 0);

		// WHEN a vote from the previous round arrives. THEN it is refused at the pool and at
		// dispatch.
		let (payload, sig) = vote_in_round(&alice, call, MultisigStartRound::get());
		assert_eq!(
			Rc2Migrator::validate_vote(&payload, &sig),
			Err(InvalidTransaction::Stale.into())
		);
		assert_noop!(submit(payload, sig), Error::<Test>::RoundStale);
	});
}

#[test]
fn a_failed_dispatch_still_ends_the_round() {
	new_test_ext().execute_with(|| {
		// GIVEN a call the multisig's signed origin may not dispatch.
		let call = RuntimeCall::System(frame_system::Call::<Test>::set_code { code: vec![] });

		// WHEN the threshold is met. THEN the failure is reported and the round still ends, so
		// the same votes cannot be resubmitted.
		reach_threshold(call);
		System::assert_has_event(
			Event::ManagerMultisigDispatched { res: Err(DispatchError::BadOrigin) }.into(),
		);
		assert_eq!(ManagerMultisigRound::<Test>::get(), MultisigStartRound::get() + 1);
		assert_eq!(ManagerMultisigs::<Test>::iter().count(), 0);
	});
}

#[test]
fn only_members_with_their_own_signature_may_vote() {
	new_test_ext().execute_with(|| {
		let (alice, alice_id) = member(1);
		let (mallory, _) = member(9); // not a member
		let call = remark(1);

		// WHEN a non-member votes. THEN it is refused.
		let (payload, sig) = vote(&mallory, call.clone());
		assert_eq!(
			Rc2Migrator::validate_vote(&payload, &sig),
			Err(InvalidTransaction::BadSigner.into())
		);
		assert_noop!(submit(payload, sig), Error::<Test>::NotMultisigMember);

		// WHEN a member's vote carries somebody else's signature. THEN it is refused.
		let (payload, _) = vote(&alice, call.clone());
		let forged = MultiSignature::Sr25519(mallory.sign(&payload.encode_with_bytes_wrapper()));
		assert_eq!(
			Rc2Migrator::validate_vote(&payload, &forged),
			Err(InvalidTransaction::BadProof.into())
		);
		assert_noop!(submit(payload.clone(), forged), Error::<Test>::BadVoteSignature);

		// WHEN a member signs the bare vote, without the wallet wrapper. THEN it is refused.
		let bare = MultiSignature::Sr25519(alice.sign(&payload.encode()));
		assert_eq!(
			Rc2Migrator::validate_vote(&payload, &bare),
			Err(InvalidTransaction::BadProof.into())
		);

		// WHEN the member's own votes arrive. THEN the pool keeps one per member, whatever the
		// call, and the vote counts.
		let (payload, sig) = vote(&alice, call);
		let (other, other_sig) = vote(&alice, remark(2));
		assert_eq!(
			Rc2Migrator::validate_vote(&payload, &sig).unwrap().provides,
			Rc2Migrator::validate_vote(&other, &other_sig).unwrap().provides
		);
		assert_ok!(submit(payload, sig));
		assert_eq!(ManagerVotesInCurrentRound::<Test>::get(&alice_id), 1);
	});
}

#[test]
fn votes_per_round_are_capped_and_reset_on_dispatch() {
	new_test_ext().execute_with(|| {
		let (alice, alice_id) = member(1);
		let (bob, _) = member(2);

		// GIVEN a member that has cast every vote this round, on distinct calls.
		for n in 1..=MultisigMaxVotesPerRound::get() as u8 {
			let (payload, sig) = vote(&alice, remark(n));
			assert_ok!(submit(payload, sig));
		}
		assert_eq!(
			ManagerVotesInCurrentRound::<Test>::get(&alice_id),
			MultisigMaxVotesPerRound::get()
		);

		// WHEN it votes once more. THEN it is refused at the pool and at dispatch.
		let (payload, sig) = vote(&alice, remark(4));
		assert_eq!(
			Rc2Migrator::validate_vote(&payload, &sig),
			Err(InvalidTransaction::Stale.into())
		);
		assert_noop!(submit(payload, sig), Error::<Test>::MaxVotesPerRound);

		// WHEN another member completes one of its calls. THEN the round ends and the member can
		// vote again.
		let (payload, sig) = vote(&bob, remark(2));
		assert_ok!(submit(payload, sig));
		assert_eq!(remark_senders(), vec![Rc2Migrator::manager_multisig_id()]);
		assert_eq!(ManagerVotesInCurrentRound::<Test>::get(&alice_id), 0);
		let (payload, sig) = vote(&alice, remark(4));
		assert_ok!(submit(payload, sig));
	});
}

#[test]
fn the_multisig_is_accepted_as_the_manager() {
	new_test_ext().execute_with(|| {
		// GIVEN a running migration and no manager set.
		RcMigrationStage::<Test>::put(MigrationStage::WarmUp { end_at: 100 });

		// WHEN the multisig pauses it. THEN the call goes through as the manager would.
		reach_threshold(RuntimeCall::Rc2Migrator(Call::<Test>::pause_migration {}));
		System::assert_has_event(Event::ManagerMultisigDispatched { res: Ok(()) }.into());
		assert!(Paused::<Test>::get());
	});
}
