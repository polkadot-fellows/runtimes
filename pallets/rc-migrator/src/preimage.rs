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

#![doc = include_str!("multisig.md")]

use crate::{types::*, *};
use sp_runtime::traits::{BlakeTwo256, Hash};

pub mod alias {
	use super::*;

	use frame_support::{traits::Currency, Identity};
	use sp_core::ConstU32;

	pub const MAX_SIZE: u32 = 4 * 1024 * 1024;

	/// A type to note whether a preimage is owned by a user or the system.
	// Copied from https://github.com/paritytech/polkadot-sdk/blob/00946b10ab18331f959f5cbced7c433b6132b1cb/substrate/frame/preimage/src/lib.rs#L67-L77
	#[derive(Clone, Eq, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	pub enum OldRequestStatus<AccountId, Balance> {
		/// The associated preimage has not yet been requested by the system. The given deposit (if
		/// some) is being held until either it becomes requested or the user retracts the
		/// preimage.
		Unrequested { deposit: (AccountId, Balance), len: u32 },
		/// There are a non-zero number of outstanding requests for this hash by this chain. If
		/// there is a preimage registered, then `len` is `Some` and it may be removed iff this
		/// counter becomes zero.
		Requested { deposit: Option<(AccountId, Balance)>, count: u32, len: Option<u32> },
	}

	/// A type to note whether a preimage is owned by a user or the system.
	// Coped from https://github.com/paritytech/polkadot-sdk/blob/00946b10ab18331f959f5cbced7c433b6132b1cb/substrate/frame/preimage/src/lib.rs#L79-L89
	#[derive(Clone, Eq, PartialEq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
	pub enum RequestStatus<AccountId, Ticket> {
		/// The associated preimage has not yet been requested by the system. The given deposit (if
		/// some) is being held until either it becomes requested or the user retracts the
		/// preimage.
		Unrequested { ticket: (AccountId, Ticket), len: u32 },
		/// There are a non-zero number of outstanding requests for this hash by this chain. If
		/// there is a preimage registered, then `len` is `Some` and it may be removed iff this
		/// counter becomes zero.
		Requested { maybe_ticket: Option<(AccountId, Ticket)>, count: u32, maybe_len: Option<u32> },
	}

	// Coped from https://github.com/paritytech/polkadot-sdk/blob/00946b10ab18331f959f5cbced7c433b6132b1cb/substrate/frame/preimage/src/lib.rs#L91-L93
	type BalanceOf<T> = <<T as pallet_preimage::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	type TicketOf<T> = <T as pallet_preimage::Config>::Consideration;

	// Coped from https://github.com/paritytech/polkadot-sdk/blob/00946b10ab18331f959f5cbced7c433b6132b1cb/substrate/frame/preimage/src/lib.rs#L173-L185
	#[deprecated = "RequestStatusFor"]
	#[frame_support::storage_alias(pallet_name)]
	pub type StatusFor<T: pallet_preimage::Config> = StorageMap<
		pallet_preimage::Pallet<T>,
		Identity,
		H256,
		OldRequestStatus<<T as frame_system::Config>::AccountId, BalanceOf<T>>,
	>;

	#[frame_support::storage_alias(pallet_name)]
	pub type RequestStatusFor<T: pallet_preimage::Config> = StorageMap<
		pallet_preimage::Pallet<T>,
		Identity,
		H256,
		RequestStatus<<T as frame_system::Config>::AccountId, TicketOf<T>>,
	>;

	#[frame_support::storage_alias(pallet_name)]
	pub type PreimageFor<T: pallet_preimage::Config> = StorageMap<
		pallet_preimage::Pallet<T>,
		Identity,
		(H256, u32),
		BoundedVec<u8, ConstU32<MAX_SIZE>>,
	>;
}

pub const CHUNK_SIZE: u32 = 49_900; // about 50KiB

/// A chunk of a preimage that was migrated out of the Relay and can be integrated into AH.
#[derive(Encode, Decode, TypeInfo, Clone, MaxEncodedLen, RuntimeDebug, PartialEq, Eq)]
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

/// An entry of the `RequestStatusFor` storage map.
#[derive(Encode, Decode, TypeInfo, Clone, MaxEncodedLen, RuntimeDebug, PartialEq, Eq)]
pub struct RcPreimageRequestStatus<AccountId, Ticket> {
	/// The hash of the original preimage.
	pub hash: H256,
	/// The request status of the original preimage.
	pub request_status: alias::RequestStatus<AccountId, Ticket>,
}

/// An entry of the `StatusFor` storage map. Should only be used to unreserve funds on AH.
#[derive(Encode, Decode, TypeInfo, Clone, MaxEncodedLen, RuntimeDebug, PartialEq, Eq)]
pub struct RcPreimageLegacyStatus<AccountId, Balance> {
	/// The hash of the original preimage.
	pub hash: H256,
	/// The request status of the original preimage.
	pub request_status: alias::OldRequestStatus<AccountId, Balance>,
}

pub struct PreimageChunkMigrator<T: pallet_preimage::Config> {
	_phantom: PhantomData<T>,
}

impl<T: Config> PalletMigration for PreimageChunkMigrator<T> {
	type Key = (Option<(H256, u32)>, u32);
	type Error = Error<T>;

	fn migrate_many(
		mut last_key: Option<Self::Key>,
		weight_counter: &mut WeightMeter,
	) -> Result<Option<Self::Key>, Self::Error> {
		let (key_iter, mut last_offset) = match last_key {
			None => (alias::PreimageFor::<T>::iter_keys(), 0),
			Some((None, offset)) => (alias::PreimageFor::<T>::iter_keys(), offset),
			Some((Some((hash, len)), offset)) => (
				alias::PreimageFor::<T>::iter_keys_from(alias::PreimageFor::<T>::hashed_key_for(
					&(hash, len),
				)),
				offset,
			),
		};

		let mut batch = Vec::new();
		let mut current_key = None;

		for kv in key_iter {
			// If we're starting a new preimage, reset offset
			if current_key.as_ref() != Some(&kv) {
				current_key = Some(kv.clone());
				// Reset offset unless we're resuming this specific hash from a previous attempt
				let should_reset =
					last_key.as_ref().and_then(|k| k.0.as_ref()).map_or(true, |h| h != &kv);

				if should_reset {
					last_offset = 0;
				}
			}

			// Get the full preimage data once
			let full_data = alias::PreimageFor::<T>::get(&kv).unwrap_or_default();

			// Process chunks while there's still data to process
			while last_offset < kv.1 {
				// Calculate how many bytes remain to be processed
				let remaining_bytes = kv.1.saturating_sub(last_offset);
				let chunk_size = remaining_bytes.min(CHUNK_SIZE);

				// Extract the chunk
				let chunk_bytes: Vec<u8> = full_data
					.iter()
					.skip(last_offset as usize)
					.take(chunk_size as usize)
					.cloned()
					.collect();

				let bounded_chunk_bytes = BoundedVec::try_from(chunk_bytes)
					.expect("Chunk size is bounded by CHUNK_SIZE; qed");
				debug_assert!(bounded_chunk_bytes.len() == chunk_size as usize);

				batch.push(RcPreimageChunk {
					preimage_hash: kv.0,
					preimage_len: kv.1,
					chunk_byte_offset: last_offset,
					chunk_bytes: bounded_chunk_bytes,
				});

				log::debug!(
					target: LOG_TARGET,
					"Processed preimage chunk {:?} at offset {}",
					kv,
					last_offset
				);

				last_offset += chunk_size;

				// Return after processing 10 chunks, saving our progress
				if batch.len() >= 10 {
					Pallet::<T>::send_chunked_xcm(batch, |batch| {
						types::AhMigratorCall::<T>::ReceivePreimageChunks { chunks: batch }
					})?;
					return Ok(Some((Some(kv), last_offset)));
				}
			}

			// After finishing a preimage, update last_key and reset offset
			last_key = Some((Some(kv), 0));
			last_offset = 0;
		}

		// Send any remaining batch before finishing
		if !batch.is_empty() {
			Pallet::<T>::send_chunked_xcm(batch, |batch| {
				types::AhMigratorCall::<T>::ReceivePreimageChunks { chunks: batch }
			})?;
		}

		Ok(None) // No more preimages to process
	}
}

impl<T: Config> PalletMigrationChecks for PreimageChunkMigrator<T> {
	type Payload = Vec<(H256, u32)>;

	fn pre_check() -> Self::Payload {
		alias::PreimageFor::<T>::iter_keys().collect()
	}

	fn post_check(keys: Self::Payload) {
		// Check that all keys are inserted
		for (hash, len) in keys {
			assert!(alias::PreimageFor::<T>::contains_key(&(hash, len)));
		}
		// Integrity check that all preimages have the correct hash and length
		for (hash, len) in alias::PreimageFor::<T>::iter_keys() {
			let preimage = alias::PreimageFor::<T>::get(&(hash, len)).expect("Storage corrupted");

			assert_eq!(preimage.len(), len as usize);
			assert_eq!(BlakeTwo256::hash(preimage.as_slice()), hash);
		}
	}
}
