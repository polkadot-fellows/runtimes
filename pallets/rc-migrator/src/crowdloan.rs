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
		block: BlockNumber,
		account: AccountId,
		para_id: ParaId,
		amount: Balance,
	},
	CrowdloanContribution {
		block: BlockNumber,
		contributor: AccountId,
		para_id: ParaId,
		amount: Balance,
		crowdloan_account: AccountId,
	},
}

pub type RcCrowdloanMessageOf<T> =
	RcCrowdloanMessage<BlockNumberFor<T>, AccountIdOf<T>, crate::BalanceOf<T>>;

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, RuntimeDebug, Clone, PartialEq, Eq)]
pub enum CrowdloanStage {
	LeaseReserve { last_key: Option<ParaId> },
	CrowdloanContribution { last_key: Option<ParaId> },
	Finished,
}

impl<T: Config> PalletMigration for CrowdloanMigrator<T>
where crate::BalanceOf<T>: From<<<T as polkadot_runtime_common::slots::Config>::Currency as frame_support::traits::Currency<sp_runtime::AccountId32>>::Balance>  {
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
				if messages.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break;
				}
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
							pallet_slots::Leases::<T>::remove(&para_id);

							let Some(last_lease) = leases.last() else {
								// This seems to be fine, but i don't know how it happens, see https://github.com/paritytech/polkadot-sdk/blob/db3ff60b5af2a9017cb968a4727835f3d00340f0/polkadot/runtime/common/src/slots/mod.rs#L108-L109
								log::warn!(target: LOG_TARGET, "Empty leases for para_id: {:?}", para_id);
								continue;
							};

							let Some((lease_acc, _)) = last_lease else {
								defensive!("Last lease cannot be None");
								continue;
							};

							// NOTE: Max instead of sum, see https://github.com/paritytech/polkadot-sdk/blob/db3ff60b5af2a9017cb968a4727835f3d00340f0/polkadot/runtime/common/src/slots/mod.rs#L102-L103
							let amount = leases.iter().flatten().map(|(_acc, amount)| amount).max().cloned().unwrap_or_default().into();

							if amount == 0 {
								// fucking stupid ParaId type
								defensive_assert!(para_id < ParaId::from(2000), "Must be system chain");
								continue;
							}

							let unlock_block = num_leases_to_ending_block::<T>(leases.len() as u32);

							log::warn!(target: LOG_TARGET, "Migrating out lease reserve for para_id: {:?}, account: {:?}, amount: {:?}, unlock_block: {:?}", &para_id, &lease_acc, &amount, &unlock_block);
							messages.push(RcCrowdloanMessage::LeaseReserve { block: unlock_block, account: lease_acc.clone(), para_id, amount });						
							CrowdloanStage::LeaseReserve { last_key: Some(para_id) }
						},
						None => CrowdloanStage::CrowdloanContribution { last_key: None },
					}
				},
				CrowdloanStage::CrowdloanContribution { last_key } => {
					todo!()
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
