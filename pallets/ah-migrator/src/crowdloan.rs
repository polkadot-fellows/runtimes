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
use frame_support::traits::tokens::Preservation;
use polkadot_runtime_common::crowdloan as pallet_crowdloan;

impl<T: Config> Pallet<T> {
	pub fn do_receive_crowdloan_messages(
		messages: Vec<RcCrowdloanMessageOf<T>>,
	) -> Result<(), Error<T>> {
		todo!()
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
			.ok_or(Error::<T>::NoLeaseDeposit)?;

		let remaining = <T as Config>::Currency::unreserve(&depositor, balance);
		if remaining > 0 {
			defensive!("Should be able to unreserve all");
			Self::deposit_event(Event::LeaseDepositUnreserveRemaining {
				depositor,
				remaining,
				para_id,
			});
		}

		Ok(())
	}

	pub fn do_withdraw_crowdloan_contribution(
		block: BlockNumberFor<T>,
		depositor: T::AccountId,
		para_id: ParaId,
	) -> Result<(), Error<T>> {
		// TODO remember to reactivate balance
		let (pot, contribution) = RcCrowdloanContribution::<T>::take((block, &depositor, para_id))
			.ok_or(Error::<T>::NoCrowdloanContribution)?;

		// Maybe this is the first one to withdraw and we need to unreserve it from the pot
		match Self::do_unreserve_lease_deposit(block, pot.clone(), para_id) {
			Ok(()) => (),
			Err(Error::<T>::NoLeaseDeposit) => (), // fine
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
}
