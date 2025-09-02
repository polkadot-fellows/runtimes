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
		log::debug!(target: LOG_TARGET, "Processing society message: {:?}", message);

		let message = message.translate_accounts(&Self::translate_account_rc_to_ah);

		match message {
			PortableSocietyMessage::Values(values) => {
				log::debug!(target: LOG_TARGET, "Integrating society values: {:?}", values);
				pallet_rc_migrator::society::SocietyValues::put_values::<T::KusamaConfig>(values);
			},
			PortableSocietyMessage::Member(account, member) => {
				log::debug!(target: LOG_TARGET, "Integrating society member: {:?}, {:?}", account, member);
				let member: pallet_society::MemberRecord = member.into();
				pallet_society::Members::<T::KusamaConfig>::insert(account, member);
			},
			PortableSocietyMessage::Payout(account, payout) => {
				log::debug!(target: LOG_TARGET, "Integrating society payout: {:?}, {:?}", account, payout);
				let payout: pallet_society::PayoutRecord<_, _> = payout.into();
				pallet_society::Payouts::<T::KusamaConfig>::insert(account, payout);
			},
			PortableSocietyMessage::MemberByIndex(index, account) => {
				log::debug!(target: LOG_TARGET, "Integrating society member by index: {:?}, {:?}", index, account);
				pallet_society::MemberByIndex::<T::KusamaConfig>::insert(index, account);
			},
			PortableSocietyMessage::SuspendedMembers(account, member) => {
				log::debug!(target: LOG_TARGET, "Integrating suspended society member: {:?}, {:?}", account, member);
				let member: pallet_society::MemberRecord = member.into();
				pallet_society::SuspendedMembers::<T::KusamaConfig>::insert(account, member);
			},
			PortableSocietyMessage::Candidates(account, candidacy) => {
				log::debug!(target: LOG_TARGET, "Integrating society candidate: {:?}, {:?}", account, candidacy);
				let candidacy: pallet_society::Candidacy<_, _> = candidacy.into();
				pallet_society::Candidates::<T::KusamaConfig>::insert(account, candidacy);
			},
			PortableSocietyMessage::Votes(account1, account2, vote) => {
				log::debug!(target: LOG_TARGET, "Integrating society vote: {:?}, {:?}, {:?}", account1, account2, vote);
				let vote: pallet_society::Vote = vote.into();
				pallet_society::Votes::<T::KusamaConfig>::insert(account1, account2, vote);
			},
			PortableSocietyMessage::VoteClearCursor(account, cursor) => {
				log::debug!(target: LOG_TARGET, "Integrating society vote clear cursor: {:?}, {:?}", account, cursor);
				pallet_society::VoteClearCursor::<T::KusamaConfig>::insert(
					account,
					BoundedVec::<u8, KeyLenOf<pallet_society::Votes<T::KusamaConfig>>>::defensive_truncate_from(cursor),
				);
			},
			PortableSocietyMessage::DefenderVotes(index, account, vote) => {
				log::debug!(target: LOG_TARGET, "Integrating society defender vote: {:?}, {:?}, {:?}", index, account, vote);
				let vote: pallet_society::Vote = vote.into();
				pallet_society::DefenderVotes::<T::KusamaConfig>::insert(index, account, vote);
			},
		}

		log::debug!(target: LOG_TARGET, "Processed society message");
	}
}
