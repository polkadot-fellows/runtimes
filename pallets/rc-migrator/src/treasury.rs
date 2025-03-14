// Copyright (C) Parity Technologies and the various Polkadot contributors, see Contributions.md
// for a list of specific contributors.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::*;
use pallet_treasury::{Proposal, ProposalIndex, SpendIndex};

/// Stage of the scheduler pallet migration.
#[derive(Encode, Decode, Clone, Default, RuntimeDebug, TypeInfo, MaxEncodedLen, PartialEq, Eq)]
pub enum TreasuryStage {
	#[default]
	ProposalCount,
	Proposals(Option<ProposalIndex>),
	Deactivated,
	Approvals,
	SpendCount,
	Spends(Option<SpendIndex>),
	// TODO: migrate with new sdk version
	// LastSpendPeriod,
	Funds,
	Finished,
}

/// Message that is being sent to the AH Migrator.
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub enum RcTreasuryMessage<
	AccountId,
	Balance,
	AssetBalance,
	BlockNumber,
	AssetKind,
	Beneficiary,
	PaymentId,
> {
	ProposalCount(ProposalIndex),
	Proposals((ProposalIndex, Proposal<AccountId, Balance>)),
	Deactivated(Balance),
	Approvals(Vec<ProposalIndex>),
	SpendCount(SpendIndex),
	Spends(
		(
			SpendIndex,
			alias::SpendStatus<AssetKind, AssetBalance, Beneficiary, BlockNumber, PaymentId>,
		),
	),
	// TODO: migrate with new sdk version
	// LastSpendPeriod(BlockNumber),
	Funds,
}

pub type RcTreasuryMessageOf<T> = RcTreasuryMessage<
	<T as frame_system::Config>::AccountId,
	pallet_treasury::BalanceOf<T, ()>,
	pallet_treasury::AssetBalanceOf<T, ()>,
	BlockNumberFor<T>,
	<T as pallet_treasury::Config>::AssetKind,
	<T as pallet_treasury::Config>::Beneficiary,
	<<T as pallet_treasury::Config>::Paymaster as Pay>::Id,
>;

pub struct TreasuryMigrator<T: Config> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for TreasuryMigrator<T> {
	type Key = TreasuryStage;
	type Error = Error<T>;

	fn migrate_many(
		last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut last_key = last_key.unwrap_or(TreasuryStage::ProposalCount);
		let mut messages = Vec::new();

		loop {
			if weight_counter
				.try_consume(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))
				.is_err()
			{
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
			}
			if messages.len() > 10_000 {
				log::warn!(target: LOG_TARGET, "Weight allowed very big batch, stopping");
				break;
			}

			last_key = match last_key {
				TreasuryStage::ProposalCount => {
					let count = pallet_treasury::ProposalCount::<T>::take();
					messages.push(RcTreasuryMessage::ProposalCount(count));
					TreasuryStage::Proposals(None)
				},
				TreasuryStage::Proposals(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_treasury::Proposals::<T>::iter_from_key(last_key)
					} else {
						pallet_treasury::Proposals::<T>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							pallet_treasury::Proposals::<T>::remove(&key);
							messages.push(RcTreasuryMessage::Proposals((key, value)));
							TreasuryStage::Proposals(Some(key))
						},
						None => TreasuryStage::Deactivated,
					}
				},
				TreasuryStage::Deactivated => {
					let deactivated = pallet_treasury::Deactivated::<T>::take();
					messages.push(RcTreasuryMessage::Deactivated(deactivated));
					TreasuryStage::Approvals
				},
				TreasuryStage::Approvals => {
					let approvals = pallet_treasury::Approvals::<T>::take();
					messages.push(RcTreasuryMessage::Approvals(approvals.into_inner()));
					TreasuryStage::SpendCount
				},
				TreasuryStage::SpendCount => {
					let count = alias::SpendCount::<T>::take();
					messages.push(RcTreasuryMessage::SpendCount(count));
					TreasuryStage::Spends(None)
				},
				TreasuryStage::Spends(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						alias::Spends::<T>::iter_from_key(last_key)
					} else {
						alias::Spends::<T>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							alias::Spends::<T>::remove(&key);
							messages.push(RcTreasuryMessage::Spends((key, value)));
							TreasuryStage::Spends(Some(key))
						},
						// TODO: TreasuryStage::LastSpendPeriod
						None => TreasuryStage::Funds,
					}
				},
				// TODO: with new sdk version
				// TreasuryStage::LastSpendPeriod => {
				//     let last_spend_period = pallet_treasury::LastSpendPeriod::<T>::take();
				// 	messages.push(RcTreasuryMessage::LastSpendPeriod(last_spend_period));
				// 	TreasuryStage::Funds
				// },
				TreasuryStage::Funds => {
					messages.push(RcTreasuryMessage::Funds);
					TreasuryStage::Finished
				},
				TreasuryStage::Finished => {
					break;
				},
			};
		}

		if messages.len() > 0 {
			Pallet::<T>::send_chunked_xcm(
				messages,
				|messages| types::AhMigratorCall::<T>::ReceiveTreasuryMessages { messages },
				|_| Weight::from_all(1), // TODO
			)?;
		}

		if last_key == TreasuryStage::Finished {
			Ok(None)
		} else {
			Ok(Some(last_key))
		}
	}
}

pub mod alias {
	use super::*;

	/// Alias for private item [`pallet_treasury::SpendCount`].
	#[frame_support::storage_alias(pallet_name)]
	pub type SpendCount<T: pallet_treasury::Config> =
		StorageValue<pallet_treasury::Pallet<T>, SpendIndex, ValueQuery>;

	/// Spends that have been approved and being processed.
	///
	/// Copy of [`pallet_treasury::Spends`].
	#[frame_support::storage_alias(pallet_name)]
	pub type Spends<T: pallet_treasury::Config> = StorageMap<
		pallet_treasury::Pallet<T>,
		Twox64Concat,
		pallet_treasury::SpendIndex,
		SpendStatus<
			<T as pallet_treasury::Config>::AssetKind,
			pallet_treasury::AssetBalanceOf<T, ()>,
			<T as pallet_treasury::Config>::Beneficiary,
			BlockNumberFor<T>,
			<<T as pallet_treasury::Config>::Paymaster as Pay>::Id,
		>,
		OptionQuery,
	>;

	/// Info regarding an approved treasury spend.
	///
	/// Copy of [`pallet_treasury::SpendStatus`].
	#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
	#[derive(Encode, Decode, Clone, PartialEq, Eq, MaxEncodedLen, Debug, TypeInfo)]
	pub struct SpendStatus<AssetKind, AssetBalance, Beneficiary, BlockNumber, PaymentId> {
		// The kind of asset to be spent.
		pub asset_kind: AssetKind,
		/// The asset amount of the spend.
		pub amount: AssetBalance,
		/// The beneficiary of the spend.
		pub beneficiary: Beneficiary,
		/// The block number from which the spend can be claimed.
		pub valid_from: BlockNumber,
		/// The block number by which the spend has to be claimed.
		pub expire_at: BlockNumber,
		/// The status of the payout/claim.
		pub status: pallet_treasury::PaymentState<PaymentId>,
	}
}
