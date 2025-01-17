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

use crate::{types::*, *};
use frame_support::traits::{Consideration, Footprint};
use pallet_rc_migrator::preimage::{chunks::*, *};

impl<T: Config> Pallet<T> {
	pub fn do_receive_preimage_chunks(chunks: Vec<RcPreimageChunk>) -> Result<(), Error<T>> {
		Self::deposit_event(Event::PreimageChunkBatchReceived { count: chunks.len() as u32 });
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
		Self::deposit_event(Event::PreimageChunkBatchProcessed { count_good, count_bad });

		Ok(())
	}

	pub fn do_receive_preimage_chunk(chunk: RcPreimageChunk) -> Result<(), Error<T>> {
		log::debug!(target: LOG_TARGET, "Integrating preimage chunk {} offset {}/{}", chunk.preimage_hash, chunk.chunk_byte_offset + chunk.chunk_bytes.len() as u32, chunk.preimage_len);
		let key = (chunk.preimage_hash, chunk.preimage_len);

		// First check that we did not miss a chunk
		let preimage = match alias::PreimageFor::<T>::get(&key) {
			Some(mut preimage) => {
				if preimage.len() != chunk.chunk_byte_offset as usize {
					defensive!("Preimage chunk missing");
					return Err(Error::<T>::TODO);
				}

				match preimage.try_mutate(|p| {
					p.extend(chunk.chunk_bytes.clone());
				}) {
					Some(preimage) => {
						alias::PreimageFor::<T>::insert(&key, &preimage);
						preimage
					},
					None => {
						defensive!("Preimage too big");
						return Err(Error::<T>::TODO);
					},
				}
			},
			None => {
				if chunk.chunk_byte_offset != 0 {
					defensive!("Preimage chunk missing");
					return Err(Error::<T>::TODO);
				}

				let preimage: BoundedVec<u8, ConstU32<{ CHUNK_SIZE }>> = chunk.chunk_bytes;
				debug_assert!(CHUNK_SIZE <= pallet_rc_migrator::preimage::alias::MAX_SIZE);
				let bounded_preimage: BoundedVec<
					u8,
					ConstU32<{ pallet_rc_migrator::preimage::alias::MAX_SIZE }>,
				> = preimage.into_inner().try_into().expect("Asserted");
				alias::PreimageFor::<T>::insert(key, &bounded_preimage);
				bounded_preimage
			},
		};

		if preimage.len() == chunk.preimage_len as usize + chunk.chunk_byte_offset as usize {
			log::debug!(target: LOG_TARGET, "Preimage complete: {}", chunk.preimage_hash);
		}

		Ok(())
	}

	pub fn do_receive_preimage_request_statuses(
		request_status: Vec<RcPreimageRequestStatusOf<T>>,
	) -> Result<(), Error<T>> {
		Self::deposit_event(Event::PreimageRequestStatusBatchReceived {
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

		Self::deposit_event(Event::PreimageRequestStatusBatchProcessed { count_good, count_bad });
		Ok(())
	}

	pub fn do_receive_preimage_request_status(
		mut request_status: RcPreimageRequestStatusOf<T>,
	) -> Result<(), Error<T>> {
		if alias::RequestStatusFor::<T>::contains_key(&request_status.hash) {
			log::warn!(target: LOG_TARGET, "Request status already migrated: {:?}", request_status.hash);
			return Ok(());
		}

		let new_ticket = match request_status.request_status {
			alias::RequestStatus::Unrequested { ticket: (ref who, ref ticket), len } => {
				let fp = Footprint::from_parts(1, len as usize);
				ticket.clone().update(&who, fp).ok()
			},
			alias::RequestStatus::Requested {
				maybe_ticket: Some((ref who, ref ticket)),
				maybe_len: Some(len),
				..
			} => {
				let fp = Footprint::from_parts(1, len as usize);
				ticket.clone().update(&who, fp).ok()
			},
			alias::RequestStatus::Requested { maybe_ticket: Some(_), maybe_len: None, .. } => {
				defensive!("Ticket cannot be re-evaluated");
				// I think this is unreachable, but not exactly sure. Either way, nothing that we
				// could do about it.
				None
			},
			_ => None,
		};

		let new_request_status = match (new_ticket, request_status.request_status.clone()) {
			(
				Some(new_ticket),
				alias::RequestStatus::Unrequested { ticket: (who, ref mut ticket), len },
			) => alias::RequestStatus::Unrequested { ticket: (who, new_ticket), len },
			(
				Some(new_ticket),
				alias::RequestStatus::Requested {
					maybe_ticket: Some((who, ref mut ticket)),
					maybe_len: Some(len),
					count: count,
				},
			) => alias::RequestStatus::Requested {
				maybe_ticket: Some((who, new_ticket)),
				maybe_len: Some(len),
				count,
			},
			_ => request_status.request_status,
		};

		alias::RequestStatusFor::<T>::insert(&request_status.hash, &new_request_status);
		log::debug!(target: LOG_TARGET, "Integrating preimage request status: {:?}", new_request_status);

		Ok(())
	}
}
