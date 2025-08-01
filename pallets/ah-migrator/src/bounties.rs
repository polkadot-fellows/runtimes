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

use crate::*;
use pallet_bounties::BountyStatus;
use pallet_rc_migrator::bounties::{
	alias::Bounty as RcBounty, BountiesMigrator, PortableBountiesMessage, PortableBountiesMessageOf,
	RcPrePayload,
};

impl<T: Config> Pallet<T> {
	/// Translates a bounty struct from RC format to AH format.
	///
	/// This function translates all account IDs in the bounty struct:
	/// - `proposer` field
	/// - Account IDs within the `status` field based on its variant
	/// Returns the same RC bounty type but with translated accounts
	fn translate_bounty(
		bounty: RcBounty<T::AccountId, pallet_treasury::BalanceOf<T>, BlockNumberFor<T>>,
	) -> RcBounty<T::AccountId, pallet_treasury::BalanceOf<T>, BlockNumberFor<T>> {
		let translated_proposer = Self::translate_account_rc_to_ah(bounty.proposer);
		let translated_status = Self::translate_bounty_status(bounty.status);

		RcBounty {
			proposer: translated_proposer,
			value: bounty.value,
			fee: bounty.fee,
			curator_deposit: bounty.curator_deposit,
			bond: bounty.bond,
			status: translated_status,
		}
	}

	/// Translates the status field of a bounty.
	///
	/// This function handles all variants of BountyStatus and translates
	/// account IDs where present.
	fn translate_bounty_status(
		status: BountyStatus<T::AccountId, BlockNumberFor<T>>,
	) -> BountyStatus<T::AccountId, BlockNumberFor<T>> {
		match status {
			BountyStatus::Proposed => BountyStatus::Proposed,
			BountyStatus::Approved => BountyStatus::Approved,
			BountyStatus::Funded => BountyStatus::Funded,
			BountyStatus::CuratorProposed { curator } =>
				BountyStatus::CuratorProposed { curator: Self::translate_account_rc_to_ah(curator) },
			BountyStatus::Active { curator, update_due } => BountyStatus::Active {
				curator: Self::translate_account_rc_to_ah(curator),
				update_due,
			},
			BountyStatus::PendingPayout { curator, beneficiary, unlock_at } =>
				BountyStatus::PendingPayout {
					curator: Self::translate_account_rc_to_ah(curator),
					beneficiary: Self::translate_account_rc_to_ah(beneficiary),
					unlock_at,
				},
			BountyStatus::ApprovedWithCurator { curator } => BountyStatus::ApprovedWithCurator {
				curator: Self::translate_account_rc_to_ah(curator),
			},
		}
	}

	pub fn do_receive_bounties_messages(
		messages: Vec<PortableBountiesMessageOf<T>>,
	) -> Result<(), Error<T>> {
		log::info!(target: LOG_TARGET, "Processing {} bounties messages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::Bounties,
			count: messages.len() as u32,
		});
		let (mut count_good, mut count_bad) = (0, 0);

		for message in messages {
			match Self::do_process_bounty_message(message) {
				Ok(()) => count_good += 1,
				Err(_) => count_bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::Bounties,
			count_good,
			count_bad,
		});
		log::info!(target: LOG_TARGET, "Processed {}/{} bounties messages", count_good, count_bad);

		Ok(())
	}

	fn do_process_bounty_message(message: PortableBountiesMessageOf<T>) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Processing bounties message: {:?}", message);

		match message {
			PortableBountiesMessage::BountyCount(count) => {
				log::debug!(target: LOG_TARGET, "Integrating bounties count: {:?}", count);
				pallet_bounties::BountyCount::<T>::put(count);
			},
			PortableBountiesMessage::BountyApprovals(approvals) => {
				log::debug!(target: LOG_TARGET, "Integrating bounties approvals: {:?}", approvals);
				let approvals = BoundedVec::<
                    _,
                    <T as pallet_treasury::Config>::MaxApprovals
                >::defensive_truncate_from(approvals);
				pallet_bounties::BountyApprovals::<T>::put(approvals);
			},
			PortableBountiesMessage::BountyDescriptions((index, description)) => {
				log::debug!(target: LOG_TARGET, "Integrating bounties descriptions: {:?}", description);
				let description = BoundedVec::<
					_,
					<T as pallet_bounties::Config>::MaximumReasonLength,
				>::defensive_truncate_from(description);
				pallet_bounties::BountyDescriptions::<T>::insert(index, description);
			},
			PortableBountiesMessage::Bounties((index, bounty)) => {
				log::debug!(target: LOG_TARGET, "Integrating bounty: {:?}", index);
				let translated_bounty = Self::translate_bounty(bounty);
				pallet_rc_migrator::bounties::alias::Bounties::<T>::insert(
					index,
					translated_bounty,
				);
			},
		}

		log::debug!(target: LOG_TARGET, "Processed bounties message");
		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for BountiesMigrator<T> {
	type RcPrePayload = RcPrePayload<T>;
	type AhPrePayload = ();

	fn pre_check(_rc_pre_payload: Self::RcPrePayload) -> Self::AhPrePayload {
		// "Assert storage 'Bounties::BountyCount::ah_pre::empty'"
		assert_eq!(
			pallet_bounties::BountyCount::<T>::get(),
			0,
			"Bounty count should be empty on asset hub before migration"
		);

		// "Assert storage 'Bounties::Bounties::ah_pre::empty'"
		assert!(
			pallet_bounties::Bounties::<T>::iter().next().is_none(),
			"The Bounties map should be empty on asset hub before migration"
		);

		// "Assert storage 'Bounties::BountyDescriptions::ah_pre::empty'"
		assert!(
			pallet_bounties::BountyDescriptions::<T>::iter().next().is_none(),
			"The Bounty Descriptions map should be empty on asset hub before migration"
		);

		// "Assert storage 'Bounties::BountyApprovals::ah_pre::empty'"
		assert!(
			pallet_bounties::BountyApprovals::<T>::get().is_empty(),
			"The Bounty Approvals vec should be empty on asset hub before migration"
		);
	}

	fn post_check(rc_pre_payload: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		let (rc_count, rc_bounties, rc_descriptions, rc_approvals) = rc_pre_payload;

		// Assert storage 'Bounties::BountyCount::ah_post::correct'
		assert_eq!(
			pallet_bounties::BountyCount::<T>::get(),
			rc_count,
			"Bounty count on Asset Hub should match the RC value"
		);

		// Assert storage 'Bounties::Bounties::ah_post::length'
		assert_eq!(
			pallet_bounties::Bounties::<T>::iter_keys().count(),
			rc_bounties.len(),
			"Bounties map length on Asset Hub should match the RC value"
		);

		// Verify that bounties were migrated successfully by checking the keys match
		let ah_bounty_keys: Vec<_> = pallet_bounties::Bounties::<T>::iter_keys().collect();
		let rc_bounty_keys: Vec<_> = rc_bounties.iter().map(|(index, _)| *index).collect();
		// Assert storage 'Bounties::Bounties::ah_post::correct'
		// Assert storage 'Bounties::Bounties::ah_post::consistent'
		assert_eq!(
			ah_bounty_keys, rc_bounty_keys,
			"Bounties map value on Asset Hub should match the RC value"
		);

		// Assert storage 'Bounties::BountyDescriptions::ah_post::length'
		assert_eq!(
			pallet_bounties::BountyDescriptions::<T>::iter_keys().count() as u32,
			rc_descriptions.len() as u32,
			"Bounty description map length on Asset Hub should match RC value"
		);

		// Assert storage 'Bounties::BountyDescriptions::ah_post::correct'
		// Assert storage 'Bounties::BountyDescriptions::ah_post::consistent'
		assert_eq!(
			pallet_bounties::BountyDescriptions::<T>::iter()
				.map(|(key, bounded_vec)| { (key, bounded_vec.into_inner()) })
				.collect::<Vec<_>>(),
			rc_descriptions,
			"Bounty descriptions map value on Asset Hub should match RC value"
		);

		// Assert storage 'Bounties::BountyApprovals::ah_post::length'
		assert_eq!(
			pallet_bounties::BountyApprovals::<T>::get().into_inner().len(),
			rc_approvals.len(),
			"Bounty approvals vec value on Asset Hub should match RC values"
		);

		// Assert storage 'Bounties::BountyApprovals::ah_post::correct'
		// Assert storage 'Bounties::BountyApprovals::ah_post::consistent'
		assert_eq!(
			pallet_bounties::BountyApprovals::<T>::get().into_inner(),
			rc_approvals,
			"Bounty approvals vec value on Asset Hub should match RC values"
		);
	}
}
