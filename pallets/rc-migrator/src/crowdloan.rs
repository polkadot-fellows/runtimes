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
	Setup,
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
		let mut inner_key = current_key.unwrap_or(CrowdloanStage::Setup);
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
				CrowdloanStage::Setup => {
					inner_key = CrowdloanStage::LeaseReserve { last_key: None };

					// Only thing to do here is to re-the bifrost crowdloan: https://polkadot.subsquare.io/referenda/524
					let leases = pallet_slots::Leases::<T>::take(ParaId::from(2030));
					if leases.is_empty() {
						defensive!("Bifrost fund maybe already ended, remove this");
						continue;
					}

					// It would be better if we can re-map all contributions to the new para id, but
					// that requires to iterate them all so we go the other way around; change the
					// new leases to the old Bifrost Crowdloan.
					pallet_slots::Leases::<T>::insert(ParaId::from(3356), leases);
					log::info!(target: LOG_TARGET, "Migrated Bifrost Leases from crowdloan 2030 to 3356");

					inner_key
				},
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

							log::debug!(target: LOG_TARGET, "Migrating out lease reserve for para_id: {:?}, account: {:?}, amount: {:?}, unreserve_block: {:?}", &para_id, &lease_acc, &amount, &unreserve_block);
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
								defensive_assert!(fund.raised == 0u32.into(), "Fund should be empty");
								inner_key = CrowdloanStage::CrowdloanContribution { last_key: Some(para_id) };
								continue;
							}

							let crowdloan_account = pallet_crowdloan::Pallet::<T>::fund_account_id(fund.fund_index);

							let withdraw_block = num_leases_to_ending_block::<T>(leases.len() as u32);
							log::debug!(target: LOG_TARGET, "Migrating out crowdloan contribution for para_id: {:?}, contributor: {:?}, amount: {:?}, withdraw_block: {:?}", &para_id, &contributor, &amount, &withdraw_block);
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
							inner_key = CrowdloanStage::CrowdloanDeposit;
							pallet_crowdloan::Funds::<T>::take(para_id);

							let leases = pallet_slots::Leases::<T>::get(para_id);
							if leases.is_empty() {
								defensive_assert!(fund.raised == 0u32.into(), "Fund should be empty");
								continue;
							}
							let unreserve_block = num_leases_to_ending_block::<T>(leases.len() as u32);

							log::debug!(target: LOG_TARGET, "Migrating out crowdloan deposit for para_id: {:?}, fund_index: {:?}, amount: {:?}, depositor: {:?}", &para_id, &fund.fund_index, &fund.deposit, &fund.depositor);
							messages.push(RcCrowdloanMessage::CrowdloanDeposit { unreserve_block, para_id, fund_index: fund.fund_index, amount: fund.deposit.into(), depositor: fund.depositor });
							inner_key
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
