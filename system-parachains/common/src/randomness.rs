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
// along with Polkadot. If not, see <http://www.gnu.org/licenses/>.

use core::marker::PhantomData;
use cumulus_primitives_core::relay_chain;
use frame_support::traits::Randomness;
use sp_runtime::traits::{BlakeTwo256, Hash};
use sp_state_machine::{Backend, TrieBackendBuilder};

pub const LOG_TARGET: &str = "runtime::randomness";

/// VRF output length for per-slot randomness.
pub const VRF_RANDOMNESS_LENGTH: usize = 32;

/// Provides randomness from the Relay Chain VRF from one epoch ago, but does not include the block
/// number indicating when this randomness was generated or became observable to chain observers.
///
/// WARNING: This implementation does not return the block number associated with the randomness,
/// because this information is not available in the validation data.
pub struct RelayChainOneEpochAgoWithoutBlockNumber<T, BlockNumber>(PhantomData<(T, BlockNumber)>);

impl<T, BlockNumber> Randomness<T::Hash, BlockNumber>
	for RelayChainOneEpochAgoWithoutBlockNumber<T, BlockNumber>
where
	T: cumulus_pallet_parachain_system::Config,
	BlockNumber: From<u32>,
{
	fn random(subject: &[u8]) -> (T::Hash, BlockNumber) {
		// Defensive fallback used if the `well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` key
		// is missing or absent from the validation data. This situation is unexpected,
		// as the key should always be present.
		let defensive_fallback = || (T::Hashing::hash(subject), 0.into());

		let Some(relay_state_proof) = cumulus_pallet_parachain_system::RelayStateProof::<T>::get()
		else {
			log::error!(
				target: LOG_TARGET,
				"No relay state proof in cumulus_pallet_parachain_system; cannot fetch randomness"
			);
			return defensive_fallback();
		};

		let relay_parent_storage_root = if let Some(validation_data) =
			cumulus_pallet_parachain_system::ValidationData::<T>::get()
		{
			validation_data.relay_parent_storage_root
		} else {
			log::error!(
				target: LOG_TARGET,
				"No validation data in cumulus_pallet_parachain_system; cannot fetch randomness"
			);
			return defensive_fallback();
		};

		let db = relay_state_proof.into_memory_db::<BlakeTwo256>();
		let trie_backend = TrieBackendBuilder::new(db, relay_parent_storage_root).build();

		let Ok(Some(random)) = trie_backend
			.storage(relay_chain::well_known_keys::ONE_EPOCH_AGO_RANDOMNESS)
			.inspect_err(|e| {
				log::error!(
					target: LOG_TARGET,
					"Failed to lookup `well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` from trie: {e}"
				);
			})
		else {
			log::error!(
				target: LOG_TARGET,
				"`well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` is none; cannot fetch randomness"
			);
			return defensive_fallback();
		};

		let mut subject = subject.to_vec();
		subject.reserve(VRF_RANDOMNESS_LENGTH);
		subject.extend_from_slice(&random);

		(T::Hashing::hash(&subject[..]), 0.into())
	}
}
