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

//! The manager multisig: `Config::MultisigMembers` vote by unsigned extrinsic, and once
//! `Config::MultisigThreshold` of them have voted for the same call it is dispatched as the
//! multisig's account. Members need no funded account on this chain, which is the point: this
//! chain is being drained.
//!
//! A vote is signed offline over (who, call, round), with the `<Bytes>` wrapper that wallets
//! prepend to `signRaw`. The round advances on every dispatch, so an old round's signatures
//! cannot be replayed; each network starts its counter at `Config::MultisigStartRound`, so two
//! chains at the same round do not accept each other's votes. Each member has
//! `Config::MultisigMaxVotesPerRound` votes per round, which bounds what the unsigned path lets
//! one member put into blocks.
//!
//! The `vote_manager_multisig` call, its `ValidateUnsigned` and accepting the multisig's account
//! wherever the manager is accepted are the stage machine's; [`ManagerMultisig::vote`] and
//! [`ManagerMultisig::validate_unsigned`] are what they invoke, and
//! [`ManagerMultisig::init_round`] runs from `on_runtime_upgrade`.

use crate::{
	Config, Event, ManagerMultisigRound, ManagerMultisigs, ManagerVotesInCurrentRound, Pallet,
};
use alloc::{vec, vec::Vec};
use codec::{Decode, DecodeWithMemTracking, Encode};
use core::marker::PhantomData;
use frame_support::{
	traits::Get, CloneNoBound, DebugNoBound, EqNoBound, PalletId, PartialEqNoBound,
};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Bounded, Dispatchable, IdentifyAccount, Verify},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	MultiSignature, MultiSigner,
};

/// Why a vote was refused. The caller maps it to its own error.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// The unsigned multisig vote did not validate.
	UnsignedValidationFailed,
	/// The vote carries a round that is no longer open.
	RoundStale,
	/// The member has used up its votes for this round.
	MaxVotesPerRound,
	/// The member has already voted for this call in this round.
	DuplicateVote,
}

/// One member's vote for `call`, signed offline and submitted by anyone.
#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	DebugNoBound,
	CloneNoBound,
	PartialEqNoBound,
	EqNoBound,
	TypeInfo,
)]
#[scale_info(skip_type_params(T))]
pub struct ManagerMultisigVote<T: Config> {
	pub who: MultiSigner,
	pub call: <T as Config>::RuntimeCall,
	pub round: u32,
}

impl<T: Config> ManagerMultisigVote<T> {
	pub fn new(who: MultiSigner, call: <T as Config>::RuntimeCall, round: u32) -> Self {
		Self { who, call, round }
	}

	/// The bytes a member signs. The wrapper is what wallet `signRaw` prepends.
	pub fn encode_with_bytes_wrapper(&self) -> Vec<u8> {
		(b"<Bytes>", self, b"</Bytes>").encode()
	}
}

pub struct ManagerMultisig<T>(PhantomData<T>);

impl<T: Config> ManagerMultisig<T> {
	/// The account the manager multisig dispatches as, once it reaches its threshold.
	pub fn manager_multisig_id() -> T::AccountId {
		PalletId(*b"rc2migmt").into_account_truncating()
	}

	/// Seed this network's round counter. Idempotent; runs from `on_runtime_upgrade`.
	///
	/// A vote is signed over (who, call, round) and nothing else, so two chains sitting at the
	/// same round would accept each other's signatures. Each network starts its counter
	/// somewhere different.
	pub fn init_round() {
		if !ManagerMultisigRound::<T>::exists() {
			ManagerMultisigRound::<T>::put(T::MultisigStartRound::get());
		}
	}

	/// Vote on behalf of any of the members in `MultisigMembers`.
	///
	/// Each vote adds the member to `ManagerMultisigs` under `payload.call`. Once
	/// [`Config::MultisigThreshold`] members have voted for the same call it is dispatched as the
	/// multisig's account, the map is cleared and the round advances, which is what stops an old
	/// round's signatures from being replayed.
	pub fn vote(payload: &ManagerMultisigVote<T>, sig: &MultiSignature) -> Result<(), Error> {
		Self::validate_unsigned(payload, sig).map_err(|_| Error::UnsignedValidationFailed)?;
		let who = payload.who.clone().into_account();

		if ManagerMultisigRound::<T>::get() != payload.round {
			return Err(Error::RoundStale);
		}
		let num_votes = ManagerVotesInCurrentRound::<T>::get(&who);
		if num_votes >= T::MultisigMaxVotesPerRound::get() {
			return Err(Error::MaxVotesPerRound);
		}
		ManagerVotesInCurrentRound::<T>::insert(&who, num_votes.saturating_add(1));

		let mut votes_for_call = ManagerMultisigs::<T>::get(&payload.call);
		if votes_for_call.contains(&who) {
			return Err(Error::DuplicateVote);
		}
		votes_for_call.push(who);

		if votes_for_call.len() >= T::MultisigThreshold::get() as usize {
			let origin: <T as frame_system::Config>::RuntimeOrigin =
				frame_system::RawOrigin::Signed(Self::manager_multisig_id()).into();
			let res = payload.call.clone().dispatch(origin);
			let _ = ManagerMultisigs::<T>::clear(u32::MAX, None);
			let _ = ManagerVotesInCurrentRound::<T>::clear(u32::MAX, None);
			ManagerMultisigRound::<T>::mutate(|round| *round = round.saturating_add(1));

			Pallet::<T>::deposit_event(Event::ManagerMultisigDispatched {
				res: res.map(|_| ()).map_err(|e| e.error),
			});
		} else {
			Pallet::<T>::deposit_event(Event::ManagerMultisigVoted {
				votes: votes_for_call.len() as u32,
			});
			ManagerMultisigs::<T>::insert(payload.call.clone(), votes_for_call);
		}
		Ok(())
	}

	/// Whether an unsigned vote may enter the pool: a member's own signature over the payload,
	/// for the current round, with votes left this round.
	pub fn validate_unsigned(
		payload: &ManagerMultisigVote<T>,
		sig: &MultiSignature,
	) -> TransactionValidity {
		let account = payload.who.clone().into_account();

		if !T::MultisigMembers::get().contains(&account) {
			return InvalidTransaction::BadSigner.into();
		}
		if !sig.verify(&payload.encode_with_bytes_wrapper()[..], &account) {
			return InvalidTransaction::BadProof.into();
		}
		if ManagerMultisigRound::<T>::get() != payload.round {
			return InvalidTransaction::Stale.into();
		}
		if ManagerVotesInCurrentRound::<T>::get(&account) >= T::MultisigMaxVotesPerRound::get() {
			return InvalidTransaction::Stale.into();
		}

		ValidTransaction::with_tag_prefix("Ahm2Multisig")
			.priority(Bounded::max_value())
			.and_provides(vec![("ahm2_multi", account).encode()])
			.propagate(true)
			.longevity(30)
			.build()
	}
}
