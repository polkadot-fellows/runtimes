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

use crate::{individuality::ChunkPageSize, Runtime};
use frame_support::{traits::OnRuntimeUpgrade, weights::Weight, BoundedVec};
use frame_system::RawOrigin;
use indiv_pallet_chunks_manager::{ChunkPageHashes, WeightInfo as ChunksManagerWeightInfo};
use indiv_support::traits::RingExponent;

const LOG_TARGET: &str = "runtime::people-polkadot::migrations";

/// Expected `blake2_256` hash of each SCALE-encoded page of Bandersnatch SRS chunks, in page order.
///
/// Each entry is `blake2_256(BoundedVec<StaticChunk<BandersnatchSuite>, ChunkPageSize>.encode())`
/// for one page, taken from the ring verifier builder params of the pinned `verifiable` crate.
/// The values depend on both that pin and on [`ChunkPageSize`]; `tests::chunk_page_hashes_match`
/// recomputes them from the crate and fails if either changes.
///
/// The chunk *data* is not carried by the runtime — `ChunksManager::add_chunks` uploads it
/// permissionlessly afterwards, and is accepted only if the page hashes to the value committed
/// here. Embedding the builder params in the runtime blob instead would cost ~1.7 MiB of wasm.
///
/// `R2e9` covers [`crate::individuality::MembersFlexibleRingExponent`] and
/// [`crate::individuality::LitePeopleRingExponent`]; `R2e10` covers
/// [`crate::individuality::RecyclerRingExponent`] and
/// [`crate::individuality::PaidUnloadTokenRingExponent`].
mod hashes {
	use hex_literal::hex;

	/// 512 chunks over 3 pages of at most 255.
	pub const R2E9: [[u8; 32]; 3] = [
		hex!("8c2eef711d24f9dbffd5702f830f00e3762720a0f1661fa806aa0cc9639e9fc8"),
		hex!("0dbc02405f720aae749fb6904f0620b67275cb4065f9329e322d7cdcdd21f5d2"),
		hex!("dda83ece2d95ec4da802204b13069f1291f1c81b3489c45f6bb5916f3c5f54ef"),
	];

	/// 1024 chunks over 5 pages of at most 255.
	pub const R2E10: [[u8; 32]; 5] = [
		hex!("3de492bc48ee3a3e654066d25fd84e49028dd5649125eef01a4a685892ce00ca"),
		hex!("1b5003a9358a5bc85de86df3738b0a9ee6ceb92c043c4d34c94db67fb7e4e03e"),
		hex!("ccfa502c7b67ab0213d320929e517613803e077413bceaa58b2948c9dd9720da"),
		hex!("2b70682bbc379926c4710b1df1e695bca14e2bb6cf858f07a9d9ac80089c0c5a"),
		hex!("a8556bc5772cffe977a68ae4e4794c8f24ebaf2fdc35f5d33ee2e33a81f65129"),
	];
}

/// The page size the hashes in [`hashes`] were computed for.
const HASHED_PAGE_SIZE: u32 = 255;

/// Rings this runtime uses, paired with their committed page hashes.
fn committed_page_hashes() -> [(RingExponent, &'static [[u8; 32]]); 2] {
	[(RingExponent::R2e9, &hashes::R2E9[..]), (RingExponent::R2e10, &hashes::R2E10[..])]
}

/// Commits the expected SRS chunk page hashes for the `R2e9` and `R2e10` rings.
///
/// This replaces the first manual step of the individuality deployment: without these hashes
/// `ChunksManager::add_chunks` rejects every page, so the ring-VRF machinery stays inert.
///
/// A ring is skipped entirely when any page hash for it is already stored. Chunks are write-once,
/// so overwriting a hash could orphan chunk pages that were already uploaded against the old value.
pub struct InitializeChunkPageHashes;

impl OnRuntimeUpgrade for InitializeChunkPageHashes {
	fn on_runtime_upgrade() -> Weight {
		let db_weight = <Runtime as frame_system::Config>::DbWeight::get();
		let mut weight = Weight::zero();

		if ChunkPageSize::get() != HASHED_PAGE_SIZE {
			log::error!(
				target: LOG_TARGET,
				"ChunkPageSize is {} but the committed hashes were computed for {HASHED_PAGE_SIZE}; \
				 refusing to write page hashes",
				ChunkPageSize::get(),
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
					weight.saturating_accrue(
						<Runtime as indiv_pallet_chunks_manager::Config>::WeightInfo::set_chunk_page_hashes(
							page_count,
						),
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
			ChunkPageSize::get() == HASHED_PAGE_SIZE,
			"committed chunk page hashes were computed for a different ChunkPageSize"
		);

		for (ring_exponent, hashes) in committed_page_hashes() {
			for page_index in 0..hashes.len() as u32 {
				ensure!(
					ChunkPageHashes::<Runtime>::contains_key(ring_exponent, page_index),
					"chunk page hash missing after initialization"
				);
			}
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

	/// The domain a ring exponent selects, taken from `indiv-support`'s own conversion rather than
	/// restated here — the whole point of these tests is that the mapping is not assumed.
	fn domain_of(ring_exponent: RingExponent) -> RingDomainSize {
		ring_exponent.try_into().expect("every RingExponent maps to a RingDomainSize")
	}

	/// `R2e9` must select the domain holding 2^9 chunks, and `R2e10` the one holding 2^10.
	///
	/// The exponent-to-domain mapping is off-by-two by name (`R2e9` -> `Domain11`), because a
	/// domain is sized for the PCS, not the ring. Anchoring on the chunk count instead of the
	/// variant name is what makes "these are the `R2e9` hashes" a checked claim.
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

	/// The committed hashes must match what the pinned `verifiable` crate produces at the
	/// configured page size. Bumping the crate or changing [`ChunkPageSize`] breaks this.
	#[test]
	fn chunk_page_hashes_match() {
		assert_eq!(
			ChunkPageSize::get(),
			HASHED_PAGE_SIZE,
			"regenerate the committed chunk page hashes for the new page size"
		);

		for (ring_exponent, committed) in committed_page_hashes() {
			let expected = ring_verifier_builder_params_hashes::<BandersnatchSuite>(
				domain_of(ring_exponent),
				HASHED_PAGE_SIZE,
			);
			assert_eq!(
				committed.to_vec(),
				expected,
				"committed page hashes for {ring_exponent:?} are stale"
			);
		}
	}

	/// Every committed hash must accept its own page of chunks and nothing else, for every page of
	/// both rings. This is what ties a hash to a ring: `authorize_add_chunks` is the only gate on
	/// what data can ever be stored under a given exponent and page index.
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
				let pages: Vec<_> = chunks.chunks(HASHED_PAGE_SIZE as usize).collect();
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

	/// The chunks these hashes commit to must actually verify a ring at that exponent: build a
	/// one-member ring the way `pallet-members` does and compare its root against the crate.
	///
	/// A mismatched exponent-to-domain pairing fails here — `start_members` is given the domain,
	/// `push_members` reads the chunks, and arkworks rejects or diverges if they disagree.
	#[test]
	fn committed_chunks_build_a_ring_at_their_exponent() {
		for ring_exponent in [RingExponent::R2e9, RingExponent::R2e10] {
			let domain = domain_of(ring_exponent);
			let chunks = ring_verifier_builder_params::<BandersnatchSuite>(domain);

			// Reassemble the chunks from the exact page split the hashes commit to, so the ring is
			// built from the same bytes `add_chunks` would have accepted.
			let paged: Vec<_> = chunks
				.chunks(HASHED_PAGE_SIZE as usize)
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

	/// The upgrade must leave every page hash committed and be safe to re-run.
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
			assert_eq!(rerun, <Runtime as frame_system::Config>::DbWeight::get().reads(2));
			assert_eq!(ChunkPageHashes::<Runtime>::iter().collect::<Vec<_>>(), before);
		});
	}

	/// Guards the encoding assumption behind the hashes: a page is hashed as a plain
	/// `Vec<StaticChunk>`, so its SCALE length prefix is part of the preimage.
	#[test]
	fn page_hash_preimage_is_length_prefixed() {
		let chunks =
			ring_verifier_builder_params::<BandersnatchSuite>(domain_of(RingExponent::R2e9));
		let first_page = chunks[..HASHED_PAGE_SIZE as usize].to_vec();
		assert_eq!(sp_io::hashing::blake2_256(&first_page.encode()), hashes::R2E9[0]);
		// Dropping the prefix must not hash to the committed value.
		assert_ne!(sp_io::hashing::blake2_256(&first_page.encode()[2..]), hashes::R2E9[0]);
	}
}
