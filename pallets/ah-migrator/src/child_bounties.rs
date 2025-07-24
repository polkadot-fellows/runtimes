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

//! Child Bounties migration.

use crate::*;
use pallet_rc_migrator::child_bounties::PortableChildBountiesMessage;

impl<T: Config> Pallet<T>
where
	<<T as pallet_treasury::Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber:
		From<u32>,
	pallet_treasury::BalanceOf<T>: From<u128>,
{
	pub fn do_receive_child_bounties_messages(
		messages: Vec<PortableChildBountiesMessage>,
	) -> Result<(), Error<T>> {
		let (mut good, mut bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} ChildBountiesMessages", messages.len());
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::ChildBounties,
			count: messages.len() as u32,
		});

		for message in messages {
			match Self::do_receive_child_bounties_message(message) {
				Ok(_) => good += 1,
				Err(_) => bad += 1,
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::ChildBounties,
			count_good: good as u32,
			count_bad: bad as u32,
		});

		Ok(())
	}

	fn do_receive_child_bounties_message(
		message: PortableChildBountiesMessage,
	) -> Result<(), Error<T>> {
		use PortableChildBountiesMessage::*;

		match message {
			ChildBountyCount(count) =>
				if pallet_child_bounties::ChildBountyCount::<T>::exists() {
					log::warn!(target: LOG_TARGET, "ChildBountyCount already exists, skipping");
				} else {
					pallet_child_bounties::ChildBountyCount::<T>::set(count);
				},
			ParentChildBounties(parent_id, count) => {
				pallet_child_bounties::ParentChildBounties::<T>::insert(parent_id, count);
			},
			ParentTotalChildBounties(parent_id, count) => {
				pallet_child_bounties::ParentTotalChildBounties::<T>::insert(parent_id, count);
			},
			ChildBounty { parent_id, child_id, child_bounty } => {
				let child_bounty: pallet_child_bounties::ChildBounty<
					AccountId32,
					pallet_treasury::BalanceOf<T>,
					pallet_treasury::BlockNumberFor<T>,
				> = child_bounty.into();
				pallet_child_bounties::ChildBounties::<T>::insert(
					parent_id,
					child_id,
					child_bounty,
				);
			},
			ChildBountyDescriptionsV1 { parent_id, child_id, description } => {
				// We take the bound from pallet-bounties since pallet-child-bounties re-uses it.
				let description = description
					.into_iter()
					.take(<T as pallet_bounties::Config>::MaximumReasonLength::get() as usize)
					.collect::<Vec<_>>();
				let description = BoundedVec::try_from(description).defensive().unwrap_or_default();

				pallet_child_bounties::ChildBountyDescriptionsV1::<T>::insert(
					parent_id,
					child_id,
					description,
				);
			},
			V0ToV1ChildBountyIds { v0_child_id, parent_id, v1_child_id } => {
				pallet_child_bounties::V0ToV1ChildBountyIds::<T>::insert(
					v0_child_id,
					(parent_id, v1_child_id),
				);
			},
			ChildrenCuratorFees { child_id, amount } => {
				let amount: pallet_treasury::BalanceOf<T> = amount.into();
				pallet_child_bounties::ChildrenCuratorFees::<T>::insert(child_id, amount);
			},
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: crate::Config> crate::types::AhMigrationCheck
	for pallet_rc_migrator::child_bounties::ChildBountiesMigratedCorrectly<T>
where
	<<T as pallet_treasury::Config>::BlockNumberProvider as BlockNumberProvider>::BlockNumber:
		From<u32>,
	pallet_treasury::BalanceOf<T>: From<u128>,
{
	type RcPrePayload = pallet_rc_migrator::child_bounties::RcData;
	type AhPrePayload = ();

	fn pre_check(_rc: Self::RcPrePayload) -> Self::AhPrePayload {
		assert_eq!(pallet_child_bounties::ChildBountyCount::<T>::get(), 0);
		assert_eq!(pallet_child_bounties::ParentChildBounties::<T>::iter().count(), 0);
		assert_eq!(pallet_child_bounties::ParentTotalChildBounties::<T>::iter().count(), 0);
		assert_eq!(pallet_child_bounties::ChildBountyDescriptionsV1::<T>::iter().count(), 0);
		assert_eq!(pallet_child_bounties::V0ToV1ChildBountyIds::<T>::iter().count(), 0);
		assert_eq!(pallet_child_bounties::ChildrenCuratorFees::<T>::iter().count(), 0);
	}

	fn post_check(rc: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		assert_eq!(rc.child_bounty_count, pallet_child_bounties::ChildBountyCount::<T>::get());
		assert_eq!(
			rc.parent_child_bounties,
			pallet_child_bounties::ParentChildBounties::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.parent_total_child_bounties,
			pallet_child_bounties::ParentTotalChildBounties::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.child_bounties
				.into_iter()
				.map(|(p, c, b)| (p, c, b.into()))
				.collect::<Vec<_>>(),
			pallet_child_bounties::ChildBounties::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.child_bounty_descriptions_v1,
			pallet_child_bounties::ChildBountyDescriptionsV1::<T>::iter()
				.map(|(p, c, d)| (p, c, d.into_inner()))
				.collect::<Vec<_>>()
		);
		assert_eq!(
			rc.v0_to_v1_child_bounty_ids,
			pallet_child_bounties::V0ToV1ChildBountyIds::<T>::iter().collect::<Vec<_>>()
		);
		assert_eq!(
			rc.children_curator_fees
				.into_iter()
				.map(|(c, a)| (c, a.into()))
				.collect::<Vec<_>>(),
			pallet_child_bounties::ChildrenCuratorFees::<T>::iter().collect::<Vec<_>>()
		);
	}
}
