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

//! Commits the expected SRS chunk page hashes so `ChunksManager::add_chunks` can accept uploads.

use crate::Runtime;
use frame_support::{
	traits::{Get, OnRuntimeUpgrade},
	weights::Weight,
	BoundedVec,
};
use frame_system::RawOrigin;
use indiv_pallet_chunks_manager::{ChunkPageHashes, WeightInfo as ChunksManagerWeightInfo};
use indiv_support::traits::RingExponent;

const LOG_TARGET: &str = "runtime::people-polkadot::migrations";

/// Expected `blake2_256` hash of each SCALE-encoded page of Bandersnatch SRS chunks, in page order.
///
/// Each entry is `blake2_256(BoundedVec<StaticChunk<BandersnatchSuite>, PageSize>.encode())`
/// for one page — taken from the pinned `verifiable` crate's builder params, so it depends on that
/// pin and on the pallet's configured `PageSize`; `tests::chunk_page_hashes_match` recomputes it
/// and fails if either changes.
///
/// The runtime keeps only hashes, not chunk data: `ChunksManager::add_chunks` uploads pages later,
/// accepted only if they hash to a committed value. Embedding the builder params instead would cost
/// ~1.7 MiB of wasm.
mod hashes {
	use hex_literal::hex;

	pub const R2E9: [[u8; 32]; 3] = [
		hex!("8c2eef711d24f9dbffd5702f830f00e3762720a0f1661fa806aa0cc9639e9fc8"),
		hex!("0dbc02405f720aae749fb6904f0620b67275cb4065f9329e322d7cdcdd21f5d2"),
		hex!("dda83ece2d95ec4da802204b13069f1291f1c81b3489c45f6bb5916f3c5f54ef"),
	];

	pub const R2E10: [[u8; 32]; 5] = [
		hex!("3de492bc48ee3a3e654066d25fd84e49028dd5649125eef01a4a685892ce00ca"),
		hex!("1b5003a9358a5bc85de86df3738b0a9ee6ceb92c043c4d34c94db67fb7e4e03e"),
		hex!("ccfa502c7b67ab0213d320929e517613803e077413bceaa58b2948c9dd9720da"),
		hex!("2b70682bbc379926c4710b1df1e695bca14e2bb6cf858f07a9d9ac80089c0c5a"),
		hex!("a8556bc5772cffe977a68ae4e4794c8f24ebaf2fdc35f5d33ee2e33a81f65129"),
	];
}

/// Chunks per page the hashes in [`hashes`] were computed for — a count of chunks.
///
/// Mirrors the pallet's `PageSize` config, which bounds the element count of the pallet's
/// `BoundedVec<Chunk, PageSize>`. One Bandersnatch chunk is a 96-byte uncompressed BLS12-381 G1
/// point, so a full page hashes 255 * 96 + 2 = 24_482 bytes of SCALE-encoded preimage.
const HASHED_CHUNKS_PER_PAGE: u32 = 255;

/// The chunk page size the pallet is configured with.
fn configured_page_size() -> u32 {
	<Runtime as indiv_pallet_chunks_manager::Config>::PageSize::get()
}

/// Rings this runtime uses, paired with their committed page hashes.
fn committed_page_hashes() -> [(RingExponent, &'static [[u8; 32]]); 2] {
	[(RingExponent::R2e9, &hashes::R2E9[..]), (RingExponent::R2e10, &hashes::R2E10[..])]
}

/// Commits the SRS chunk page hashes, replacing the first manual step of the deployment: without
/// them `ChunksManager::add_chunks` rejects every page and the ring-VRF machinery stays inert.
pub struct InitializeChunkPageHashes;

impl OnRuntimeUpgrade for InitializeChunkPageHashes {
	fn on_runtime_upgrade() -> Weight {
		let db_weight = <Runtime as frame_system::Config>::DbWeight::get();
		let mut weight = Weight::zero();

		if configured_page_size() != HASHED_CHUNKS_PER_PAGE {
			log::error!(
				target: LOG_TARGET,
				"the configured page size is {} chunks but the committed hashes were computed for \
				 {HASHED_CHUNKS_PER_PAGE}; refusing to write page hashes",
				configured_page_size(),
			);
			return weight;
		}

		for (ring_exponent, hashes) in committed_page_hashes() {
			// A single stored page is enough to treat the ring as already committed.
			weight.saturating_accrue(db_weight.reads(1));
			if ChunkPageHashes::<Runtime>::iter_key_prefix(ring_exponent).next().is_some() {
				log::info!(
					target: LOG_TARGET,
					"chunk page hashes for ring {ring_exponent:?} already set, skipping"
				);
				continue;
			}

			let page_count = hashes.len() as u32;
			let Ok(page_hashes) = BoundedVec::try_from(hashes.to_vec()) else {
				log::error!(
					target: LOG_TARGET,
					"chunk page hashes for ring {ring_exponent:?} exceed the page count limit"
				);
				continue;
			};

			// The call consumes its benchmarked weight whether or not it succeeds.
			weight.saturating_accrue(
				<Runtime as indiv_pallet_chunks_manager::Config>::WeightInfo::set_chunk_page_hashes(
					page_count,
				),
			);
			match indiv_pallet_chunks_manager::Pallet::<Runtime>::set_chunk_page_hashes(
				RawOrigin::Root.into(),
				ring_exponent,
				page_hashes,
			) {
				Ok(()) => {
					log::info!(
						target: LOG_TARGET,
						"chunk page hashes for ring {ring_exponent:?} initialized ({page_count} pages)"
					);
				},
				Err(e) => {
					log::error!(
						target: LOG_TARGET,
						"failed to set chunk page hashes for ring {ring_exponent:?}: {e:?}"
					);
				},
			}
		}

		weight
	}

	#[cfg(feature = "try-runtime")]
	fn post_upgrade(_state: alloc::vec::Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
		use frame_support::ensure;

		ensure!(
			configured_page_size() == HASHED_CHUNKS_PER_PAGE,
			"committed chunk page hashes were computed for a different page size"
		);

		for (ring_exponent, hashes) in committed_page_hashes() {
			for (page_index, expected) in hashes.iter().enumerate() {
				let stored = ChunkPageHashes::<Runtime>::get(ring_exponent, page_index as u32)
					.ok_or("chunk page hash missing after initialization")?;
				ensure!(
					&stored == expected,
					"stored chunk page hash differs from the committed one"
				);
			}
			ensure!(
				ChunkPageHashes::<Runtime>::iter_key_prefix(ring_exponent).count() == hashes.len(),
				"unexpected number of chunk page hashes stored for this ring"
			);
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use codec::Encode;
	use indiv_support::{
		crypto::{BandersnatchSuite, BandersnatchVrfVerifiable, GenerateVerifiable},
		genesis::{ring_verifier_builder_params, ring_verifier_builder_params_hashes},
	};
	use verifiable::ring::RingDomainSize;

	fn domain_of(ring_exponent: RingExponent) -> RingDomainSize {
		ring_exponent.try_into().expect("every RingExponent maps to a RingDomainSize")
	}

	#[test]
	fn ring_exponent_selects_the_domain_with_that_many_chunks() {
		for ring_exponent in [RingExponent::R2e9, RingExponent::R2e10] {
			let chunks =
				ring_verifier_builder_params::<BandersnatchSuite>(domain_of(ring_exponent));
			assert_eq!(
				chunks.len(),
				1usize << ring_exponent.exponent(),
				"{ring_exponent:?} must have 2^{} chunks",
				ring_exponent.exponent(),
			);
		}
	}

	/// Committed hashes must match the pinned `verifiable` crate at this page size; bumping the
	/// crate or changing the pallet's `PageSize` config breaks this.
	#[test]
	fn chunk_page_hashes_match() {
		assert_eq!(
			configured_page_size(),
			HASHED_CHUNKS_PER_PAGE,
			"regenerate the committed chunk page hashes for the new page size"
		);

		for (ring_exponent, committed) in committed_page_hashes() {
			let expected = ring_verifier_builder_params_hashes::<BandersnatchSuite>(
				domain_of(ring_exponent),
				HASHED_CHUNKS_PER_PAGE,
			);
			assert_eq!(
				committed.to_vec(),
				expected,
				"committed page hashes for {ring_exponent:?} are stale — regenerate them with \
				 `cargo test -p people-polkadot-runtime --lib print_chunk_page_hashes -- \
				 --ignored --nocapture`"
			);
		}
	}

	#[test]
	fn each_committed_hash_accepts_only_its_own_page() {
		use crate::RuntimeGenesisConfig;
		use sp_runtime::BuildStorage;

		let mut ext = sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
		);
		ext.execute_with(|| {
			InitializeChunkPageHashes::on_runtime_upgrade();

			for (ring_exponent, committed) in committed_page_hashes() {
				let chunks =
					ring_verifier_builder_params::<BandersnatchSuite>(domain_of(ring_exponent));
				let pages: Vec<_> = chunks.chunks(HASHED_CHUNKS_PER_PAGE as usize).collect();
				assert_eq!(pages.len(), committed.len(), "page count for {ring_exponent:?}");

				for (page_index, page) in pages.iter().enumerate() {
					let encoded = page.to_vec().encode();
					let page_index = page_index as u32;
					assert!(
						indiv_pallet_chunks_manager::Pallet::<Runtime>::authorize_add_chunks(
							&ring_exponent,
							&page_index,
							&encoded,
						)
						.is_ok(),
						"page {page_index} of {ring_exponent:?} must authorize"
					);
					// The same bytes at any other index, and the wrong ring, must both fail.
					for other in (0..committed.len() as u32).filter(|i| *i != page_index) {
						assert!(
							indiv_pallet_chunks_manager::Pallet::<Runtime>::authorize_add_chunks(
								&ring_exponent,
								&other,
								&encoded,
							)
							.is_err(),
							"page {page_index} of {ring_exponent:?} must not authorize at {other}"
						);
					}
				}
			}
		});
	}

	#[test]
	fn committed_chunks_build_a_ring_at_their_exponent() {
		for ring_exponent in [RingExponent::R2e9, RingExponent::R2e10] {
			let domain = domain_of(ring_exponent);
			let chunks = ring_verifier_builder_params::<BandersnatchSuite>(domain);

			// Reassemble the chunks from the exact page split the hashes commit to, so the ring is
			// built from the same bytes `add_chunks` would have accepted.
			let paged: Vec<_> = chunks
				.chunks(HASHED_CHUNKS_PER_PAGE as usize)
				.flat_map(|page| page.to_vec())
				.collect();
			assert_eq!(paged, chunks);

			let secret = BandersnatchVrfVerifiable::new_secret([7u8; 32]);
			let member = BandersnatchVrfVerifiable::member_from_secret(&secret);
			let mut intermediate = BandersnatchVrfVerifiable::start_members(domain);
			BandersnatchVrfVerifiable::push_members(
				&mut intermediate,
				core::iter::once(member),
				|range| Ok(paged[range].to_vec()),
			)
			.expect("committed chunks must build a ring at their own exponent");
			let root = BandersnatchVrfVerifiable::finish_members(intermediate);

			// An empty ring at the same domain must differ, so the root is not a degenerate value.
			let empty = BandersnatchVrfVerifiable::finish_members(
				BandersnatchVrfVerifiable::start_members(domain),
			);
			assert_ne!(root.encode(), empty.encode(), "{ring_exponent:?} root must include member");
		}
	}

	#[test]
	fn migration_commits_hashes_and_is_idempotent() {
		use crate::RuntimeGenesisConfig;
		use sp_runtime::{
			transaction_validity::{InvalidTransaction, TransactionValidityError},
			BuildStorage,
		};

		let mut ext = sp_io::TestExternalities::new(
			RuntimeGenesisConfig::default().build_storage().expect("runtime genesis builds"),
		);
		ext.execute_with(|| {
			assert!(ChunkPageHashes::<Runtime>::iter().next().is_none());
			assert!(matches!(
				indiv_pallet_chunks_manager::Pallet::<Runtime>::authorize_add_chunks(
					&RingExponent::R2e9,
					&0,
					&[]
				),
				Err(TransactionValidityError::Invalid(InvalidTransaction::Call))
			));

			let weight = InitializeChunkPageHashes::on_runtime_upgrade();
			assert_ne!(weight, Weight::zero());

			for (ring_exponent, committed) in committed_page_hashes() {
				for (page_index, hash) in committed.iter().enumerate() {
					assert_eq!(
						ChunkPageHashes::<Runtime>::get(ring_exponent, page_index as u32).as_ref(),
						Some(hash),
					);
				}
				// Exactly the committed pages, nothing beyond them.
				assert_eq!(
					ChunkPageHashes::<Runtime>::iter_key_prefix(ring_exponent).count(),
					committed.len(),
				);
			}
			// No hashes for any ring this runtime does not use.
			assert_eq!(ChunkPageHashes::<Runtime>::iter_key_prefix(RingExponent::R2e14).count(), 0);

			// Re-running is a no-op: reads only, and no hash is rewritten.
			let before: Vec<_> = ChunkPageHashes::<Runtime>::iter().collect();
			let rerun = InitializeChunkPageHashes::on_runtime_upgrade();
			// One presence read per ring.
			assert_eq!(rerun, <Runtime as frame_system::Config>::DbWeight::get().reads(2));
			assert_eq!(ChunkPageHashes::<Runtime>::iter().collect::<Vec<_>>(), before);
		});
	}

	/// Pins the unit of [`HASHED_CHUNKS_PER_PAGE`]: it counts chunks, and one chunk is 96-byte,
	/// so a full page's preimage is 24_482 bytes including the SCALE length prefix.
	#[test]
	fn a_page_is_a_count_of_chunks() {
		use verifiable::ring::RingSuiteExt;

		let chunks =
			ring_verifier_builder_params::<BandersnatchSuite>(domain_of(RingExponent::R2e9));
		assert_eq!(<BandersnatchSuite as RingSuiteExt>::STATIC_CHUNK_SIZE, 96);

		let full_page = chunks[..HASHED_CHUNKS_PER_PAGE as usize].to_vec();
		assert_eq!(full_page.len(), HASHED_CHUNKS_PER_PAGE as usize);
		assert_eq!(full_page.encode().len(), 255 * 96 + 2);

		// The tail page is short in chunks, so it is also short in bytes.
		let tail = chunks[2 * HASHED_CHUNKS_PER_PAGE as usize..].to_vec();
		assert_eq!(tail.len(), 2);
		assert_eq!(tail.encode().len(), 2 * 96 + 1);
	}
}
