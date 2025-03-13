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

impl<T: Config> Pallet<T> {
	pub fn do_receive_treasury_messages(messages: Vec<RcTreasuryMessageOf<T>>) -> DispatchResult {
		log::info!(target: LOG_TARGET, "Processing {} treasury messages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Treasury,
			count: messages.len() as u32,
		});
		let (mut count_good, mut count_bad) = (0, 0);

		for message in messages {
			match Self::do_process_treasury_message(message) {
				Ok(()) => count_good += 1,
				Err(_) => count_bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Treasury,
			count_good,
			count_bad,
		});
		log::info!(target: LOG_TARGET, "Processed {} treasury messages", count_good);

		Ok(())
	}

	fn do_process_treasury_message(message: RcTreasuryMessageOf<T>) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Processing treasury message: {:?}", message);

		match message {
			RcTreasuryMessage::ProposalCount(proposal_count) => {
				pallet_treasury::ProposalCount::<T>::put(proposal_count);
			},
			RcTreasuryMessage::Proposals((proposal_index, proposal)) => {
				pallet_treasury::Proposals::<T>::insert(proposal_index, proposal);
			},
			RcTreasuryMessage::Deactivated(deactivated) => {
				pallet_treasury::Deactivated::<T>::put(deactivated);
			},
			RcTreasuryMessage::Approvals(approvals) => {
				let approvals = BoundedVec::<_, <T as pallet_treasury::Config>::MaxApprovals>::defensive_truncate_from(approvals);
				pallet_treasury::Approvals::<T>::put(approvals);
			},
			RcTreasuryMessage::SpendCount(spend_count) => {
				pallet_rc_migrator::treasury::alias::SpendCount::<T>::put(spend_count);
			},
			RcTreasuryMessage::Spends((spend_index, spend)) => {
				pallet_treasury::Spends::<T>::insert(spend_index, spend);
			},
			// TODO: migrate with new sdk version
			// RcTreasuryMessage::LastSpendPeriod(last_spend_period) => {
			// 	pallet_treasury::LastSpendPeriod::<T>::put(last_spend_period);
			// },
			RcTreasuryMessage::Funds => {
				Self::migrate_treasury_funds();
			},
		}

		Ok(())
	}

	/// Migrate treasury funds.
	///
	/// Transfer all assets from old treasury account id on Asset Hub (account id derived from the
	/// treasury pallet location on RC from the perspective of AH) to new account id on Asset Hub
	/// (the treasury account id used on RC).
	fn migrate_treasury_funds() {
		let (old_account_id, assets) = T::TreasuryAccounts::get();
		let account_id = pallet_treasury::Pallet::<T>::account_id();

		// transfer all assets from old treasury account id to new account id
		for asset in assets {
			let reducible = T::Assets::reducible_balance(
				asset.clone(),
				&old_account_id,
				Preservation::Expendable,
				Fortitude::Polite,
			);

			match T::Assets::transfer(
				asset.clone(),
				&old_account_id,
				&account_id,
				reducible,
				Preservation::Expendable,
			) {
				Ok(_) => log::info!(
					target: LOG_TARGET,
					"Transferred treasury funds from old account {:?} to new account {:?} \
					for asset: {:?}, amount: {:?}",
					old_account_id,
					account_id,
					asset,
					reducible
				),
				Err(e) => {
					log::error!(
						target: LOG_TARGET,
						"Failed to transfer treasury funds from old account {:?} to new \
						account {:?} for asset: {:?}, amount: {:?}, error: {:?}",
						old_account_id,
						account_id,
						asset,
						reducible,
						e
					);
				},
			}
		}

		let reducible = <<T as Config>::Currency as Inspect<T::AccountId>>::reducible_balance(
			&old_account_id,
			Preservation::Expendable,
			Fortitude::Polite,
		);

		match <<T as Config>::Currency as Mutate<T::AccountId>>::transfer(
			&account_id,
			&old_account_id,
			reducible,
			Preservation::Expendable,
		) {
			Ok(_) => log::info!(
				target: LOG_TARGET,
				"Transferred treasury native asset funds from new account {:?} \
				to old account {:?} amount: {:?}",
				account_id,
				old_account_id,
				reducible
			),
			Err(e) => log::error!(
				target: LOG_TARGET,
				"Failed to transfer treasury funds from new account {:?} \
				to old account {:?} amount: {:?}, error: {:?}",
				account_id,
				old_account_id,
				reducible,
				e
			),
		};
	}
}
