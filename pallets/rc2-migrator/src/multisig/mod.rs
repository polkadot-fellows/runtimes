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

//! The manager multisig. `Config::MultisigMembers` vote by unsigned extrinsic; once
//! `Config::MultisigThreshold` of them have voted for the same call it is dispatched as the
//! multisig's account, which is accepted wherever the manager is. Members need no account here.
//!
//! A vote is a member's signature over `(who, call, round)`, in the `<Bytes>` wrapper wallet
//! `signRaw` adds. The round advances on every dispatch, so an old round's votes cannot be
//! replayed, and each network starts at its own `Config::MultisigStartRound`, so one chain's
//! votes are not valid on another. A member has `Config::MultisigMaxVotesPerRound` votes per
//! round, which bounds what the unsigned path lets one member put into blocks.

#[cfg(test)]
mod tests;

use crate::{
	Config, Error, Event, ManagerMultisigRound, ManagerMultisigs, ManagerVotesInCurrentRound,
	Pallet,
};
use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{ensure, traits::Get, PalletId};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Bounded, Dispatchable, IdentifyAccount, Verify},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	AccountId32, DispatchResult, MultiSignature, MultiSigner,
};

/// One member's vote for `call`, signed offline and submitted by anyone.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct ManagerMultisigVote<Call> {
	/// The member voting.
	pub who: MultiSigner,
	/// The call voted for.
	pub call: Call,
	/// The round the vote is cast in.
	pub round: u32,
}

pub type ManagerMultisigVoteOf<T> = ManagerMultisigVote<<T as Config>::RuntimeCall>;

impl<Call: Encode> ManagerMultisigVote<Call> {
	/// The bytes a member signs: the vote in the wrapper wallet `signRaw` adds.
	pub fn encode_with_bytes_wrapper(&self) -> Vec<u8> {
		(b"<Bytes>", self, b"</Bytes>").encode()
	}
}

impl<T: Config> Pallet<T> {
	/// The account the multisig dispatches as.
	pub fn manager_multisig_id() -> T::AccountId {
		PalletId(*b"rc2migmt").into_account_truncating()
	}

	/// Record a vote. Once the threshold is met the call is dispatched as the multisig and the
	/// round ends.
	pub(crate) fn do_vote_manager_multisig(
		payload: ManagerMultisigVoteOf<T>,
		sig: &MultiSignature,
	) -> DispatchResult {
		let (who, mut votes) = Self::check_vote(&payload, sig)?;
		votes.push(who.clone());

		if votes.len() < T::MultisigThreshold::get() as usize {
			ManagerVotesInCurrentRound::<T>::mutate(&who, |n| *n = n.saturating_add(1));
			Self::deposit_event(Event::ManagerMultisigVoted { votes: votes.len() as u32 });
			ManagerMultisigs::<T>::insert(&payload.call, votes);
			return Ok(());
		}

		// dispatch as the multisig and end the round
		let origin: <T as frame_system::Config>::RuntimeOrigin =
			frame_system::RawOrigin::Signed(Self::manager_multisig_id()).into();
		let res = payload.call.dispatch(origin);
		let _ = ManagerMultisigs::<T>::clear(u32::MAX, None);
		let _ = ManagerVotesInCurrentRound::<T>::clear(u32::MAX, None);
		ManagerMultisigRound::<T>::mutate(|round| *round = round.saturating_add(1));
		Self::deposit_event(Event::ManagerMultisigDispatched {
			res: res.map(|_| ()).map_err(|e| e.error),
		});
		Ok(())
	}

	/// Whether a vote may enter the transaction pool. One vote per member is kept there.
	pub(crate) fn validate_vote(
		payload: &ManagerMultisigVoteOf<T>,
		sig: &MultiSignature,
	) -> TransactionValidity {
		let (who, _) = Self::check_vote(payload, sig).map_err(|e| match e {
			Error::<T>::NotMultisigMember => InvalidTransaction::BadSigner,
			Error::<T>::BadVoteSignature => InvalidTransaction::BadProof,
			Error::<T>::RoundStale | Error::<T>::MaxVotesPerRound | Error::<T>::DuplicateVote =>
				InvalidTransaction::Stale,
			// `check_vote` returns nothing else
			_ => InvalidTransaction::Call,
		})?;
		ValidTransaction::with_tag_prefix("Ahm2Multisig")
			.priority(Bounded::max_value())
			.and_provides(vec![("ahm2_multi", who).encode()])
			.propagate(true)
			.longevity(30)
			.build()
	}

	/// A member's own signature over the vote, for the current round, with votes left and none
	/// cast for this call yet. The signature is checked last. Returns the member and the votes
	/// cast for the call so far.
	fn check_vote(
		payload: &ManagerMultisigVoteOf<T>,
		sig: &MultiSignature,
	) -> Result<(AccountId32, Vec<AccountId32>), Error<T>> {
		let who = payload.who.clone().into_account();
		ensure!(T::MultisigMembers::get().contains(&who), Error::<T>::NotMultisigMember);
		ensure!(ManagerMultisigRound::<T>::get() == payload.round, Error::<T>::RoundStale);
		ensure!(
			ManagerVotesInCurrentRound::<T>::get(&who) < T::MultisigMaxVotesPerRound::get(),
			Error::<T>::MaxVotesPerRound
		);
		let votes = ManagerMultisigs::<T>::get(&payload.call);
		ensure!(!votes.contains(&who), Error::<T>::DuplicateVote);
		ensure!(
			sig.verify(&payload.encode_with_bytes_wrapper()[..], &who),
			Error::<T>::BadVoteSignature
		);
		Ok((who, votes))
	}
}
