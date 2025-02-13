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

// TODO FAIL-CI: Insecure unless your chain includes `PrevalidateAttests` as a
// `TransactionExtension`.

use crate::{types::AccountIdOf, *};

pub struct CrowdloanMigrator<T> {
	_marker: sp_std::marker::PhantomData<T>,
}

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug, Clone, PartialEq, Eq)]
pub enum RcCrowdloanMessage<BlockNumber, AccountId, Balance> {
	LeaseReserve {
		/// The block number at which this deposit can be unreserved.
		unreserve_block: BlockNumber,
		account: AccountId,
		para_id: ParaId,
		amount: Balance,
	},
	CrowdloanContribution {
		/// The block number at which this contribution can be withdrawn.
		withdraw_block: BlockNumber,
		contributor: AccountId,
		para_id: ParaId,
		amount: Balance,
		crowdloan_account: AccountId,
	},
	CrowdloanDeposit {
		unreserve_block: BlockNumber,
		depositor: AccountId,
		para_id: ParaId,
		fund_index: u32,
		amount: Balance,
	},
}

pub type RcCrowdloanMessageOf<T> =
	RcCrowdloanMessage<BlockNumberFor<T>, AccountIdOf<T>, crate::BalanceOf<T>>;

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug, Clone, PartialEq, Eq)]
pub enum CrowdloanStage {
	LeaseReserve { last_key: Option<ParaId> },
	CrowdloanContribution { last_key: Option<ParaId> },
	CrowdloanDeposit,
	Finished,
}

impl<T: Config> PalletMigration for CrowdloanMigrator<T>
	where
	crate::BalanceOf<T>:
		From<<<T as polkadot_runtime_common::slots::Config>::Currency as frame_support::traits::Currency<sp_runtime::AccountId32>>::Balance>,
	crate::BalanceOf<T>:
		From<<<<T as polkadot_runtime_common::crowdloan::Config>::Auctioneer as polkadot_runtime_common::traits::Auctioneer<<<<T as frame_system::Config>::Block as sp_runtime::traits::Block>::Header as sp_runtime::traits::Header>::Number>>::Currency as frame_support::traits::Currency<sp_runtime::AccountId32>>::Balance>,
{
	type Key = CrowdloanStage;
	type Error = Error<T>;

	fn migrate_many(
		current_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut inner_key = current_key.unwrap_or(CrowdloanStage::LeaseReserve { last_key: None });
		let mut messages = Vec::new();

		loop {
			if weight_counter
				.try_consume(<T as frame_system::Config>::DbWeight::get().reads_writes(1, 1))
				.is_err()
			{
				/*if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}*/
				log::warn!("Out of weight, stop. num messages: {}", messages.len());
				break;
			}

			if messages.len() > 10_000 {
				log::warn!("Weight allowed very big batch, stopping");
				break;
			}

			inner_key = match inner_key {
				CrowdloanStage::LeaseReserve { last_key } => {
					let mut iter = match last_key.clone() {
						Some(last_key) => pallet_slots::Leases::<T>::iter_from_key(last_key),
						None => pallet_slots::Leases::<T>::iter(),
					};

					match iter.next() {
						Some((para_id, leases)) => {
							let Some(last_lease) = leases.last() else {
								// This seems to be fine, but i don't know how it happens, see https://github.com/paritytech/polkadot-sdk/blob/db3ff60b5af2a9017cb968a4727835f3d00340f0/polkadot/runtime/common/src/slots/mod.rs#L108-L109
								log::warn!(target: LOG_TARGET, "Empty leases for para_id: {:?}", para_id);
								inner_key = CrowdloanStage::LeaseReserve { last_key: Some(para_id) };
								continue;
							};

							let Some((lease_acc, _)) = last_lease else {
								defensive!("Last lease cannot be None");
								inner_key = CrowdloanStage::LeaseReserve { last_key: Some(para_id) };
								continue;
							};

							// NOTE: Max instead of sum, see https://github.com/paritytech/polkadot-sdk/blob/db3ff60b5af2a9017cb968a4727835f3d00340f0/polkadot/runtime/common/src/slots/mod.rs#L102-L103
							let amount: crate::BalanceOf<T> = leases.iter().flatten().map(|(_acc, amount)| amount).max().cloned().unwrap_or_default().into();

							if amount == 0u32.into() {
								defensive_assert!(para_id < ParaId::from(2000), "Must be system chain");
								inner_key = CrowdloanStage::LeaseReserve { last_key: Some(para_id) };
								continue;
							}

							let unreserve_block = num_leases_to_ending_block::<T>(leases.len() as u32);

							log::warn!(target: LOG_TARGET, "Migrating out lease reserve for para_id: {:?}, account: {:?}, amount: {:?}, unreserve_block: {:?}", &para_id, &lease_acc, &amount, &unreserve_block);
							messages.push(RcCrowdloanMessage::LeaseReserve { unreserve_block, account: lease_acc.clone(), para_id, amount });
							CrowdloanStage::LeaseReserve { last_key: Some(para_id) }
						},
						None => CrowdloanStage::CrowdloanContribution { last_key: None },
					}
				},
				CrowdloanStage::CrowdloanContribution { last_key } => {
					let mut funds_iter = match last_key.clone() {
						Some(last_key) => pallet_crowdloan::Funds::<T>::iter_from_key(last_key),
						None => pallet_crowdloan::Funds::<T>::iter(),
					};

					let (para_id, fund) = match funds_iter.next() {
						Some((para_id, fund)) => (para_id, fund),
						None => {
							inner_key = CrowdloanStage::CrowdloanDeposit;
							continue;
						},
					};

					let mut contributions_iter = pallet_crowdloan::Pallet::<T>::contribution_iterator(fund.fund_index);

					match contributions_iter.next() {
						Some((contributor, (amount, memo))) => {
							// Dont really care about memos, but we can add them, if needed.
							if !memo.is_empty() {
								log::warn!(target: LOG_TARGET, "Discarding crowdloan memo of length: {}", &memo.len());
							}

							let leases = pallet_slots::Leases::<T>::get(para_id);
							if leases.is_empty() {
								defensive!("Leases should not be empty if there is a fund");
							}

							let crowdloan_account = pallet_crowdloan::Pallet::<T>::fund_account_id(fund.fund_index);

							let withdraw_block = num_leases_to_ending_block::<T>(leases.len() as u32);
							log::warn!(target: LOG_TARGET, "Migrating out crowdloan contribution for para_id: {:?}, contributor: {:?}, amount: {:?}, withdraw_block: {:?}", &para_id, &contributor, &amount, &withdraw_block);
							pallet_crowdloan::Pallet::<T>::contribution_kill(fund.fund_index, &contributor);
							messages.push(RcCrowdloanMessage::CrowdloanContribution { withdraw_block, contributor, para_id, amount: amount.into(), crowdloan_account });

							inner_key // does not change since we deleted the contribution
						},
						None => {
							CrowdloanStage::CrowdloanContribution { last_key: Some(para_id) }
						},
					}
				},
				CrowdloanStage::CrowdloanDeposit => {
					match pallet_crowdloan::Funds::<T>::iter().next() {
						Some((para_id, fund)) => {
							pallet_crowdloan::Funds::<T>::take(para_id);

							let leases = pallet_slots::Leases::<T>::get(para_id);
							if leases.is_empty() {
								defensive!("Leases should not be empty if there is a fund");
							}
							let unreserve_block = num_leases_to_ending_block::<T>(leases.len() as u32);

							log::warn!(target: LOG_TARGET, "Migrating out crowdloan deposit for para_id: {:?}, fund_index: {:?}, amount: {:?}, depositor: {:?}", &para_id, &fund.fund_index, &fund.deposit, &fund.depositor);
							messages.push(RcCrowdloanMessage::CrowdloanDeposit { unreserve_block, para_id, fund_index: fund.fund_index, amount: fund.deposit.into(), depositor: fund.depositor });
							CrowdloanStage::CrowdloanDeposit
						},
						None => CrowdloanStage::Finished,
					}
				},
				CrowdloanStage::Finished => {
					break;
				},
			}
		}

		if !messages.is_empty() {
			Pallet::<T>::send_chunked_xcm(messages, |messages| {
				types::AhMigratorCall::<T>::ReceiveCrowdloanMessages { messages }
			})?;
		}

		if inner_key == CrowdloanStage::Finished {
			Ok(None)
		} else {
			Ok(Some(inner_key))
		}
	}
}

/// Calculate the lease ending block from the number of remaining leases (including the current).
fn num_leases_to_ending_block<T: Config>(num_leases: u32) -> BlockNumberFor<T> {
	let now = frame_system::Pallet::<T>::block_number();
	let num_leases: BlockNumberFor<T> = num_leases.into();
	let offset = <T as pallet_slots::Config>::LeaseOffset::get();
	let period = <T as pallet_slots::Config>::LeasePeriod::get();

	let current_period = (now - offset) / period;
	(current_period + num_leases) * period + offset
}
