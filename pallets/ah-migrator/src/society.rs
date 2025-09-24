// This file is part of Substrate.

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

use crate::*;
use frame_support::storage::KeyLenOf;
use pallet_rc_migrator::types::TranslateAccounts;

impl<T: Config> Pallet<T> {
	pub fn do_receive_society_messages(
		messages: Vec<PortableSocietyMessage>,
	) -> Result<(), Error<T>> {
		log::info!("Received {} society messages", messages.len());

		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Society,
			count: messages.len() as u32,
		});

		for message in &messages {
			Self::do_receive_society_message(message.clone());
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Society,
			count_good: messages.len() as u32,
			count_bad: 0,
		});

		log::info!("Processed {} society messages", messages.len());

		Ok(())
	}

	pub fn do_receive_society_message(message: PortableSocietyMessage) {
		log::debug!(target: LOG_TARGET, "Processing society message: {message:?}");

		let message = message.translate_accounts(&Self::translate_account_rc_to_ah);

		match message {
			PortableSocietyMessage::Values(values) => {
				log::debug!(target: LOG_TARGET, "Integrating society values: {values:?}");
				pallet_rc_migrator::society::SocietyValues::put_values::<T::KusamaConfig>(*values);
			},
			PortableSocietyMessage::Member(account, member) => {
				log::debug!(target: LOG_TARGET, "Integrating society member: {account:?}, {member:?}");
				let member: pallet_society::MemberRecord = member.into();
				pallet_society::Members::<T::KusamaConfig>::insert(account, member);
			},
			PortableSocietyMessage::Payout(account, payout) => {
				log::debug!(target: LOG_TARGET, "Integrating society payout: {account:?}, {payout:?}");
				let payout: pallet_society::PayoutRecord<_, _> = payout.into();
				pallet_society::Payouts::<T::KusamaConfig>::insert(account, payout);
			},
			PortableSocietyMessage::MemberByIndex(index, account) => {
				log::debug!(target: LOG_TARGET, "Integrating society member by index: {index:?}, {account:?}");
				pallet_society::MemberByIndex::<T::KusamaConfig>::insert(index, account);
			},
			PortableSocietyMessage::SuspendedMembers(account, member) => {
				log::debug!(target: LOG_TARGET, "Integrating suspended society member: {account:?}, {member:?}");
				let member: pallet_society::MemberRecord = member.into();
				pallet_society::SuspendedMembers::<T::KusamaConfig>::insert(account, member);
			},
			PortableSocietyMessage::Candidates(account, candidacy) => {
				log::debug!(target: LOG_TARGET, "Integrating society candidate: {account:?}, {candidacy:?}");
				let candidacy: pallet_society::Candidacy<_, _> = candidacy.into();
				pallet_society::Candidates::<T::KusamaConfig>::insert(account, candidacy);
			},
			PortableSocietyMessage::Votes(account1, account2, vote) => {
				log::debug!(target: LOG_TARGET, "Integrating society vote: {account1:?}, {account2:?}, {vote:?}");
				let vote: pallet_society::Vote = vote.into();
				pallet_society::Votes::<T::KusamaConfig>::insert(account1, account2, vote);
			},
			PortableSocietyMessage::VoteClearCursor(account, cursor) => {
				log::debug!(target: LOG_TARGET, "Integrating society vote clear cursor: {account:?}, {cursor:?}");
				pallet_society::VoteClearCursor::<T::KusamaConfig>::insert(
					account,
					BoundedVec::<u8, KeyLenOf<pallet_society::Votes<T::KusamaConfig>>>::defensive_truncate_from(cursor),
				);
			},
			PortableSocietyMessage::DefenderVotes(index, account, vote) => {
				log::debug!(target: LOG_TARGET, "Integrating society defender vote: {index:?}, {account:?}, {vote:?}");
				let vote: pallet_society::Vote = vote.into();
				pallet_society::DefenderVotes::<T::KusamaConfig>::insert(index, account, vote);
			},
		}

		log::debug!(target: LOG_TARGET, "Processed society message");
	}
}

#[cfg(feature = "std")]
pub mod tests {
	use super::*;
	use pallet_rc_migrator::society::tests::{RcPrePayload, SocietyMigratorTest};

	impl<T: Config> crate::types::AhMigrationCheck for SocietyMigratorTest<T> {
		type RcPrePayload = RcPrePayload;
		type AhPrePayload = ();

		fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
			use pallet_society::*;

			assert!(
				Parameters::<T::KusamaConfig>::get().is_none(),
				"Parameters should be None on the relay chain after migration"
			);

			assert!(
				!Pot::<T::KusamaConfig>::exists(),
				"Pot should be empty on the relay chain after migration"
			);

			assert!(
				Founder::<T::KusamaConfig>::get().is_none(),
				"Founder should be None on the relay chain after migration"
			);

			assert!(
				Head::<T::KusamaConfig>::get().is_none(),
				"Head should be None on the relay chain after migration"
			);

			assert!(
				Rules::<T::KusamaConfig>::get().is_none(),
				"Rules should be None on the relay chain after migration"
			);

			assert!(
				!MemberCount::<T::KusamaConfig>::exists(),
				"MemberCount should be empty on the relay chain after migration"
			);

			assert!(
				!RoundCount::<T::KusamaConfig>::exists(),
				"RoundCount should be empty on the relay chain after migration"
			);

			assert!(
				!Bids::<T::KusamaConfig>::exists(),
				"Bids should be empty on the relay chain after migration"
			);

			assert!(
				Skeptic::<T::KusamaConfig>::get().is_none(),
				"Skeptic should be None on the relay chain after migration"
			);

			assert!(
				NextHead::<T::KusamaConfig>::get().is_none(),
				"NextHead should be None on the relay chain after migration"
			);

			assert!(
				!ChallengeRoundCount::<T::KusamaConfig>::exists(),
				"ChallengeRoundCount should be empty on the relay chain after migration"
			);

			assert!(
				Defending::<T::KusamaConfig>::get().is_none(),
				"Defending should be None on the relay chain after migration"
			);

			assert!(
				Members::<T::KusamaConfig>::iter().next().is_none(),
				"Members map should be empty on the relay chain after migration"
			);

			assert!(
				Payouts::<T::KusamaConfig>::iter().next().is_none(),
				"Payouts map should be empty on the relay chain after migration"
			);

			assert!(
				MemberByIndex::<T::KusamaConfig>::iter().next().is_none(),
				"MemberByIndex map should be empty on the relay chain after migration"
			);

			assert!(
				SuspendedMembers::<T::KusamaConfig>::iter().next().is_none(),
				"SuspendedMembers map should be empty on the relay chain after migration"
			);

			assert!(
				Candidates::<T::KusamaConfig>::iter().next().is_none(),
				"Candidates map should be empty on the relay chain after migration"
			);

			assert!(
				Votes::<T::KusamaConfig>::iter().next().is_none(),
				"Votes map should be empty on the relay chain after migration"
			);

			assert!(
				VoteClearCursor::<T::KusamaConfig>::iter().next().is_none(),
				"VoteClearCursor map should be empty on the relay chain after migration"
			);

			assert!(
				DefenderVotes::<T::KusamaConfig>::iter().next().is_none(),
				"DefenderVotes map should be empty on the relay chain after migration"
			);

			// if let Some(next_challenge_at) = NextChallengeAt::<T::KusamaConfig>::get() {
			// 	let challenge_period =
			// 		<T::KusamaConfig as pallet_society::Config>::ChallengePeriod::get();
			// 	assert_eq!(
			// 		next_challenge_at, challenge_period,
			// 		"`next_challenge_at` must be equal to the `ChallengePeriod` if not `None`",
			// 	);
			// };

			// if let Some(next_intake_at) = NextIntakeAt::<T::KusamaConfig>::get() {
			// let rotation_period =
			// <T::KusamaConfig as pallet_society::Config>::VotingPeriod::get()
			// .saturating_add(
			// <T::KusamaConfig as pallet_society::Config>::ClaimPeriod::get(),
			// );
			// assert_eq!(
			// next_intake_at, rotation_period,
			// "`next_intake_at` must be equal to the rotation period if not `None`",
			// );
			// };
		}

		fn post_check(rc_payload: Self::RcPrePayload, _: Self::AhPrePayload) {
			use pallet_society::*;

			assert_eq!(
				Parameters::<T::KusamaConfig>::get(),
				rc_payload.parameters,
				"Parameters should match the pre migration RC value"
			);

			assert_eq!(
				Pot::<T::KusamaConfig>::get(),
				rc_payload.pot,
				"Pot should match the pre migration RC value"
			);

			assert_eq!(
				Founder::<T::KusamaConfig>::get(),
				rc_payload.founder,
				"Founder should match the pre migration RC value"
			);

			assert_eq!(
				Head::<T::KusamaConfig>::get(),
				rc_payload.head,
				"Head should match the pre migration RC value"
			);

			assert_eq!(
				Rules::<T::KusamaConfig>::get(),
				rc_payload.rules,
				"Rules should match the pre migration RC value"
			);

			assert_eq!(
				MemberCount::<T::KusamaConfig>::get(),
				rc_payload.member_count,
				"MemberCount should match the pre migration RC value"
			);

			assert_eq!(
				RoundCount::<T::KusamaConfig>::get(),
				rc_payload.round_count,
				"RoundCount should match the pre migration RC value"
			);

			assert_eq!(
				Bids::<T::KusamaConfig>::get().into_inner(),
				rc_payload.bids,
				"Bids should match the pre migration RC value"
			);

			assert_eq!(
				Skeptic::<T::KusamaConfig>::get(),
				rc_payload.skeptic,
				"Skeptic should match the pre migration RC value"
			);

			assert_eq!(
				NextHead::<T::KusamaConfig>::get(),
				rc_payload.next_head,
				"NextHead should match the pre migration RC value"
			);

			assert_eq!(
				ChallengeRoundCount::<T::KusamaConfig>::get(),
				rc_payload.challenge_round_count,
				"ChallengeRoundCount should match the pre migration RC value"
			);

			assert_eq!(
				Defending::<T::KusamaConfig>::get(),
				rc_payload.defending,
				"Defending should match the pre migration RC value"
			);

			assert_eq!(
				NextIntakeAt::<T::KusamaConfig>::get(),
				rc_payload.next_intake_at,
				"NextIntakeAt should match the pre migration RC value"
			);

			assert_eq!(
				NextChallengeAt::<T::KusamaConfig>::get(),
				rc_payload.next_challenge_at,
				"NextChallengeAt should match the pre migration RC value"
			);

			assert_eq!(
				Members::<T::KusamaConfig>::iter().collect::<Vec<_>>(),
				rc_payload.members,
				"Members should match the pre migration RC value"
			);

			assert_eq!(
				Payouts::<T::KusamaConfig>::iter().collect::<Vec<_>>(),
				rc_payload.payouts,
				"Payouts should match the pre migration RC value"
			);

			assert_eq!(
				MemberByIndex::<T::KusamaConfig>::iter().collect::<Vec<_>>(),
				rc_payload.member_by_index,
				"MemberByIndex should match the pre migration RC value"
			);

			assert_eq!(
				SuspendedMembers::<T::KusamaConfig>::iter().collect::<Vec<_>>(),
				rc_payload.suspended_members,
				"SuspendedMembers should match the pre migration RC value"
			);

			assert_eq!(
				Candidates::<T::KusamaConfig>::iter().collect::<Vec<_>>(),
				rc_payload.candidates,
				"Candidates should match the pre migration RC value"
			);

			assert_eq!(
				Votes::<T::KusamaConfig>::iter().collect::<Vec<_>>(),
				rc_payload.votes,
				"Votes should match the pre migration RC value"
			);
			assert_eq!(
				VoteClearCursor::<T::KusamaConfig>::iter()
					.map(|(key, value)| (key, value.into_inner()))
					.collect::<Vec<_>>(),
				rc_payload.vote_clear_cursor,
				"VoteClearCursor should match the pre migration RC value"
			);

			assert_eq!(
				DefenderVotes::<T::KusamaConfig>::iter().collect::<Vec<_>>(),
				rc_payload.defender_votes,
				"DefenderVotes should match the pre migration RC value"
			);
		}
	}
}
