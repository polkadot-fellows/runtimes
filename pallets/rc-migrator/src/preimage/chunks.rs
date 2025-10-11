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

/// Max size that we want a preimage chunk to be.
///
/// The -100 is to account for the encoding overhead and additional fields.
pub const CHUNK_SIZE: u32 = MAX_XCM_SIZE - 100;

/// A chunk of a preimage that was migrated out of the Relay and can be integrated into AH.
#[derive(
	Encode,
	DecodeWithMemTracking,
	Decode,
	TypeInfo,
	Clone,
	MaxEncodedLen,
	RuntimeDebug,
	PartialEq,
	Eq,
)]
pub struct RcPreimageChunk {
	/// The hash of the original preimage.
	pub preimage_hash: H256,
	/// The length of the original preimage.
	pub preimage_len: u32,
	/// Where this chunk starts in the original preimage.
	pub chunk_byte_offset: u32,
	/// A chunk of the original preimage.
	pub chunk_bytes: BoundedVec<u8, ConstU32<CHUNK_SIZE>>,
}

pub struct PreimageChunkMigrator<T: pallet_preimage::Config> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for PreimageChunkMigrator<T> {
	type Key = ((H256, u32), u32);
	type Error = Error<T>;

	// The `next_key` is the next key that we will migrate. Not the last one that we migrated.
	// This makes the code simpler.
	fn migrate_many(
		mut next_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let mut batch = XcmBatchAndMeter::new_from_config::<T>();
		let mut ah_weight_counter = WeightMeter::with_limit(T::MaxAhWeight::get());

		let last_key = loop {
			if weight_counter.try_consume(T::DbWeight::get().reads_writes(1, 2)).is_err()
				|| weight_counter.try_consume(batch.consume_weight()).is_err()
			{
				log::info!(
					target: LOG_TARGET,
					"RC weight limit reached at batch length {}, stopping",
					batch.len()
				);
				if batch.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break next_key;
				}
			}

			let (next_key_inner, mut last_offset) = match next_key {
				None => {
					let (maybe_next_key, skipped) = Self::next_key();
					// Remove skipped storage items that won't be migrated
					for (old_hash, old_len) in skipped {
						pallet_preimage::PreimageFor::<T>::remove((old_hash, old_len));
					}
					let Some(next_key) = maybe_next_key else {
						// No more preimages
						break None;
					};
					(next_key, 0)
				},
				Some(((hash, len), offset)) if offset < len => ((hash, len), offset),
				Some(((hash, len), _)) => {
					// Remove the previous key for which the migration is complete.
					pallet_preimage::PreimageFor::<T>::remove((hash, len));
					// Get the next key and remove the ones skipped before that.
					let (next_key_maybe, skipped) = Self::next_key();
					for (old_hash, old_len) in skipped {
						pallet_preimage::PreimageFor::<T>::remove((old_hash, old_len));
					}
					let Some(next_key) = next_key_maybe else {
						break None;
					};
					(next_key, 0)
				},
			};
			// Load the preimage
			let Some(preimage) = pallet_preimage::PreimageFor::<T>::get(next_key_inner) else {
				defensive!("Storage corruption {:?}", next_key_inner);
				// Remove the previous key for which the migration failed.
				pallet_preimage::PreimageFor::<T>::remove(next_key_inner);
				next_key = None;
				continue;
			};
			debug_assert!(last_offset < preimage.len() as u32);

			// Extract the chunk
			let chunk_bytes: Vec<u8> = preimage
				.iter()
				.skip(last_offset as usize)
				.take(CHUNK_SIZE as usize)
				.cloned()
				.collect();
			debug_assert!(!chunk_bytes.is_empty());

			let Ok(bounded_chunk) = BoundedVec::try_from(chunk_bytes.clone()).defensive() else {
				defensive!("Unreachable");
				// Remove the previous key for which the migration failed.
				pallet_preimage::PreimageFor::<T>::remove(next_key_inner);
				next_key = None;
				continue;
			};

			// check if AH can process the next chunk
			if ah_weight_counter
				.try_consume(T::AhWeightInfo::receive_preimage_chunk(last_offset / CHUNK_SIZE))
				.is_err()
			{
				log::info!("AH weight limit reached at batch length {}, stopping", batch.len());
				if batch.is_empty() {
					return Err(Error::OutOfWeight);
				} else {
					break Some((next_key_inner, last_offset));
				}
			}

			batch.push(RcPreimageChunk {
				preimage_hash: next_key_inner.0,
				preimage_len: next_key_inner.1,
				chunk_byte_offset: last_offset,
				chunk_bytes: bounded_chunk,
			});

			last_offset += chunk_bytes.len() as u32;
			log::debug!(
				target: LOG_TARGET,
				"Exported preimage chunk {next_key_inner:?} until offset {last_offset}"
			);

			// set the offset of the next_key
			next_key = Some((next_key_inner, last_offset));

			const MAX_CHUNKS_PER_BLOCK: u32 = 10;
			if batch.len() >= MAX_CHUNKS_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Maximum number of items ({}) to migrate per block reached, current batch size: {}",
					MAX_CHUNKS_PER_BLOCK,
					batch.len()
				);
				break next_key;
			}

			if batch.batch_count() >= MAX_XCM_MSG_PER_BLOCK {
				log::info!(
					target: LOG_TARGET,
					"Reached the maximum number of batches ({:?}) allowed per block; current batch count: {}",
					MAX_XCM_MSG_PER_BLOCK,
					batch.batch_count()
				);
				break next_key;
			}
		};

		if last_key.is_none() {
			log::info!(target: LOG_TARGET, "No more preimages");
		}

		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm_and_track(batch, |batch| {
				types::AhMigratorCall::<T>::ReceivePreimageChunks { chunks: batch }
			})?;
		}

		Ok(last_key)
	}
}

/// Key into the `PreimageFor` map.
type PreimageKey = (H256, u32);

impl<T: Config> PreimageChunkMigrator<T> {
	// Returns the next key to migrated and all the legacy preimages skipped before that, which will
	// be deleted
	#[allow(deprecated)] // StatusFor is deprecated
	fn next_key() -> (Option<PreimageKey>, Vec<PreimageKey>) {
		let mut skipped = Vec::new();
		let next_key_maybe = pallet_preimage::PreimageFor::<T>::iter_keys()
			// Skip all preimages that are tracked by the old `StatusFor` map. This is an unbounded
			// loop, but it cannot be exploited since the pallet does not allow to add more items to
			// the `StatusFor` map anymore.
			.find(|(hash, len)| {
				if pallet_preimage::RequestStatusFor::<T>::contains_key(hash) {
					true
				} else {
					log::info!(
						"Ignoring old preimage that is not in the request status map: {hash:?}"
					);
					skipped.push((*hash, *len));
					debug_assert!(
						pallet_preimage::StatusFor::<T>::contains_key(hash),
						"Preimage must be tracked somewhere"
					);
					false
				}
			});
		(next_key_maybe, skipped)
	}
}

impl<T: Config> RcMigrationCheck for PreimageChunkMigrator<T> {
	type RcPrePayload = Vec<PreimageKey>;

	fn pre_check() -> Self::RcPrePayload {
		let all_keys = pallet_preimage::PreimageFor::<T>::iter_keys().count();
		let good_keys = pallet_preimage::PreimageFor::<T>::iter_keys()
			.filter(|(hash, _)| pallet_preimage::RequestStatusFor::<T>::contains_key(hash))
			.count();
		log::info!("Migrating {good_keys} keys out of {all_keys}");
		pallet_preimage::PreimageFor::<T>::iter_keys()
			.filter(|(hash, _)| pallet_preimage::RequestStatusFor::<T>::contains_key(hash))
			.collect()
	}

	fn post_check(_rc_pre_payload: Self::RcPrePayload) {
		// "Assert storage 'Preimage::PreimageFor::rc_post::empty'"
		assert_eq!(
			pallet_preimage::PreimageFor::<T>::iter_keys().count(),
			0,
			"Preimage::PreimageFor is not empty on relay chain after migration"
		);
	}
}
