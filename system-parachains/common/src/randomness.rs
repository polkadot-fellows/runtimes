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
use cumulus_pallet_parachain_system::{RelaychainDataProvider, RelaychainStateProvider};
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
/// WARNING: A malicious collator can omit the `well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` key and
/// value in the validation data, causing this implementation to fall back to randomness from the
/// current relay chain state root.
///
/// WARNING: This implementation does not return the block number associated with the randomness,
/// because this information is not available in the validation data.
pub struct RelayChainOneEpochAgoWithoutBlockNumberWarningUnsafe<T, BlockNumber>(
	PhantomData<(T, BlockNumber)>,
);

impl<T, BlockNumber> Randomness<T::Hash, BlockNumber>
	for RelayChainOneEpochAgoWithoutBlockNumberWarningUnsafe<T, BlockNumber>
where
	T: cumulus_pallet_parachain_system::Config,
	BlockNumber: From<u32>,
{
	fn random(subject: &[u8]) -> (T::Hash, BlockNumber) {
		// Defensive fallback used if the `well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` key
		// is missing or absent from the validation data. This situation is expected when the
		// collator is malicious.
		let defensive_fallback = || {
			let rc_state = RelaychainDataProvider::<T>::current_relay_chain_state();
			let mut subject = subject.to_vec();
			subject.extend_from_slice(&rc_state.state_root.0);

			(T::Hashing::hash(&subject[..]), 0.into())
		};

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
					"Failed to lookup `well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` from the \
					validation data. The node may have maliciously omitted it. Error: {e}"
				);
			})
		else {
			log::error!(
				target: LOG_TARGET,
				"Value at `well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` is none; cannot fetch \
				randomness"
			);
			return defensive_fallback();
		};

		if random.len() != VRF_RANDOMNESS_LENGTH {
			log::error!(
				target: LOG_TARGET,
				"value at `well_known_keys::ONE_EPOCH_AGO_RANDOMNESS` has invalid length {}; \
				expected {VRF_RANDOMNESS_LENGTH}; cannot fetch randomness",
				random.len(),
			);
			return defensive_fallback();
		}

		let mut subject = subject.to_vec();
		subject.reserve(VRF_RANDOMNESS_LENGTH);
		subject.extend_from_slice(&random);

		(T::Hashing::hash(&subject[..]), 0.into())
	}
}
