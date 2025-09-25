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
	Values(Box<SocietyValues>),
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
			Values(values) => Values(Box::new(values.translate_accounts(f))),
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
	pub next_intake_at: Option<u32>,
	pub next_challenge_at: Option<u32>,
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
			next_intake_at: self.next_intake_at,
			next_challenge_at: self.next_challenge_at,
		}
	}
}

impl SocietyValues {
	pub fn take_values<T>() -> Self
	where
		T: pallet_society::Config,
		<T as pallet_society::Config>::Currency: Currency<T::AccountId, Balance = u128>,
		<T as pallet_society::Config>::BlockNumberProvider: BlockNumberProvider<BlockNumber = u32>,
		T: frame_system::Config<AccountId = AccountId32, Hash = sp_core::H256>,
	{
		use pallet_society::*;

		let next_intake_at = if let Some(next_intake_at) = NextIntakeAt::<T>::take() {
			let rotation_period = T::VotingPeriod::get().saturating_add(T::ClaimPeriod::get());
			if next_intake_at != rotation_period {
				Some(next_intake_at)
			} else {
				// current `next_intake_at` is the result of the `on_initialize` execution with
				// disabled rotation. this may happen if this part of migration is executed twice.
				None
			}
		} else {
			None
		};
		let next_challenge_at = if let Some(next_challenge_at) = NextChallengeAt::<T>::take() {
			let challenge_period = T::ChallengePeriod::get();
			if next_challenge_at != challenge_period {
				Some(next_challenge_at)
			} else {
				// current `next_challenge_at` is the result of the `on_initialize` execution with
				// disabled rotation. this may happen if this part of migration is executed twice.
				None
			}
		} else {
			None
		};

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
			next_intake_at,
			next_challenge_at,
		}
	}

	#[allow(clippy::option_map_unit_fn)]
	pub fn put_values<T>(values: Self)
	where
		T: pallet_society::Config,
		<T as pallet_society::Config>::Currency: Currency<T::AccountId, Balance = u128>,
		<T as pallet_society::Config>::BlockNumberProvider: BlockNumberProvider<BlockNumber = u32>,
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
		values.next_intake_at.map(NextIntakeAt::<T>::put);
		values.next_challenge_at.map(NextChallengeAt::<T>::put);
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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

#[allow(clippy::from_over_into)]
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
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					messages.len()
				);
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if T::MaxAhWeight::get()
				.any_lt(Self::receive_society_messages_weight(messages.len() + 1))
			{
				log::info!(
					target: LOG_TARGET,
					"AH weight limit reached at batch length {}, stopping",
					messages.len()
				);
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}

			if messages.len() > MAX_ITEMS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Maximum number of items ({:?}) to migrate per block reached, current batch size: {}",
					MAX_ITEMS_PER_BLOCK,
					messages.len()
				);
				break;
			}

			if messages.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					messages.batch_count()
				);
				break;
			}

			last_key = match last_key {
				SocietyStage::Values => {
					weight_counter.consume(T::DbWeight::get().writes(12));
					let values = SocietyValues::take_values::<T::KusamaConfig>();
					messages.push(PortableSocietyMessage::Values(Box::new(values)));
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
							pallet_society::MemberByIndex::<T::KusamaConfig>::remove(key);
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
							pallet_society::DefenderVotes::<T::KusamaConfig>::remove(key1, &key2);
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

impl<T: Config> SocietyMigrator<T> {
	fn receive_society_messages_weight(messages_len: u32) -> Weight {
		Weight::from_parts(10_000_000, 1000)
			.saturating_add(T::DbWeight::get().writes(1_u64).saturating_mul(messages_len.into()))
	}
}

#[cfg(feature = "std")]
pub mod tests {
	use super::*;

	#[derive(Decode, Encode, Debug, Clone)]
	pub struct RcPrePayload {
		pub parameters: Option<pallet_society::GroupParams<u128>>,
		pub pot: u128,
		pub founder: Option<AccountId32>,
		pub head: Option<AccountId32>,
		pub rules: Option<H256>,
		pub member_count: u32,
		pub round_count: u32,
		pub bids: Vec<pallet_society::Bid<AccountId32, u128>>,
		pub skeptic: Option<AccountId32>,
		pub next_head: Option<pallet_society::IntakeRecord<AccountId32, u128>>,
		pub challenge_round_count: u32,
		pub defending: Option<(AccountId32, AccountId32, pallet_society::Tally)>,
		pub members: Vec<(AccountId32, pallet_society::MemberRecord)>,
		#[allow(clippy::type_complexity)]
		pub payouts: Vec<(
			AccountId32,
			pallet_society::PayoutRecord<u128, BoundedVec<(u32, u128), ConstU32<MAX_PAYOUTS>>>,
		)>,
		pub member_by_index: Vec<(u32, AccountId32)>,
		pub suspended_members: Vec<(AccountId32, pallet_society::MemberRecord)>,
		pub candidates: Vec<(AccountId32, pallet_society::Candidacy<AccountId32, u128>)>,
		pub votes: Vec<(AccountId32, AccountId32, pallet_society::Vote)>,
		pub vote_clear_cursor: Vec<(AccountId32, Vec<u8>)>,
		pub defender_votes: Vec<(u32, AccountId32, pallet_society::Vote)>,
		pub next_intake_at: Option<u32>,
		pub next_challenge_at: Option<u32>,
	}

	pub struct SocietyMigratorTest<T>(PhantomData<T>);
	impl<T: Config> crate::types::RcMigrationCheck for SocietyMigratorTest<T> {
		type RcPrePayload = RcPrePayload;

		fn pre_check() -> Self::RcPrePayload {
			use pallet_society::*;

			let parameters = Parameters::<T::KusamaConfig>::get();
			let pot = Pot::<T::KusamaConfig>::get();
			let founder = Founder::<T::KusamaConfig>::get();
			let head = Head::<T::KusamaConfig>::get();
			let rules = Rules::<T::KusamaConfig>::get();
			let member_count = MemberCount::<T::KusamaConfig>::get();
			let round_count = RoundCount::<T::KusamaConfig>::get();
			let bids = Bids::<T::KusamaConfig>::get().into_inner();
			let skeptic = Skeptic::<T::KusamaConfig>::get();
			let next_head = NextHead::<T::KusamaConfig>::get();
			let challenge_round_count = ChallengeRoundCount::<T::KusamaConfig>::get();
			let defending = Defending::<T::KusamaConfig>::get();
			let members: Vec<(AccountId32, pallet_society::MemberRecord)> =
				Members::<T::KusamaConfig>::iter().collect();
			#[allow(clippy::type_complexity)]
			let payouts: Vec<(
				AccountId32,
				pallet_society::PayoutRecord<u128, BoundedVec<(u32, u128), ConstU32<MAX_PAYOUTS>>>,
			)> = Payouts::<T::KusamaConfig>::iter().collect();
			let member_by_index: Vec<(u32, AccountId32)> =
				MemberByIndex::<T::KusamaConfig>::iter().collect();
			let suspended_members: Vec<(AccountId32, pallet_society::MemberRecord)> =
				SuspendedMembers::<T::KusamaConfig>::iter().collect();
			let candidates: Vec<(AccountId32, pallet_society::Candidacy<AccountId32, u128>)> =
				Candidates::<T::KusamaConfig>::iter().collect();
			let votes: Vec<(AccountId32, AccountId32, pallet_society::Vote)> =
				Votes::<T::KusamaConfig>::iter().collect();
			let vote_clear_cursor: Vec<(AccountId32, Vec<u8>)> =
				VoteClearCursor::<T::KusamaConfig>::iter()
					.map(|(key, value)| (key, value.into_inner()))
					.collect();
			let defender_votes: Vec<(u32, AccountId32, pallet_society::Vote)> =
				DefenderVotes::<T::KusamaConfig>::iter().collect();

			let next_intake_at =
				if let Some(next_intake_at) = NextIntakeAt::<T::KusamaConfig>::get() {
					let rotation_period =
						<T::KusamaConfig as pallet_society::Config>::VotingPeriod::get()
							.saturating_add(
								<T::KusamaConfig as pallet_society::Config>::ClaimPeriod::get(),
							);
					if next_intake_at != rotation_period {
						Some(next_intake_at)
					} else {
						None
					}
				} else {
					None
				};
			let next_challenge_at =
				if let Some(next_challenge_at) = NextChallengeAt::<T::KusamaConfig>::get() {
					let challenge_period =
						<T::KusamaConfig as pallet_society::Config>::ChallengePeriod::get();
					if next_challenge_at != challenge_period {
						Some(next_challenge_at)
					} else {
						None
					}
				} else {
					None
				};

			RcPrePayload {
				parameters,
				pot,
				founder,
				head,
				rules,
				member_count,
				round_count,
				bids,
				skeptic,
				next_head,
				challenge_round_count,
				defending,
				members,
				payouts,
				member_by_index,
				suspended_members,
				candidates,
				votes,
				vote_clear_cursor,
				defender_votes,
				next_intake_at,
				next_challenge_at,
			}
		}

		fn post_check(_: Self::RcPrePayload) {
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

			//if let Some(next_challenge_at) = NextChallengeAt::<T::KusamaConfig>::get() {
			//	let challenge_period =
			//		<T::KusamaConfig as pallet_society::Config>::ChallengePeriod::get();
			//	assert_eq!(
			//		next_challenge_at, challenge_period,
			//		"`next_challenge_at` must be equal to the `ChallengePeriod` if not `None`",
			//	);
			//};

			// if let Some(next_intake_at) = NextIntakeAt::<T::KusamaConfig>::get() {
			// 	let rotation_period =
			// 		<T::KusamaConfig as pallet_society::Config>::VotingPeriod::get()
			// 			.saturating_add(
			// 				<T::KusamaConfig as pallet_society::Config>::ClaimPeriod::get(),
			// 			);
			// 	assert_eq!(
			// 		next_intake_at, rotation_period,
			// 		"`next_intake_at` must be equal to the rotation period if not `None`",
			// 	);
			// };
		}
	}
}
