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
use pallet_rc_migrator::{
	treasury::{PortableSpendStatus, PortableTreasuryMessage, TreasuryMigrator},
	types::SortByEncoded,
};
use pallet_treasury::{ProposalIndex, SpendIndex};

impl<T: Config> Pallet<T> {
	pub fn do_receive_treasury_messages(messages: Vec<PortableTreasuryMessage>) -> DispatchResult {
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

	fn do_process_treasury_message(message: PortableTreasuryMessage) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Processing treasury message: {:?}", message);

		match message {
			PortableTreasuryMessage::ProposalCount(proposal_count) => {
				pallet_treasury::ProposalCount::<T>::put(proposal_count);
			},
			PortableTreasuryMessage::Proposals((proposal_index, proposal)) => {
				let translated_proposal = pallet_treasury::Proposal {
					proposer: Self::translate_account_rc_to_ah(proposal.proposer),
					value: proposal.value,
					beneficiary: Self::translate_account_rc_to_ah(proposal.beneficiary),
					bond: proposal.bond,
				};
				pallet_treasury::Proposals::<T>::insert(proposal_index, translated_proposal);
			},
			PortableTreasuryMessage::Approvals(approvals) => {
				let approvals = BoundedVec::<_, <T as pallet_treasury::Config>::MaxApprovals>::defensive_truncate_from(approvals);
				pallet_treasury::Approvals::<T>::put(approvals);
			},
			PortableTreasuryMessage::SpendCount(spend_count) => {
				pallet_treasury::SpendCount::<T>::put(spend_count);
			},
			PortableTreasuryMessage::Spends { id: spend_index, status: spend } => {
				let pallet_treasury::SpendStatus {
					asset_kind,
					amount,
					beneficiary,
					valid_from,
					expire_at,
					status,
				} = spend.into();

				// Apply account translation to beneficiary before type conversion
				let translated_beneficiary = Self::translate_beneficiary_location(beneficiary)
					.map_err(|_| {
						defensive!(
							"Failed to translate treasury spend beneficiary for spend: {:?}",
							spend_index
						);
						Error::FailedToConvertType
					})?;

				let (asset_kind, beneficiary) =
					T::RcToAhTreasurySpend::convert((asset_kind, translated_beneficiary)).map_err(
						|_| {
							defensive!(
								"Failed to convert RC treasury spend to AH treasury spend: {:?}",
								spend_index
							);
							Error::FailedToConvertType
						},
					)?;
				let spend = pallet_treasury::SpendStatus {
					asset_kind,
					amount,
					beneficiary,
					valid_from,
					expire_at,
					status,
				};
				log::debug!(target: LOG_TARGET, "Mapped treasury spend: {:?}", spend);
				pallet_treasury::Spends::<T>::insert(spend_index, spend);
			},
			PortableTreasuryMessage::LastSpendPeriod(last_spend_period) => {
				pallet_treasury::LastSpendPeriod::<T>::set(last_spend_period);
			},
			PortableTreasuryMessage::Funds => {
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

			if reducible.is_zero() {
				log::info!(
					target: LOG_TARGET,
					"Treasury old asset account is empty. asset: {:?}, old_account_id: {:?}",
					asset,
					old_account_id,
				);
				continue;
			}

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

		if reducible.is_zero() {
			log::info!(
				target: LOG_TARGET,
				"Treasury old native asset account is empty. old_account_id: {:?}",
				old_account_id,
			);
		} else {
			match <<T as Config>::Currency as Mutate<T::AccountId>>::transfer(
				&old_account_id,
				&account_id,
				reducible,
				Preservation::Expendable,
			) {
				Ok(_) => log::info!(
					target: LOG_TARGET,
					"Transferred treasury native asset funds from old account {:?} \
					to new account {:?} amount: {:?}",
					old_account_id,
					account_id,
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
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for TreasuryMigrator<T> {
	// (proposals with ids, data, historical proposals count, approvals ids, spends, historical
	// spends count)
	type RcPrePayload = (
		Vec<(
			ProposalIndex,
			pallet_treasury::Proposal<T::AccountId, pallet_treasury::BalanceOf<T>>,
		)>,
		u32,
		Vec<ProposalIndex>,
		Vec<(SpendIndex, PortableSpendStatus)>,
		u32,
	);
	type AhPrePayload = ();

	fn pre_check(_: Self::RcPrePayload) -> Self::AhPrePayload {
		// Assert storage 'Treasury::ProposalCount::ah_pre::empty'
		assert_eq!(
			pallet_treasury::ProposalCount::<T>::get(),
			0,
			"ProposalCount should be 0 on Asset Hub before migration"
		);

		// Assert storage 'Treasury::Approvals::ah_pre::empty'
		assert!(
			pallet_treasury::Approvals::<T>::get().is_empty(),
			"Approvals should be empty on Asset Hub before migration"
		);

		// Assert storage 'Treasury::Proposals::ah_pre::empty'
		assert!(
			pallet_treasury::Proposals::<T>::iter().next().is_none(),
			"Proposals should be empty on Asset Hub before migration"
		);

		// Assert storage 'Treasury::SpendCount::ah_pre::empty'
		assert_eq!(
			pallet_treasury::SpendCount::<T>::get(),
			0,
			"SpendCount should be 0 on Asset Hub before migration"
		);

		// Assert storage 'Treasury::Spends::ah_pre::empty'
		assert!(
			pallet_treasury::Spends::<T>::iter().next().is_none(),
			"Spends should be empty on Asset Hub before migration"
		);
	}

	fn post_check(
		(proposals, proposals_count, approvals, rc_spends, spends_count): Self::RcPrePayload,
		_: Self::AhPrePayload,
	) {
		// Assert storage 'Treasury::ProposalCount::ah_post::correct'
		assert_eq!(
			pallet_treasury::ProposalCount::<T>::get(),
			proposals_count,
			"ProposalCount on Asset Hub should match Relay Chain value"
		);

		// Assert storage 'Treasury::SpendCount::ah_post::correct'
		assert_eq!(
			pallet_treasury::SpendCount::<T>::get(),
			spends_count,
			"SpendCount on Asset Hub should match Relay Chain value"
		);

		// Assert storage 'Treasury::ProposalCount::ah_post::consistent'
		// Assert storage 'Treasury::Proposals::ah_post::length'
		assert_eq!(
			pallet_treasury::Proposals::<T>::iter_keys().count() as u32,
			proposals.len() as u32,
			"Number of active proposals on Asset Hub should match Relay Chain value"
		);

		// Assert storage 'Treasury::Proposals::ah_post::consistent'
		// Assert storage 'Treasury::Proposals::ah_post::correct'
		let rc_proposals_translated: Vec<(
			ProposalIndex,
			pallet_treasury::Proposal<T::AccountId, pallet_treasury::BalanceOf<T>>,
		)> = proposals
			.into_iter()
			.map(|(proposal_index, proposal)| {
				let translated_proposal = pallet_treasury::Proposal {
					proposer: Pallet::<T>::translate_account_rc_to_ah(proposal.proposer),
					value: proposal.value,
					beneficiary: Pallet::<T>::translate_account_rc_to_ah(proposal.beneficiary),
					bond: proposal.bond,
				};
				(proposal_index, translated_proposal)
			})
			.collect();

		let ah_proposals: Vec<(
			ProposalIndex,
			pallet_treasury::Proposal<T::AccountId, pallet_treasury::BalanceOf<T>>,
		)> = pallet_treasury::Proposals::<T>::iter().collect();

		assert_eq!(
			rc_proposals_translated, ah_proposals,
			"Proposals on Asset Hub should match translated Relay Chain proposals"
		);

		// Assert storage 'Treasury::Approvals::ah_post::correct'
		// Assert storage 'Treasury::Approvals::ah_post::consistent'
		assert_eq!(
			pallet_treasury::Approvals::<T>::get().into_inner(),
			approvals,
			"Approvals on Asset Hub should match Relay Chain approvals"
		);

		// Assert storage 'Treasury::Approvals::ah_post::length'
		assert_eq!(
			pallet_treasury::Approvals::<T>::get().into_inner().len(),
			approvals.len(),
			"Treasury::Approvals::ah_post::length"
		);

		// Assert storage 'Treasury::SpendCount::ah_post::consistent'
		// Assert storage 'Treasury::SpendCount::ah_post::length'
		assert_eq!(
			pallet_treasury::Spends::<T>::iter_keys().count() as u32,
			rc_spends.len() as u32,
			"Number of active spends on Asset Hub should match Relay Chain value"
		);

		// Assert storage 'Treasury::Spends::ah_post::consistent'
		let mut ah_spends = pallet_treasury::Spends::<T>::iter().collect::<Vec<_>>();

		let mut untranslated_rc_spends = Vec::new();
		for (spend_id, spend) in rc_spends {
			let translated_beneficiary =
				crate::Pallet::<T>::translate_beneficiary_location(spend.beneficiary).unwrap();

			let (asset_kind, beneficiary) =
				T::RcToAhTreasurySpend::convert((spend.asset_kind, translated_beneficiary))
					.unwrap();

			untranslated_rc_spends.push((
				spend_id,
				pallet_treasury::SpendStatus {
					asset_kind,
					amount: spend.amount,
					beneficiary,
					valid_from: spend.valid_from,
					expire_at: spend.expire_at,
					status: spend.status.into(),
				},
			));
		}

		ah_spends.sort_by_encoded();
		untranslated_rc_spends.sort_by_encoded(); // VersionedLocatableAsset is not Ord

		// Assert storage 'Treasury::Spends::ah_post::correct'
		for (rc_spend, ah_spend) in untranslated_rc_spends.iter().zip(ah_spends.iter()) {
			assert_eq!(rc_spend, ah_spend);
		}
	}
}
