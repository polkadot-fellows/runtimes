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
use pallet_rc_migrator::treasury::{
	alias as treasury_alias, RcSpendStatus, RcSpendStatusOf, TreasuryMigrator,
};
use pallet_treasury::{ProposalIndex, SpendIndex};
use sp_std::{sync::Arc, vec::Vec};
use xcm::VersionedLocation;

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
				let translated_proposal = pallet_treasury::Proposal {
					proposer: Self::translate_account_rc_to_ah(proposal.proposer),
					value: proposal.value,
					beneficiary: Self::translate_account_rc_to_ah(proposal.beneficiary),
					bond: proposal.bond,
				};
				pallet_treasury::Proposals::<T>::insert(proposal_index, translated_proposal);
			},
			RcTreasuryMessage::Approvals(approvals) => {
				let approvals = BoundedVec::<_, <T as pallet_treasury::Config>::MaxApprovals>::defensive_truncate_from(approvals);
				pallet_treasury::Approvals::<T>::put(approvals);
			},
			RcTreasuryMessage::SpendCount(spend_count) => {
				treasury_alias::SpendCount::<T>::put(spend_count);
			},
			RcTreasuryMessage::Spends { id: spend_index, status: spend } => {
				let treasury_alias::SpendStatus {
					asset_kind,
					amount,
					beneficiary,
					valid_from,
					expire_at,
					status,
				} = spend;

				// Apply account translation to beneficiary before type conversion
				let translated_beneficiary = Self::translate_beneficiary_location(beneficiary);

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
				let spend = treasury_alias::SpendStatus {
					asset_kind,
					amount,
					beneficiary,
					valid_from,
					expire_at,
					status,
				};
				log::debug!(target: LOG_TARGET, "Mapped treasury spend: {:?}", spend);
				treasury_alias::Spends::<T>::insert(spend_index, spend);
			},
			RcTreasuryMessage::LastSpendPeriod(last_spend_period) => {
				pallet_treasury::LastSpendPeriod::<T>::set(last_spend_period);
			},
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

	/// Translate AccountId32 junctions in a VersionedLocation from RC format to AH format.
	///
	/// This function handles all supported XCM versions (V3, V4, V5) and applies account
	/// translation to any AccountId32 junctions found within the location structure.
	/// All other junction types are preserved unchanged.
	fn translate_beneficiary_location(location: VersionedLocation) -> VersionedLocation {
		match location {
			VersionedLocation::V3(v3_location) => {
				match Self::translate_v3_location(v3_location.clone()) {
					Ok(translated) => VersionedLocation::V3(translated),
					Err(_) => {
						log::warn!(
							target: LOG_TARGET,
							"Failed to translate V3 location, returning original"
						);
						VersionedLocation::V3(v3_location)
					},
				}
			},
			VersionedLocation::V4(v4_location) => {
				match Self::translate_v4_location(v4_location.clone()) {
					Ok(translated) => VersionedLocation::V4(translated),
					Err(_) => {
						log::warn!(
							target: LOG_TARGET,
							"Failed to translate V4 location, returning original"
						);
						VersionedLocation::V4(v4_location)
					},
				}
			},
			VersionedLocation::V5(v5_location) => {
				match Self::translate_v5_location(v5_location.clone()) {
					Ok(translated) => VersionedLocation::V5(translated),
					Err(_) => {
						log::warn!(
							target: LOG_TARGET,
							"Failed to translate V5 location, returning original"
						);
						VersionedLocation::V5(v5_location)
					},
				}
			},
		}
	}

	/// Translate AccountId32 junctions in XCM v3 MultiLocation.
	fn translate_v3_location(
		location: xcm::v3::MultiLocation,
	) -> Result<xcm::v3::MultiLocation, ()> {
		use xcm::v3::{Junction, Junctions};

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(j1) => Junctions::X1(Self::translate_v3_junction(j1)?),
			Junctions::X2(j1, j2) =>
				Junctions::X2(Self::translate_v3_junction(j1)?, Self::translate_v3_junction(j2)?),
			Junctions::X3(j1, j2, j3) => Junctions::X3(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
			),
			Junctions::X4(j1, j2, j3, j4) => Junctions::X4(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
			),
			Junctions::X5(j1, j2, j3, j4, j5) => Junctions::X5(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
			),
			Junctions::X6(j1, j2, j3, j4, j5, j6) => Junctions::X6(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
				Self::translate_v3_junction(j6)?,
			),
			Junctions::X7(j1, j2, j3, j4, j5, j6, j7) => Junctions::X7(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
				Self::translate_v3_junction(j6)?,
				Self::translate_v3_junction(j7)?,
			),
			Junctions::X8(j1, j2, j3, j4, j5, j6, j7, j8) => Junctions::X8(
				Self::translate_v3_junction(j1)?,
				Self::translate_v3_junction(j2)?,
				Self::translate_v3_junction(j3)?,
				Self::translate_v3_junction(j4)?,
				Self::translate_v3_junction(j5)?,
				Self::translate_v3_junction(j6)?,
				Self::translate_v3_junction(j7)?,
				Self::translate_v3_junction(j8)?,
			),
		};

		Ok(xcm::v3::MultiLocation { parents: location.parents, interior: translated_junctions })
	}

	/// Translate AccountId32 junctions in XCM v4 Location.
	fn translate_v4_location(location: xcm::v4::Location) -> Result<xcm::v4::Location, ()> {
		use xcm::v4::{Junction, Junctions};

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X1(Arc::from([translated[0].clone()]))
			},
			Junctions::X2(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X2(Arc::from([translated[0].clone(), translated[1].clone()]))
			},
			Junctions::X3(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X3(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
				]))
			},
			Junctions::X4(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X4(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
				]))
			},
			Junctions::X5(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X5(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
				]))
			},
			Junctions::X6(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X6(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
				]))
			},
			Junctions::X7(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X7(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
				]))
			},
			Junctions::X8(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v4_junction(junction.clone())?);
				}
				Junctions::X8(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
					translated[7].clone(),
				]))
			},
		};

		Ok(xcm::v4::Location { parents: location.parents, interior: translated_junctions })
	}

	/// Translate AccountId32 junctions in XCM v5 Location.
	fn translate_v5_location(location: xcm::v5::Location) -> Result<xcm::v5::Location, ()> {
		use xcm::v5::{Junction, Junctions};

		let translated_junctions = match location.interior {
			Junctions::Here => Junctions::Here,
			Junctions::X1(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X1(Arc::from([translated[0].clone()]))
			},
			Junctions::X2(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X2(Arc::from([translated[0].clone(), translated[1].clone()]))
			},
			Junctions::X3(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X3(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
				]))
			},
			Junctions::X4(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X4(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
				]))
			},
			Junctions::X5(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X5(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
				]))
			},
			Junctions::X6(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X6(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
				]))
			},
			Junctions::X7(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X7(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
				]))
			},
			Junctions::X8(junctions) => {
				let mut translated = Vec::new();
				for junction in junctions.iter() {
					translated.push(Self::translate_v5_junction(junction.clone())?);
				}
				Junctions::X8(Arc::from([
					translated[0].clone(),
					translated[1].clone(),
					translated[2].clone(),
					translated[3].clone(),
					translated[4].clone(),
					translated[5].clone(),
					translated[6].clone(),
					translated[7].clone(),
				]))
			},
		};

		Ok(xcm::v5::Location { parents: location.parents, interior: translated_junctions })
	}

	/// Translate AccountId32 junction in XCM v3.
	fn translate_v3_junction(junction: xcm::v3::Junction) -> Result<xcm::v3::Junction, ()> {
		use xcm::v3::Junction;

		match junction {
			Junction::AccountId32 { network, id } => {
				// Convert [u8; 32] to AccountId, translate, then back to [u8; 32]
				let account_id = T::AccountId::decode(&mut &id[..]).map_err(|_| ())?;
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] =
					translated_account.encode().try_into().map_err(|_| ())?;

				Ok(Junction::AccountId32 { network, id: translated_id })
			},
			// All other junction types pass through unchanged
			other => Ok(other),
		}
	}

	/// Translate AccountId32 junction in XCM v4.
	fn translate_v4_junction(junction: xcm::v4::Junction) -> Result<xcm::v4::Junction, ()> {
		use xcm::v4::Junction;

		match junction {
			Junction::AccountId32 { network, id } => {
				// Convert [u8; 32] to AccountId, translate, then back to [u8; 32]
				let account_id = T::AccountId::decode(&mut &id[..]).map_err(|_| ())?;
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] =
					translated_account.encode().try_into().map_err(|_| ())?;

				Ok(Junction::AccountId32 { network, id: translated_id })
			},
			// All other junction types pass through unchanged
			other => Ok(other),
		}
	}

	/// Translate AccountId32 junction in XCM v5.
	fn translate_v5_junction(junction: xcm::v5::Junction) -> Result<xcm::v5::Junction, ()> {
		use xcm::v5::Junction;

		match junction {
			Junction::AccountId32 { network, id } => {
				// Convert [u8; 32] to AccountId, translate, then back to [u8; 32]
				let account_id = T::AccountId::decode(&mut &id[..]).map_err(|_| ())?;
				let translated_account = Self::translate_account_rc_to_ah(account_id);
				let translated_id: [u8; 32] =
					translated_account.encode().try_into().map_err(|_| ())?;

				Ok(Junction::AccountId32 { network, id: translated_id })
			},
			// All other junction types pass through unchanged
			other => Ok(other),
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
		Vec<(SpendIndex, RcSpendStatusOf<T>)>,
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
			treasury_alias::SpendCount::<T>::get(),
			0,
			"SpendCount should be 0 on Asset Hub before migration"
		);

		// Assert storage 'Treasury::Spends::ah_pre::empty'
		assert!(
			treasury_alias::Spends::<T>::iter().next().is_none(),
			"Spends should be empty on Asset Hub before migration"
		);
	}

	fn post_check(
		(proposals, proposals_count, approvals, spends, spends_count): Self::RcPrePayload,
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
			treasury_alias::SpendCount::<T>::get(),
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
			treasury_alias::Spends::<T>::iter_keys().count() as u32,
			spends.len() as u32,
			"Number of active spends on Asset Hub should match Relay Chain value"
		);

		// Assert storage 'Treasury::Spends::ah_post::consistent'
		let mut ah_spends = Vec::new();
		for (spend_id, spend) in treasury_alias::Spends::<T>::iter() {
			ah_spends.push((
				spend_id,
				RcSpendStatus {
					amount: spend.amount,
					valid_from: spend.valid_from,
					expire_at: spend.expire_at,
					status: spend.status.clone(),
				},
			));
		}
		// Assert storage 'Treasury::Spends::ah_post::correct'
		assert_eq!(
			ah_spends, spends,
			"Spends on Asset Hub should match migrated Spends from the relay chain"
		);
	}
}
