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

#![allow(clippy::from_over_into)] // We just want Into<s>, no From<s> coercion needed.

use crate::*;
use pallet_treasury::{Proposal, ProposalIndex, SpendIndex};
use polkadot_runtime_common::impls::VersionedLocatableAsset;

/// Stage of the scheduler pallet migration.
#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	Clone,
	Default,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	PartialEq,
	Eq,
)]
pub enum TreasuryStage {
	#[default]
	ProposalCount,
	Proposals(Option<ProposalIndex>),
	// should not be migrated since automatically updated `on_initialize`.
	// Deactivated,
	Approvals,
	SpendCount,
	Spends(Option<SpendIndex>),
	LastSpendPeriod,
	Funds,
	Finished,
}

/// Message that is being sent to the AH Migrator.
#[derive(Encode, DecodeWithMemTracking, Decode, Debug, Clone, TypeInfo, PartialEq, Eq)]
pub enum PortableTreasuryMessage {
	ProposalCount(ProposalIndex),
	Proposals((ProposalIndex, Proposal<AccountId32, u128>)),
	Approvals(Vec<ProposalIndex>),
	SpendCount(SpendIndex),
	Spends { id: SpendIndex, status: PortableSpendStatus },
	LastSpendPeriod(Option<u32>),
	Funds,
}

pub struct TreasuryMigrator<T> {
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
				.any_lt(T::AhWeightInfo::receive_treasury_messages(messages.len() + 1))
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
				TreasuryStage::ProposalCount => {
					if pallet_treasury::ProposalCount::<T>::exists() {
						let count = pallet_treasury::ProposalCount::<T>::take();
						messages.push(PortableTreasuryMessage::ProposalCount(count));
					}
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
							pallet_treasury::Proposals::<T>::remove(key);
							messages.push(PortableTreasuryMessage::Proposals((key, value)));
							TreasuryStage::Proposals(Some(key))
						},
						None => TreasuryStage::Approvals,
					}
				},
				TreasuryStage::Approvals => {
					if pallet_treasury::Approvals::<T>::exists() {
						let approvals = pallet_treasury::Approvals::<T>::take();
						messages.push(PortableTreasuryMessage::Approvals(approvals.into_inner()));
					}
					TreasuryStage::SpendCount
				},
				TreasuryStage::SpendCount => {
					if pallet_treasury::SpendCount::<T>::exists() {
						let count = pallet_treasury::SpendCount::<T>::take();
						messages.push(PortableTreasuryMessage::SpendCount(count));
					}
					TreasuryStage::Spends(None)
				},
				TreasuryStage::Spends(last_key) => {
					let mut iter = if let Some(last_key) = last_key {
						pallet_treasury::Spends::<T>::iter_from_key(last_key)
					} else {
						pallet_treasury::Spends::<T>::iter()
					};
					match iter.next() {
						Some((key, value)) => {
							pallet_treasury::Spends::<T>::remove(key);
							messages.push(PortableTreasuryMessage::Spends {
								id: key,
								status: value.into_portable(),
							});
							TreasuryStage::Spends(Some(key))
						},
						None => TreasuryStage::LastSpendPeriod,
					}
				},
				TreasuryStage::LastSpendPeriod => {
					if pallet_treasury::LastSpendPeriod::<T>::exists() {
						let last_spend_period = pallet_treasury::LastSpendPeriod::<T>::take();
						messages.push(PortableTreasuryMessage::LastSpendPeriod(last_spend_period));
					}
					TreasuryStage::Funds
				},
				TreasuryStage::Funds => {
					messages.push(PortableTreasuryMessage::Funds);
					TreasuryStage::Finished
				},
				TreasuryStage::Finished => {
					break;
				},
			};
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(messages, |messages| {
				types::AhMigratorCall::<T>::ReceiveTreasuryMessages { messages }
			})?;
		}

		if last_key == TreasuryStage::Finished {
			Ok(None)
		} else {
			Ok(Some(last_key))
		}
	}
}

#[derive(
	Clone, PartialEq, Eq, Debug, Encode, DecodeWithMemTracking, Decode, TypeInfo, MaxEncodedLen,
)]
pub struct PortableSpendStatus {
	pub asset_kind: VersionedLocatableAsset,
	pub amount: u128,
	pub beneficiary: VersionedLocation,
	pub valid_from: u32,
	pub expire_at: u32,
	pub status: PortablePaymentState,
}

// RC -> Portable
impl IntoPortable
	for pallet_treasury::SpendStatus<VersionedLocatableAsset, u128, VersionedLocation, u32, u64>
{
	type Portable = PortableSpendStatus;

	fn into_portable(self) -> Self::Portable {
		PortableSpendStatus {
			asset_kind: self.asset_kind,
			amount: self.amount,
			beneficiary: self.beneficiary,
			valid_from: self.valid_from,
			expire_at: self.expire_at,
			status: self.status.into_portable(),
		}
	}
}

// Portable -> AH
impl Into<pallet_treasury::SpendStatus<VersionedLocatableAsset, u128, VersionedLocation, u32, u64>>
	for PortableSpendStatus
{
	fn into(
		self,
	) -> pallet_treasury::SpendStatus<VersionedLocatableAsset, u128, VersionedLocation, u32, u64> {
		pallet_treasury::SpendStatus {
			asset_kind: self.asset_kind,
			amount: self.amount,
			beneficiary: self.beneficiary,
			valid_from: self.valid_from,
			expire_at: self.expire_at,
			status: self.status.into(),
		}
	}
}

#[derive(
	Clone, PartialEq, Eq, Debug, Encode, DecodeWithMemTracking, Decode, TypeInfo, MaxEncodedLen,
)]
pub enum PortablePaymentState {
	Pending,
	Attempted { id: u64 },
	Failed,
}

// RC -> Portable
impl IntoPortable for pallet_treasury::PaymentState<u64> {
	type Portable = PortablePaymentState;

	fn into_portable(self) -> Self::Portable {
		match self {
			pallet_treasury::PaymentState::Pending => PortablePaymentState::Pending,
			pallet_treasury::PaymentState::Attempted { id } =>
				PortablePaymentState::Attempted { id },
			pallet_treasury::PaymentState::Failed => PortablePaymentState::Failed,
		}
	}
}

// Portable -> AH
impl Into<pallet_treasury::PaymentState<u64>> for PortablePaymentState {
	fn into(self) -> pallet_treasury::PaymentState<u64> {
		match self {
			PortablePaymentState::Pending => pallet_treasury::PaymentState::Pending,
			PortablePaymentState::Attempted { id } =>
				pallet_treasury::PaymentState::Attempted { id },
			PortablePaymentState::Failed => pallet_treasury::PaymentState::Failed,
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::RcMigrationCheck for TreasuryMigrator<T> {
	// (proposals with data, historical proposals count, approvals ids, spends, historical spends
	// count)
	type RcPrePayload = (
		Vec<(ProposalIndex, Proposal<AccountId32, u128>)>,
		u32,
		Vec<ProposalIndex>,
		Vec<(SpendIndex, PortableSpendStatus)>,
		u32,
	);

	fn pre_check() -> Self::RcPrePayload {
		// Store the counts and approvals before migration
		let proposals = pallet_treasury::Proposals::<T>::iter().collect::<Vec<_>>();
		let proposals_count = pallet_treasury::ProposalCount::<T>::get();
		let approvals = pallet_treasury::Approvals::<T>::get().into_inner();
		let spends = pallet_treasury::Spends::<T>::iter()
			.map(|(spend_id, spend_status)| {
				(
					spend_id,
					PortableSpendStatus {
						asset_kind: spend_status.asset_kind,
						amount: spend_status.amount,
						beneficiary: spend_status.beneficiary,
						valid_from: spend_status.valid_from,
						expire_at: spend_status.expire_at,
						status: spend_status.status.into_portable(),
					},
				)
			})
			.collect::<Vec<_>>();
		let spends_count = pallet_treasury::SpendCount::<T>::get();
		(proposals, proposals_count, approvals, spends, spends_count)
	}

	fn post_check(_rc_payload: Self::RcPrePayload) {
		// Assert storage 'Treasury::ProposalCount::rc_post::empty'
		assert_eq!(
			pallet_treasury::ProposalCount::<T>::get(),
			0,
			"ProposalCount should be 0 on relay chain after migration"
		);

		// Assert storage 'Treasury::Approvals::rc_post::empty'
		assert!(
			pallet_treasury::Approvals::<T>::get().is_empty(),
			"Approvals should be empty on relay chain after migration"
		);

		// Assert storage 'Treasury::Proposals::rc_post::empty'
		assert!(
			pallet_treasury::Proposals::<T>::iter().next().is_none(),
			"Proposals should be empty on relay chain after migration"
		);

		// Assert storage 'Treasury::SpendCount::rc_post::empty'
		assert_eq!(
			pallet_treasury::SpendCount::<T>::get(),
			0,
			"SpendCount should be 0 on relay chain after migration"
		);

		// Assert storage 'Treasury::Spends::rc_post::empty'
		assert!(
			pallet_treasury::Spends::<T>::iter().next().is_none(),
			"Spends should be empty on relay chain after migration"
		);
	}
}
