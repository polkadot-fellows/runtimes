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
use frame_support::traits::{Consideration, Footprint};
use pallet_rc_migrator::preimage::{chunks::*, *};
use sp_runtime::traits::{BlakeTwo256, Hash};

// NOTE: preimage doesn't require post-check account translation: the account translation is applied
// during processing and the post-checks focus on hash integrity rather than account-based
// comparisons.
impl<T: Config> Pallet<T> {
	pub fn do_receive_preimage_chunks(chunks: Vec<RcPreimageChunk>) -> Result<(), Error<T>> {
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::PreimageChunk,
			count: chunks.len() as u32,
		});
		let (mut count_good, mut count_bad) = (0, 0);
		log::info!(target: LOG_TARGET, "Integrating {} preimage chunks", chunks.len());

		for chunk in chunks {
			match Self::do_receive_preimage_chunk(chunk) {
				Ok(()) => count_good += 1,
				Err(e) => {
					count_bad += 1;
					log::error!(target: LOG_TARGET, "Error while integrating preimage chunk: {:?}", e);
				},
			}
		}
		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::PreimageChunk,
			count_good,
			count_bad,
		});

		Ok(())
	}

	pub fn do_receive_preimage_chunk(chunk: RcPreimageChunk) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Integrating preimage chunk {} offset {}/{}", chunk.preimage_hash, chunk.chunk_byte_offset + chunk.chunk_bytes.len() as u32, chunk.preimage_len);
		let key = (chunk.preimage_hash, chunk.preimage_len);

		// First check that we did not miss a chunk
		let preimage = match pallet_preimage::PreimageFor::<T>::get(key) {
			Some(preimage) => {
				if preimage.len() != chunk.chunk_byte_offset as usize {
					defensive!("Preimage chunk missing");
					return Err(Error::<T>::PreimageChunkMissing);
				}

				match preimage.try_mutate(|p| {
					p.extend(chunk.chunk_bytes.clone());
				}) {
					Some(preimage) => {
						pallet_preimage::PreimageFor::<T>::insert(key, &preimage);
						preimage
					},
					None => {
						defensive!("Preimage too big");
						return Err(Error::<T>::PreimageTooBig);
					},
				}
			},
			None => {
				if chunk.chunk_byte_offset != 0 {
					defensive!("Preimage chunk missing");
					return Err(Error::<T>::PreimageChunkMissing);
				}

				let preimage: BoundedVec<u8, ConstU32<{ CHUNK_SIZE }>> = chunk.chunk_bytes;
				debug_assert!(CHUNK_SIZE <= pallet_preimage::MAX_SIZE);
				let bounded_preimage: BoundedVec<u8, ConstU32<{ pallet_preimage::MAX_SIZE }>> =
					preimage.into_inner().try_into().expect("Asserted");
				pallet_preimage::PreimageFor::<T>::insert(key, &bounded_preimage);
				bounded_preimage
			},
		};

		if preimage.len() == chunk.preimage_len as usize + chunk.chunk_byte_offset as usize {
			log::debug!(target: LOG_TARGET, "Preimage complete: {}", chunk.preimage_hash);
		}

		Ok(())
	}

	pub fn do_receive_preimage_request_statuses(
		request_status: Vec<PortableRequestStatus>,
	) -> Result<(), Error<T>> {
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::PreimageRequestStatus,
			count: request_status.len() as u32,
		});
		log::info!(target: LOG_TARGET, "Integrating {} preimage request status", request_status.len());
		let (mut count_good, mut count_bad) = (0, 0);

		for request_status in request_status {
			match Self::do_receive_preimage_request_status(request_status) {
				Ok(()) => count_good += 1,
				Err(e) => {
					count_bad += 1;
					log::error!(target: LOG_TARGET, "Error while integrating preimage request status: {:?}", e);
				},
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::PreimageRequestStatus,
			count_good,
			count_bad,
		});
		Ok(())
	}

	pub fn do_receive_preimage_request_status(
		request_status: PortableRequestStatus,
	) -> Result<(), Error<T>> {
		if pallet_preimage::RequestStatusFor::<T>::contains_key(request_status.hash) {
			log::warn!(target: LOG_TARGET, "Request status already migrated: {:?}", request_status.hash);
			return Ok(());
		}

		if !pallet_preimage::PreimageFor::<T>::iter_keys()
			.any(|(key_hash, _)| key_hash == request_status.hash)
		{
			log::error!("Missing preimage for request status hash {:?}", request_status.hash);
			return Err(Error::<T>::PreimageMissing);
		}
		let ah_status: pallet_preimage::RequestStatus<AccountId32, pallet_preimage::TicketOf<T>> =
			request_status
				.request_status
				.try_into()
				.defensive()
				.map_err(|_| Error::<T>::PreimageStatusInvalid)?;

		let new_ticket = match ah_status {
			pallet_preimage::RequestStatus::Unrequested { ticket: (ref who, ref ticket), len } => {
				let fp = Footprint::from_parts(1, len as usize);
				ticket.clone().update(who, fp).ok()
			},
			pallet_preimage::RequestStatus::Requested {
				maybe_ticket: Some((ref who, ref ticket)),
				maybe_len: Some(len),
				..
			} => {
				let fp = Footprint::from_parts(1, len as usize);
				ticket.clone().update(who, fp).ok()
			},
			pallet_preimage::RequestStatus::Requested {
				maybe_ticket: Some(_),
				maybe_len: None,
				..
			} => {
				defensive!("Ticket cannot be re-evaluated");
				// I think this is unreachable, but not exactly sure. Either way, nothing that we
				// could do about it.
				None
			},
			_ => None,
		};

		let new_request_status = match (new_ticket, ah_status.clone()) {
			(
				Some(new_ticket),
				pallet_preimage::RequestStatus::Unrequested { ticket: (who, _), len },
			) => pallet_preimage::RequestStatus::Unrequested {
				ticket: (Self::translate_account_rc_to_ah(who), new_ticket),
				len,
			},
			(
				Some(new_ticket),
				pallet_preimage::RequestStatus::Requested {
					maybe_ticket: Some((who, _)),
					maybe_len: Some(len),
					count,
				},
			) => pallet_preimage::RequestStatus::Requested {
				maybe_ticket: Some((Self::translate_account_rc_to_ah(who), new_ticket)),
				maybe_len: Some(len),
				count,
			},
			_ => match ah_status {
				pallet_preimage::RequestStatus::Unrequested { ticket: (who, ticket), len } =>
					pallet_preimage::RequestStatus::Unrequested {
						ticket: (Self::translate_account_rc_to_ah(who), ticket),
						len,
					},
				pallet_preimage::RequestStatus::Requested { maybe_ticket, count, maybe_len } => {
					let translated_maybe_ticket = maybe_ticket
						.map(|(who, ticket)| (Self::translate_account_rc_to_ah(who), ticket));
					pallet_preimage::RequestStatus::Requested {
						maybe_ticket: translated_maybe_ticket,
						count,
						maybe_len,
					}
				},
			},
		};

		pallet_preimage::RequestStatusFor::<T>::insert(request_status.hash, &new_request_status);
		log::debug!(target: LOG_TARGET, "Integrating preimage request status: {:?}", new_request_status);

		Ok(())
	}

	pub fn do_receive_preimage_legacy_statuses(
		statuses: Vec<RcPreimageLegacyStatusOf<T>>,
	) -> Result<(), Error<T>> {
		Self::deposit_event(Event::BatchReceived {
			pallet: PalletEventName::PreimageLegacyStatus,
			count: statuses.len() as u32,
		});
		log::info!(target: LOG_TARGET, "Integrating {} preimage legacy status", statuses.len());
		let (mut count_good, mut count_bad) = (0, 0);

		for status in statuses {
			match Self::do_receive_preimage_legacy_status(status) {
				Ok(()) => count_good += 1,
				Err(_) => {
					count_bad += 1;
				},
			}
		}

		Self::deposit_event(Event::BatchProcessed {
			pallet: PalletEventName::PreimageLegacyStatus,
			count_good,
			count_bad,
		});
		Ok(())
	}

	pub fn do_receive_preimage_legacy_status(
		status: RcPreimageLegacyStatusOf<T>,
	) -> Result<(), Error<T>> {
		let translated_depositor = Self::translate_account_rc_to_ah(status.depositor);

		// Unreserve the deposit from the translated account
		let missing = <T as pallet_preimage::Config>::Currency::unreserve(
			&translated_depositor,
			status.deposit,
		);

		if missing != Default::default() {
			log::error!(target: LOG_TARGET, "Failed to unreserve deposit for preimage legacy status {:?}, who: {}, missing {:?}", status.hash, translated_depositor.to_ss58check(), missing);
			return Err(Error::<T>::FailedToUnreserveDeposit);
		}

		Ok(())
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for PreimageChunkMigrator<T> {
	type RcPrePayload = Vec<(H256, u32)>;
	type AhPrePayload = ();

	fn pre_check(_rc_pre_payload: Self::RcPrePayload) -> Self::AhPrePayload {
		// AH does not have a preimage pallet, therefore must be empty.
		assert!(
			pallet_preimage::PreimageFor::<T>::iter_keys().next().is_none(),
			"Assert storage 'Preimage::PreimageFor::ah_pre::empty'"
		);
	}

	// The payload should come from the relay chain pre-check method on the same pallet
	fn post_check(rc_pre_payload: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		// Assert storage "Preimage::PreimageFor::ah_post::consistent"
		for (key, preimage) in pallet_preimage::PreimageFor::<T>::iter() {
			assert!(preimage.len() > 0, "Preimage::PreimageFor is empty");
			assert!(preimage.len() <= 4 * 1024 * 1024_usize, "Preimage::PreimageFor is too big");
			assert!(
				preimage.len() == key.1 as usize,
				"Preimage::PreimageFor is not the correct length"
			);
			assert!(
				<T as frame_system::Config>::Hashing::hash(&preimage) == key.0,
				"Preimage::PreimageFor hash mismatch"
			);
			// Assert storage "Preimage::RequestStatusFor::ah_post::consistent"
			assert!(
				pallet_preimage::RequestStatusFor::<T>::contains_key(key.0),
				"Preimage::RequestStatusFor is missing"
			);
		}

		let new_preimages = alias::PreimageFor::<T>::iter_keys().count();
		// Some preimages may have been deleted as a side effect of being unrequested during
		// migration.
		if new_preimages != rc_pre_payload.len() {
			log::warn!(
				"Preimage::PreimageFor and relay chain payload have different size: {} vs {}",
				new_preimages,
				rc_pre_payload.len(),
			);
		}

		// All items have been successfully migrated from the relay chain
		// Assert storage "Preimage::PreimageFor::ah_post::correct"
		for (hash, len) in rc_pre_payload.iter() {
			assert!(
				alias::PreimageFor::<T>::contains_key((hash, len)),
				"Relay chain Preimage::PreimageFor storage item with key {:?} {:?} is not found on Asset Hub",
				hash,
				len,
			);
		}

		// Integrity check that all preimages have the correct hash and length
		// Assert storage "Preimage::PreimageFor::ah_post::consistent"
		for (hash, len) in pallet_preimage::PreimageFor::<T>::iter_keys() {
			let preimage =
				pallet_preimage::PreimageFor::<T>::get((hash, len)).expect("Storage corrupted");

			assert_eq!(preimage.len(), len as usize);
			assert_eq!(BlakeTwo256::hash(preimage.as_slice()), hash);
		}
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for PreimageRequestStatusMigrator<T> {
	type RcPrePayload = Vec<(H256, bool)>;
	type AhPrePayload = ();

	fn pre_check(_rc_pre_payload: Self::RcPrePayload) -> Self::AhPrePayload {
		// AH does not have a preimage pallet, therefore must be empty.
		// Assert storage "Preimage::RequestStatusFor::ah_pre::empty"
		assert!(
			pallet_preimage::RequestStatusFor::<T>::iter_keys().next().is_none(),
			"Preimage::RequestStatusFor is not empty"
		);
	}

	// The payload should come from the relay chain pre-check method on the same pallet
	fn post_check(rc_pre_payload: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		for (hash, requested) in rc_pre_payload.iter() {
			// Assert storage "Preimage::RequestStatusFor::ah_post::correct"
			assert!(
				alias::RequestStatusFor::<T>::contains_key(hash),
				"Relay chain Preimage::RequestStatusFor storage item with key {:?} is not found on Asset Hub",
				hash
			);
			match alias::RequestStatusFor::<T>::get(hash).unwrap() {
				alias::RequestStatus::Unrequested { len, .. } => {
					assert!(
						alias::PreimageFor::<T>::contains_key((hash, len)),
						"Preimage::RequestStatusFor is missing preimage"
					);
					assert!(
						!requested,
						"Preimage with hash {:?} should be requested on Asset Hub, but is unrequested instead",
						hash
					);
				},
				alias::RequestStatus::Requested { maybe_len: Some(len), .. } => {
					assert!(
						alias::PreimageFor::<T>::contains_key((hash, len)),
						"Preimage::RequestStatusFor is missing preimage"
					);
					assert!(
						requested,
						"Preimage with hash {:?} should be unrequested on Asset Hub, but is requested instead",
						hash
					);
				},
				alias::RequestStatus::Requested { .. } => {
					assert!(
						requested,
						"Preimage with hash {:?} should be unrequested on Asset Hub, but is requested instead",
						hash
					);
				},
			}
		}

		for hash in pallet_preimage::RequestStatusFor::<T>::iter_keys() {
			// Preimages for referendums that did not pass on the relay chain can be noted when
			// migrating to Asset Hub.
			if !rc_pre_payload.contains(&(hash, true)) && !rc_pre_payload.contains(&(hash, false)) {
				log::warn!("Asset Hub migrated Preimage::RequestStatusFor storage item with key {:?} was not present on the relay chain", hash);
			}
		}

		// Assert storage "Preimage::PreimageFor::ah_post::consistent"
		assert_eq!(
			pallet_preimage::PreimageFor::<T>::iter_keys().count(),
			pallet_preimage::RequestStatusFor::<T>::iter_keys().count(),
			"Preimage::PreimageFor and Preimage::RequestStatusFor have different lengths on Asset Hub"
		);
	}
}

#[cfg(feature = "std")]
impl<T: Config> crate::types::AhMigrationCheck for PreimageLegacyRequestStatusMigrator<T> {
	type RcPrePayload = Vec<H256>;
	type AhPrePayload = ();

	#[allow(deprecated)] // StatusFor
	fn pre_check(_rc_pre_payload: Self::RcPrePayload) -> Self::AhPrePayload {
		// AH does not have a preimage pallet, therefore must be empty.
		// Assert storage "Preimage::StatusFor::ah_pre::empty"
		assert!(
			pallet_preimage::StatusFor::<T>::iter_keys().next().is_none(),
			"Preimage::StatusFor is not empty on the relay chain"
		);
	}

	#[allow(deprecated)] // StatusFor
	fn post_check(_rc_pre_payload: Self::RcPrePayload, _ah_pre_payload: Self::AhPrePayload) {
		// All items have been deleted
		// Assert storage "Preimage::StatusFor::ah_post::correct"
		assert!(
			pallet_preimage::StatusFor::<T>::iter_keys().next().is_none(),
			"Preimage::StatusFor is not empty on assetHub"
		);
	}
}
