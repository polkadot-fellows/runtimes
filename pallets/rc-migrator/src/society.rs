// Copyright (C) Parity Technologies (UK) Ltd.
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
// along with Polkadot.  If not, see <http://www.gnu.org/licenses/>.

use super::*;
use crate::types::TranslateAccounts;
use frame_support::traits::{Currency, DefensiveTruncateInto};

pub const MAX_PAYOUTS: u32 = 8;

/// Stage of the society pallet migration.
#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	Default,
	Debug,
	TypeInfo,
	MaxEncodedLen,
	PartialEq,
	Eq,
)]
pub enum SocietyStage {
	#[default]
	Values,
	Members(Option<AccountId32>),
	Payouts(Option<AccountId32>),
	MemberByIndex(Option<u32>),
	SuspendedMembers(Option<AccountId32>),
	Candidates(Option<AccountId32>),
	Votes(Option<(AccountId32, AccountId32)>),
	VoteClearCursor(Option<AccountId32>),
	DefenderVotes(Option<(u32, AccountId32)>),
	Finished,
}

/// Data transfer message that is being sent to the AH Migrator.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, Debug, TypeInfo, PartialEq, Eq)]
pub enum PortableSocietyMessage {
	Values(SocietyValues),
	Member(AccountId32, PortableMemberRecord),
	Payout(AccountId32, PortablePayoutRecord),
	MemberByIndex(u32, AccountId32),
	SuspendedMembers(AccountId32, PortableMemberRecord),
	Candidates(AccountId32, PortableCandidacy),
	Votes(AccountId32, AccountId32, PortableVote),
	VoteClearCursor(AccountId32, Vec<u8>),
	DefenderVotes(u32, AccountId32, PortableVote),
}

impl TranslateAccounts for PortableSocietyMessage {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		use PortableSocietyMessage::*;
		match self {
			Values(values) => Values(values.translate_accounts(f)),
			Member(account, member) => Member(f(account), member),
			Payout(account, payout) => Payout(f(account), payout),
			MemberByIndex(index, account) => MemberByIndex(index, f(account)),
			SuspendedMembers(account, member) => SuspendedMembers(f(account), member),
			Candidates(account, candidacy) =>
				Candidates(f(account), candidacy.translate_accounts(f)),
			Votes(account1, account2, vote) => Votes(f(account1), f(account2), vote),
			VoteClearCursor(account, cursor) => VoteClearCursor(f(account), cursor),
			DefenderVotes(index, account, vote) => DefenderVotes(index, f(account), vote),
		}
	}
}

/// Society storage values.
#[derive(Encode, Decode, DecodeWithMemTracking, TypeInfo, Debug, Clone, PartialEq, Eq)]
pub struct SocietyValues {
	pub parameters: Option<PortableGroupParams>,
	pub pot: Option<u128>,
	pub founder: Option<AccountId32>,
	pub head: Option<AccountId32>,
	pub rules: Option<H256>,
	pub member_count: Option<u32>,
	pub round_count: Option<u32>,
	pub bids: Option<Vec<PortableBid>>,
	pub sceptic: Option<AccountId32>,
	pub next_head: Option<PortableIntakeRecord>,
	pub challenge_round_count: Option<u32>,
	pub defending: Option<(AccountId32, AccountId32, PortableTally)>,
}

impl TranslateAccounts for SocietyValues {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		Self {
			parameters: self.parameters,
			pot: self.pot,
			founder: self.founder.map(f),
			head: self.head.map(f),
			rules: self.rules,
			member_count: self.member_count,
			round_count: self.round_count,
			bids: self.bids.map(|bids| {
				bids.into_iter().map(|bid| bid.translate_accounts(f)).collect::<Vec<_>>()
			}),
			sceptic: self.sceptic.map(f),
			next_head: self.next_head.map(|next_head| next_head.translate_accounts(f)),
			challenge_round_count: self.challenge_round_count,
			defending: self
				.defending
				.map(|defending| (f(defending.0), f(defending.1), defending.2)),
		}
	}
}

impl SocietyValues {
	pub fn take_values<T>() -> Self
	where
		T: pallet_society::Config,
		<T as pallet_society::Config>::Currency: Currency<T::AccountId, Balance = u128>,
		T: frame_system::Config<AccountId = AccountId32, Hash = sp_core::H256>,
	{
		use pallet_society::*;

		SocietyValues {
			parameters: Parameters::<T>::take().map(|p| p.into_portable()),
			pot: Pot::<T>::exists().then(Pot::<T>::take),
			founder: Founder::<T>::take(),
			head: Head::<T>::take(),
			rules: Rules::<T>::take(),
			member_count: MemberCount::<T>::exists().then(MemberCount::<T>::take),
			round_count: RoundCount::<T>::exists().then(RoundCount::<T>::take),
			bids: Bids::<T>::exists()
				.then(Bids::<T>::take)
				.map(|bids| bids.into_iter().map(|bid| bid.into_portable()).collect::<Vec<_>>()),
			sceptic: Skeptic::<T>::take(),
			next_head: NextHead::<T>::take().map(|next_head| next_head.into_portable()),
			challenge_round_count: ChallengeRoundCount::<T>::exists()
				.then(ChallengeRoundCount::<T>::take),
			defending: Defending::<T>::take()
				.map(|(a, b, portable_tally)| (a, b, portable_tally.into_portable())),
		}
	}

	pub fn put_values<T>(values: Self)
	where
		T: pallet_society::Config,
		<T as pallet_society::Config>::Currency: Currency<T::AccountId, Balance = u128>,
		T: frame_system::Config<AccountId = AccountId32, Hash = sp_core::H256>,
	{
		use pallet_society::*;

		values
			.parameters
			.map(|p| Parameters::<T>::put::<pallet_society::GroupParams<u128>>(p.into()));
		values.pot.map(Pot::<T>::put);
		values.founder.map(Founder::<T>::put);
		values.head.map(Head::<T>::put);
		values.rules.map(Rules::<T>::put);
		values.member_count.map(MemberCount::<T>::put);
		values.round_count.map(RoundCount::<T>::put);
		values.bids.map(|bids| {
			Bids::<T>::put(BoundedVec::defensive_truncate_from(
				bids.into_iter().map(|bid| bid.into()).collect::<Vec<_>>(),
			))
		});
		values.sceptic.map(Skeptic::<T>::put);
		values.next_head.map(|next_head| {
			NextHead::<T>::put::<pallet_society::IntakeRecord<AccountId32, u128>>(next_head.into())
		});
		values.challenge_round_count.map(ChallengeRoundCount::<T>::put);
		values.defending.map(|(account1, account2, portable_tally)| {
			Defending::<T>::put::<(AccountId32, AccountId32, pallet_society::Tally)>((
				account1,
				account2,
				portable_tally.into(),
			))
		});
	}
}

#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct PortableGroupParams {
	pub max_members: u32,
	pub max_intake: u32,
	pub max_strikes: u32,
	pub candidate_deposit: u128,
}

impl IntoPortable for pallet_society::GroupParams<u128> {
	type Portable = PortableGroupParams;
	fn into_portable(self) -> Self::Portable {
		PortableGroupParams {
			max_members: self.max_members,
			max_intake: self.max_intake,
			max_strikes: self.max_strikes,
			candidate_deposit: self.candidate_deposit,
		}
	}
}

impl Into<pallet_society::GroupParams<u128>> for PortableGroupParams {
	fn into(self) -> pallet_society::GroupParams<u128> {
		pallet_society::GroupParams {
			max_members: self.max_members,
			max_intake: self.max_intake,
			max_strikes: self.max_strikes,
			candidate_deposit: self.candidate_deposit,
		}
	}
}

/// Portable version of the [pallet_society::Bid].
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct PortableBid {
	/// The bidder/candidate trying to enter society
	pub who: AccountId32,
	/// The kind of bid placed for this bidder/candidate. See `BidKind`.
	pub kind: PortableBidKind,
	/// The reward that the bidder has requested for successfully joining the society.
	pub value: u128,
}

impl TranslateAccounts for PortableBid {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableBid { who: f(self.who), kind: self.kind.translate_accounts(f), value: self.value }
	}
}

impl IntoPortable for pallet_society::Bid<AccountId32, u128> {
	type Portable = PortableBid;
	fn into_portable(self) -> Self::Portable {
		PortableBid { who: self.who, kind: self.kind.into_portable(), value: self.value }
	}
}

impl Into<pallet_society::Bid<AccountId32, u128>> for PortableBid {
	fn into(self) -> pallet_society::Bid<AccountId32, u128> {
		pallet_society::Bid { who: self.who, kind: self.kind.into(), value: self.value }
	}
}

/// Record for an individual new member who was elevated from a candidate recently.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct PortableIntakeRecord {
	pub who: AccountId32,
	pub bid: u128,
	pub round: u32,
}

impl TranslateAccounts for PortableIntakeRecord {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableIntakeRecord { who: f(self.who), bid: self.bid, round: self.round }
	}
}

impl IntoPortable for pallet_society::IntakeRecord<AccountId32, u128> {
	type Portable = PortableIntakeRecord;
	fn into_portable(self) -> Self::Portable {
		PortableIntakeRecord { who: self.who, bid: self.bid, round: self.round }
	}
}

impl Into<pallet_society::IntakeRecord<AccountId32, u128>> for PortableIntakeRecord {
	fn into(self) -> pallet_society::IntakeRecord<AccountId32, u128> {
		pallet_society::IntakeRecord { who: self.who, bid: self.bid, round: self.round }
	}
}

/// Information concerning a member.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct PortableMemberRecord {
	pub rank: u32,
	pub strikes: u32,
	pub vouching: Option<PortableVouchingStatus>,
	pub index: u32,
}

/// Portable version of the [pallet_society::VouchingStatus] of a vouching member.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum PortableVouchingStatus {
	/// Member is currently vouching for a user.
	Vouching,
	/// Member is banned from vouching for other members.
	Banned,
}

impl IntoPortable for pallet_society::MemberRecord {
	type Portable = PortableMemberRecord;

	fn into_portable(self) -> Self::Portable {
		PortableMemberRecord {
			rank: self.rank,
			strikes: self.strikes,
			vouching: self.vouching.map(|v| match v {
				pallet_society::VouchingStatus::Vouching => PortableVouchingStatus::Vouching,
				pallet_society::VouchingStatus::Banned => PortableVouchingStatus::Banned,
			}),
			index: self.index,
		}
	}
}

impl Into<pallet_society::MemberRecord> for PortableMemberRecord {
	fn into(self) -> pallet_society::MemberRecord {
		pallet_society::MemberRecord {
			rank: self.rank,
			strikes: self.strikes,
			vouching: self.vouching.map(|v| match v {
				PortableVouchingStatus::Vouching => pallet_society::VouchingStatus::Vouching,
				PortableVouchingStatus::Banned => pallet_society::VouchingStatus::Banned,
			}),
			index: self.index,
		}
	}
}

/// Portable version of the [pallet_society::PayoutRecord] of a member.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo, Default)]
pub struct PortablePayoutRecord {
	pub paid: u128,
	pub payouts: Vec<(u32, u128)>,
}

impl IntoPortable
	for pallet_society::PayoutRecord<u128, BoundedVec<(u32, u128), ConstU32<MAX_PAYOUTS>>>
{
	type Portable = PortablePayoutRecord;

	fn into_portable(self) -> Self::Portable {
		PortablePayoutRecord { paid: self.paid, payouts: self.payouts.into_inner() }
	}
}

impl Into<pallet_society::PayoutRecord<u128, BoundedVec<(u32, u128), ConstU32<MAX_PAYOUTS>>>>
	for PortablePayoutRecord
{
	fn into(
		self,
	) -> pallet_society::PayoutRecord<u128, BoundedVec<(u32, u128), ConstU32<MAX_PAYOUTS>>> {
		pallet_society::PayoutRecord {
			paid: self.paid,
			payouts: self.payouts.defensive_truncate_into(),
		}
	}
}

/// Portable version of the [pallet_society::Candidacy] of a candidate.
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct PortableCandidacy {
	/// The index of the round where the candidacy began.
	pub round: u32,
	/// The kind of bid placed for this bidder/candidate. See `BidKind`.
	pub kind: PortableBidKind,
	/// The reward that the bidder has requested for successfully joining the society.
	pub bid: u128,
	/// The tally of votes so far.
	pub tally: PortableTally,
	/// True if the skeptic was already punished for note voting.
	pub skeptic_struck: bool,
}

impl TranslateAccounts for PortableCandidacy {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		PortableCandidacy {
			round: self.round,
			kind: self.kind.translate_accounts(f),
			bid: self.bid,
			tally: self.tally,
			skeptic_struck: self.skeptic_struck,
		}
	}
}

impl IntoPortable for pallet_society::Candidacy<AccountId32, u128> {
	type Portable = PortableCandidacy;

	fn into_portable(self) -> Self::Portable {
		PortableCandidacy {
			round: self.round,
			kind: self.kind.into_portable(),
			bid: self.bid,
			tally: self.tally.into_portable(),
			skeptic_struck: self.skeptic_struck,
		}
	}
}

impl Into<pallet_society::Candidacy<AccountId32, u128>> for PortableCandidacy {
	fn into(self) -> pallet_society::Candidacy<AccountId32, u128> {
		pallet_society::Candidacy {
			round: self.round,
			kind: self.kind.into(),
			bid: self.bid,
			tally: self.tally.into(),
			skeptic_struck: self.skeptic_struck,
		}
	}
}

/// Portable version of the [pallet_society::BidKind].
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub enum PortableBidKind {
	/// The given deposit was paid for this bid.
	Deposit(u128),
	/// A member vouched for this bid. The account should be reinstated into `Members` once the
	/// bid is successful (or if it is rescinded prior to launch).
	Vouch(AccountId32, u128),
}

impl TranslateAccounts for PortableBidKind {
	fn translate_accounts(self, f: &impl Fn(AccountId32) -> AccountId32) -> Self {
		use PortableBidKind::*;
		match self {
			Deposit(deposit) => Deposit(deposit),
			Vouch(account, deposit) => Vouch(f(account), deposit),
		}
	}
}

impl IntoPortable for pallet_society::BidKind<AccountId32, u128> {
	type Portable = PortableBidKind;

	fn into_portable(self) -> Self::Portable {
		use pallet_society::BidKind::*;
		match self {
			Deposit(deposit) => PortableBidKind::Deposit(deposit),
			Vouch(account, deposit) => PortableBidKind::Vouch(account, deposit),
		}
	}
}

impl Into<pallet_society::BidKind<AccountId32, u128>> for PortableBidKind {
	fn into(self) -> pallet_society::BidKind<AccountId32, u128> {
		use PortableBidKind::*;
		match self {
			Deposit(deposit) => pallet_society::BidKind::Deposit(deposit),
			Vouch(account, deposit) => pallet_society::BidKind::Vouch(account, deposit),
		}
	}
}

/// Portable version of the [pallet_society::Tally].
#[derive(Default, Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct PortableTally {
	/// The approval votes.
	pub approvals: u32,
	/// The rejection votes.
	pub rejections: u32,
}

impl IntoPortable for pallet_society::Tally {
	type Portable = PortableTally;
	fn into_portable(self) -> Self::Portable {
		PortableTally { approvals: self.approvals, rejections: self.rejections }
	}
}

impl Into<pallet_society::Tally> for PortableTally {
	fn into(self) -> pallet_society::Tally {
		pallet_society::Tally { approvals: self.approvals, rejections: self.rejections }
	}
}

/// Portable version of the [pallet_society::Vote].
#[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
pub struct PortableVote {
	pub approve: bool,
	pub weight: u32,
}

impl IntoPortable for pallet_society::Vote {
	type Portable = PortableVote;
	fn into_portable(self) -> Self::Portable {
		PortableVote { approve: self.approve, weight: self.weight }
	}
}

impl Into<pallet_society::Vote> for PortableVote {
	fn into(self) -> pallet_society::Vote {
		pallet_society::Vote { approve: self.approve, weight: self.weight }
	}
}

pub struct SocietyMigrator<T>(PhantomData<T>);

impl<T: Config> PalletMigration for SocietyMigrator<T> {
	type Key = SocietyStage;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut last_key = last_key.unwrap_or(SocietyStage::Values);
		let mut messages = XcmBatchAndMeter::new_from_config::<T>();

		loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 1)).is_err() ||
				weight_counter.try_consume(messages.consume_weight()).is_err()
			{
				log::info!("RC weight limit reached at batch length {}, stopping", messages.len());
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			// TODO replace `receive_vesting_schedules` by actual ah weight func
			if T::MaxAhWeight::get()
				.any_lt(T::AhWeightInfo::receive_vesting_schedules((messages.len() + 1) as u32))
			{
				log::info!("AH weight limit reached at batch length {}, stopping", messages.len());
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > MAX_ITEMS_PER_BLOCK {
				log::info!(
                    "Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
                    MAX_ITEMS_PER_BLOCK,
                    messages.len()
                );
				break;
			}

			last_key = match last_key {
				SocietyStage::Values => {
					weight_counter.consume(T::DbWeight::get().writes(12));
					let values = SocietyValues::take_values::<T::KusamaConfig>();
					messages.push(PortableSocietyMessage::Values(values));
					SocietyStage::Members(None)
				},
				SocietyStage::Members(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_society::Members::<T::KusamaConfig>::iter_from_key(last_key)
					} else {
						pallet_society::Members::<T::KusamaConfig>::iter()
					};

					match iter.next() {
						Some((key, value)) => {
							pallet_society::Members::<T::KusamaConfig>::remove(&key);
							messages.push(PortableSocietyMessage::Member(
								key.clone(),
								value.into_portable(),
							));
							SocietyStage::Members(Some(key))
						},
						None => SocietyStage::Payouts(None),
					}
				},
				SocietyStage::Payouts(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_society::Payouts::<T::KusamaConfig>::iter_from_key(last_key)
					} else {
						pallet_society::Payouts::<T::KusamaConfig>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							pallet_society::Payouts::<T::KusamaConfig>::remove(&key);
							messages.push(PortableSocietyMessage::Payout(
								key.clone(),
								value.into_portable(),
							));
							SocietyStage::Payouts(Some(key))
						},
						None => SocietyStage::MemberByIndex(None),
					}
				},
				SocietyStage::MemberByIndex(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_society::MemberByIndex::<T::KusamaConfig>::iter_from_key(last_key)
					} else {
						pallet_society::MemberByIndex::<T::KusamaConfig>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							pallet_society::MemberByIndex::<T::KusamaConfig>::remove(&key);
							messages.push(PortableSocietyMessage::MemberByIndex(key, value));
							SocietyStage::MemberByIndex(Some(key))
						},
						None => SocietyStage::SuspendedMembers(None),
					}
				},
				SocietyStage::SuspendedMembers(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_society::SuspendedMembers::<T::KusamaConfig>::iter_from_key(last_key)
					} else {
						pallet_society::SuspendedMembers::<T::KusamaConfig>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							pallet_society::SuspendedMembers::<T::KusamaConfig>::remove(&key);
							messages.push(PortableSocietyMessage::SuspendedMembers(
								key.clone(),
								value.into_portable(),
							));
							SocietyStage::SuspendedMembers(Some(key))
						},
						None => SocietyStage::Candidates(None),
					}
				},
				SocietyStage::Candidates(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_society::Candidates::<T::KusamaConfig>::iter_from_key(last_key)
					} else {
						pallet_society::Candidates::<T::KusamaConfig>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							pallet_society::Candidates::<T::KusamaConfig>::remove(&key);
							messages.push(PortableSocietyMessage::Candidates(
								key.clone(),
								value.into_portable(),
							));
							SocietyStage::Candidates(Some(key))
						},
						None => SocietyStage::Votes(None),
					}
				},
				SocietyStage::Votes(last_key) => {
					let mut iter = if let Some((key1, key2)) = last_key {
						pallet_society::Votes::<T::KusamaConfig>::iter_from(
							pallet_society::Votes::<T::KusamaConfig>::hashed_key_for(key1, key2),
						)
					} else {
						pallet_society::Votes::<T::KusamaConfig>::iter()
					};
					match iter.next() {
						Some((key1, key2, value)) => {
							pallet_society::Votes::<T::KusamaConfig>::remove(&key1, &key2);
							messages.push(PortableSocietyMessage::Votes(
								key1.clone(),
								key2.clone(),
								value.into_portable(),
							));
							SocietyStage::Votes(Some((key1, key2)))
						},
						None => SocietyStage::VoteClearCursor(None),
					}
				},
				SocietyStage::VoteClearCursor(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_society::VoteClearCursor::<T::KusamaConfig>::iter_from_key(last_key)
					} else {
						pallet_society::VoteClearCursor::<T::KusamaConfig>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							pallet_society::VoteClearCursor::<T::KusamaConfig>::remove(&key);
							messages.push(PortableSocietyMessage::VoteClearCursor(
								key.clone(),
								value.to_vec(),
							));
							SocietyStage::VoteClearCursor(Some(key))
						},
						None => SocietyStage::DefenderVotes(None),
					}
				},
				SocietyStage::DefenderVotes(last_key) => {
					let mut iter = if let Some((key1, key2)) = last_key {
						pallet_society::DefenderVotes::<T::KusamaConfig>::iter_from(
							pallet_society::DefenderVotes::<T::KusamaConfig>::hashed_key_for(
								key1, key2,
							),
						)
					} else {
						pallet_society::DefenderVotes::<T::KusamaConfig>::iter()
					};
					match iter.next() {
						Some((key1, key2, value)) => {
							pallet_society::DefenderVotes::<T::KusamaConfig>::remove(&key1, &key2);
							messages.push(PortableSocietyMessage::DefenderVotes(
								key1,
								key2.clone(),
								value.into_portable(),
							));
							SocietyStage::DefenderVotes(Some((key1, key2)))
						},
						None => SocietyStage::Finished,
					}
				},
				SocietyStage::Finished => {
					break;
				},
			}
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(messages.into_inner(), |messages| {
				types::AhMigratorCall::<T>::ReceiveSocietyMessages { messages }
			})?;
		}

		if last_key == SocietyStage::Finished {
			log::info!(target: LOG_TARGET, "Society migration finished");
			Ok(None)
		} else {
			log::info!(
				target: LOG_TARGET,
				"Society migration iteration stopped at {:?}",
				&last_key
			);
			Ok(Some(last_key))
		}
	}
}
