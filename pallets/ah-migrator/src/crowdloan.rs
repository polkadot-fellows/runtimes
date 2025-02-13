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

use crate::*;
use frame_support::traits::tokens::Preservation;
use pallet_rc_migrator::crowdloan::RcCrowdloanMessage;

impl<T: Config> Pallet<T> {
	pub fn do_receive_crowdloan_messages(
		messages: Vec<RcCrowdloanMessageOf<T>>,
	) -> Result<(), Error<T>> {
		let (mut good, mut bad) = (0, 0);
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Crowdloan,
			count: messages.len() as u32,
		});
		log::info!(target: LOG_TARGET, "Received {} crowdloan messages", messages.len());

		for message in messages {
			match Self::do_process_crowdloan_message(message) {
				Ok(()) => good += 1,
				Err(e) => {
					bad += 1;
					log::error!(target: LOG_TARGET, "Error while integrating crowdloan message: {:?}", e);
				},
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Crowdloan,
			count_good: good,
			count_bad: bad,
		});

		Ok(())
	}

	pub fn do_process_crowdloan_message(message: RcCrowdloanMessageOf<T>) -> Result<(), Error<T>> {
		match message {
			RcCrowdloanMessage::LeaseReserve { unreserve_block, account, para_id, amount } => {
				log::info!(target: LOG_TARGET, "Integrating lease reserve for para_id: {:?}, account: {:?}, amount: {:?}, unreserve_block: {:?}", &para_id, &account, &amount, &unreserve_block);
				defensive_assert!(!RcLeaseReserve::<T>::contains_key((
					unreserve_block,
					&account,
					para_id
				)));

				RcLeaseReserve::<T>::insert((unreserve_block, account, para_id), amount);
			},
			RcCrowdloanMessage::CrowdloanContribution {
				withdraw_block,
				contributor,
				para_id,
				amount,
				crowdloan_account,
			} => {
				log::info!(target: LOG_TARGET, "Integrating crowdloan contribution for para_id: {:?}, contributor: {:?}, amount: {:?}, crowdloan_account: {:?}, withdraw_block: {:?}", &para_id, &contributor, &amount, &crowdloan_account, &withdraw_block);
				defensive_assert!(!RcCrowdloanContribution::<T>::contains_key((
					withdraw_block,
					&contributor,
					para_id
				)));

				RcCrowdloanContribution::<T>::insert(
					(withdraw_block, contributor, para_id),
					(crowdloan_account, amount),
				);
			},
			RcCrowdloanMessage::CrowdloanDeposit {
				unreserve_block,
				para_id,
				fund_index,
				amount,
				depositor,
			} => {
				log::info!(target: LOG_TARGET, "Integrating crowdloan deposit for para_id: {:?}, fund_index: {:?}, amount: {:?}, depositor: {:?}", &para_id, &fund_index, &amount, &depositor);
				defensive_assert!(!RcCrowdloanReserve::<T>::contains_key((
					unreserve_block,
					&depositor,
					para_id
				)));

				RcCrowdloanReserve::<T>::insert((unreserve_block, depositor, para_id), amount);
			},
		}

		Ok(())
	}
}

// extrinsic code
impl<T: Config> Pallet<T> {
	pub fn do_unreserve_lease_deposit(
		block: BlockNumberFor<T>,
		depositor: T::AccountId,
		para_id: ParaId,
	) -> Result<(), Error<T>> {
		let balance = RcLeaseReserve::<T>::take((block, &depositor, para_id))
			.ok_or(Error::<T>::NoLeaseReserve)?;

		let remaining = <T as Config>::Currency::unreserve(&depositor, balance);
		if remaining > 0 {
			defensive!("Should be able to unreserve all");
			Self::deposit_event(Event::LeaseUnreserveRemaining { depositor, remaining, para_id });
		}

		Ok(())
	}

	pub fn do_withdraw_crowdloan_contribution(
		block: BlockNumberFor<T>,
		depositor: T::AccountId,
		para_id: ParaId,
	) -> Result<(), Error<T>> {
		let (pot, contribution) = RcCrowdloanContribution::<T>::take((block, &depositor, para_id))
			.ok_or(Error::<T>::NoCrowdloanContribution)?;

		// Maybe this is the first one to withdraw and we need to unreserve it from the pot
		match Self::do_unreserve_lease_deposit(block, pot.clone(), para_id) {
			Ok(()) => (),
			Err(Error::<T>::NoLeaseReserve) => (), // fine
			Err(e) => return Err(e),
		}

		// Ideally this does not fail. But if it does, then we keep it for manual inspection.
		let transferred = <T as Config>::Currency::transfer(
			&pot,
			&depositor,
			contribution,
			Preservation::Preserve,
		)
		.defensive()
		.map_err(|_| Error::<T>::FailedToWithdrawCrowdloanContribution)?;
		defensive_assert!(transferred == contribution);
		// Need to reactivate since we deactivated it here https://github.com/paritytech/polkadot-sdk/blob/04847d515ef56da4d0801c9b89a4241dfa827b33/polkadot/runtime/common/src/crowdloan/mod.rs#L793
		<T as Config>::Currency::reactivate(transferred);

		Ok(())
	}

	pub fn do_unreserve_crowdloan_reserve(
		block: BlockNumberFor<T>,
		depositor: T::AccountId,
		para_id: ParaId,
	) -> Result<(), Error<T>> {
		let amount = RcCrowdloanReserve::<T>::take((block, &depositor, para_id))
			.ok_or(Error::<T>::NoCrowdloanReserve)?;

		let remaining = <T as Config>::Currency::unreserve(&depositor, amount);
		if remaining > 0 {
			defensive!("Should be able to unreserve all");
			Self::deposit_event(Event::CrowdloanUnreserveRemaining {
				depositor,
				remaining,
				para_id,
			});
		}

		Ok(())
	}
}
