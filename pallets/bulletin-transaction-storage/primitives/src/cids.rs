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

//! Utilities for working with CIDs (Content Identifiers).
//!
//! This module provides types and functions to compute CIDs for raw data or
//! DAG blocks using supported hashing algorithms and codecs.
//!
//! See [`CidData`].

use crate::ContentHash;
use alloc::vec::Vec;
use cid::{multihash::Multihash, CidGeneric};
use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};

const LOG_TARGET: &str = "runtime::transaction-storage::cids";

/// CIDv1 serialized bytes (codec + multihash(ContentHash)).
pub type Cid = Vec<u8>;

/// Type alias representing a CID codec (e.g., raw = 0x55, dag-pb = 0x70).
pub type CidCodec = u64;

/// CID codec for raw binary content.
pub const RAW_CODEC: CidCodec = 0x55;

/// Supported hashing algorithms for computing CIDs.
#[derive(
	Clone,
	Copy,
	PartialEq,
	Eq,
	Encode,
	Debug,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	MaxEncodedLen,
)]
#[non_exhaustive]
pub enum HashingAlgorithm {
	/// Blake2b-256 hash function.
	Blake2b256,
	/// SHA2-256 hash function.
	Sha2_256,
	/// Keccak-256 hash function.
	Keccak256,
}

impl HashingAlgorithm {
	/// Compute the hash of the given data using the selected algorithm.
	pub fn hash(&self, data: &[u8]) -> ContentHash {
		match self {
			HashingAlgorithm::Blake2b256 => sp_io::hashing::blake2_256(data),
			HashingAlgorithm::Sha2_256 => sp_io::hashing::sha2_256(data),
			HashingAlgorithm::Keccak256 => sp_io::hashing::keccak_256(data),
		}
	}

	/// Return the multihash code corresponding to this hashing algorithm.
	///
	/// These codes follow the [multihash table](https://github.com/multiformats/multicodec/blob/master/table.csv):
	/// - Blake2b-256 = 0xb220
	/// - SHA2-256 = 0x12
	/// - Keccak-256 = 0x1b
	pub fn multihash_code(&self) -> u64 {
		match self {
			HashingAlgorithm::Blake2b256 => 0xb220,
			HashingAlgorithm::Sha2_256 => 0x12,
			HashingAlgorithm::Keccak256 => 0x1b,
		}
	}
}

/// Configuration for generating a CID.
#[derive(
	Clone,
	PartialEq,
	Eq,
	Encode,
	Debug,
	Decode,
	DecodeWithMemTracking,
	scale_info::TypeInfo,
	MaxEncodedLen,
)]
pub struct CidConfig {
	/// CID codec (e.g., raw = 0x55, dag-pb = 0x70).
	pub codec: CidCodec,
	/// Hashing algorithm to use for computing the content hash.
	pub hashing: HashingAlgorithm,
}

/// Error returned when CID creation fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CidError;

/// Representation of a generated CID containing only the component parts.
///
/// Use `CidGeneric::<32>::try_from(cid_data)` to build the actual [`CidGeneric`], or
/// [`CidData::to_bytes()`] for the serialized CIDv1 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CidData {
	/// 32-byte content hash of the input data.
	///
	/// Note: This is used for indexing transactions and retrieving
	/// `self.client.indexed_transaction(hash)`. This is equal to `cid.hash().digest()`.
	pub content_hash: ContentHash,
	/// Hashing algorithm used.
	pub hashing: HashingAlgorithm,
	/// Codec used for the CIDv1.
	pub codec: CidCodec,
}

impl TryFrom<CidData> for CidGeneric<32> {
	type Error = CidError;

	fn try_from(cid_data: CidData) -> Result<Self, Self::Error> {
		let mh = Multihash::<32>::wrap(cid_data.hashing.multihash_code(), &cid_data.content_hash)
			.map_err(|e| {
			tracing::warn!(
				target: LOG_TARGET,
				"Failed to create CID for content_hash: {:?}, hashing: {:?}, codec: {:?}, error: {:?}",
				cid_data.content_hash, cid_data.hashing, cid_data.codec, e
			);
			CidError
		})?;
		Ok(CidGeneric::<32>::new_v1(cid_data.codec, mh))
	}
}

impl CidData {
	/// Serialize the CID to bytes (CIDv1 format).
	///
	/// Returns `None` if the CID cannot be created.
	pub fn to_bytes(&self) -> Option<Cid> {
		CidGeneric::<32>::try_from(*self).ok().map(|cid| cid.to_bytes())
	}
}

/// Compute a CIDv1 for the given data with the specified configuration.
///
/// # Errors
/// Returns `Err(CidError)` if multihash wrapping fails.
pub fn calculate_cid(data: &[u8], config: CidConfig) -> Result<CidData, CidError> {
	let (hashing, codec) = (config.hashing, config.codec);

	// Hash the data
	let content_hash = hashing.hash(data);
	let cid_data = CidData { content_hash, hashing, codec };

	// Validate CID can be created
	let _: CidGeneric<32> = cid_data.try_into()?;

	Ok(cid_data)
}

#[cfg(test)]
mod tests {
	use super::{calculate_cid, CidConfig, HashingAlgorithm};
	use cid::{
		multibase::{encode as to_base32, Base},
		CidGeneric,
	};
	use core::str::FromStr;

	#[test]
	fn test_cid_raw_blake2b_256_roundtrip_works() {
		// Prepare data.
		let data = "Hello, Bulletin with PAPI - Fri Nov 21 2025 11:09:18 GMT+0000";
		let expected_content_hash = sp_io::hashing::blake2_256(data.as_bytes());

		// Expected raw CID calculated for the same data with `examples/common.js`.
		let expected_cid_base32 = "bafk2bzacedvk4eijklisgdjijnxky24pmkg7jgk5vsct4mwndj3nmx7plzz7m";
		let expected_cid = CidGeneric::<32>::from_str(expected_cid_base32).expect("valid_cid");
		assert_eq!(expected_cid.codec(), 0x55);
		assert_eq!(expected_cid.hash().code(), 0xb220);
		assert_eq!(expected_cid.hash().size(), 0x20);
		assert_eq!(expected_cid.hash().digest(), expected_content_hash);

		// Calculate CIDv1 with default raw codec and blake2b-256.
		let cid_raw = calculate_cid(
			data.as_ref(),
			CidConfig { codec: 0x55, hashing: HashingAlgorithm::Blake2b256 },
		)
		.expect("valid_cid");
		let cid_blake2b_256_raw = calculate_cid(
			data.as_ref(),
			CidConfig { codec: 0x55, hashing: HashingAlgorithm::Blake2b256 },
		)
		.expect("valid_cid");
		assert_eq!(cid_raw.to_bytes().expect("valid cid"), expected_cid.to_bytes());
		assert_eq!(
			to_base32(Base::Base32Lower, cid_raw.to_bytes().expect("valid cid")),
			expected_cid_base32
		);
		assert_eq!(cid_raw.codec, expected_cid.codec());
		assert_eq!(cid_raw.hashing.multihash_code(), expected_cid.hash().code());
		assert_eq!(cid_raw.content_hash, expected_cid.hash().digest());
		assert_eq!(cid_raw, cid_blake2b_256_raw);
	}

	/// Return the HashingAlgorithm corresponding to a multihash code.
	pub fn from_multihash_code(code: u64) -> HashingAlgorithm {
		match code {
			0xb220 => HashingAlgorithm::Blake2b256,
			0x12 => HashingAlgorithm::Sha2_256,
			0x1b => HashingAlgorithm::Keccak256,
			code => panic!("{code} is not supported"),
		}
	}

	#[test]
	fn test_cid_various_codecs_and_hashes() {
		let data = "Hello, Bulletin with PAPI - Fri Nov 21 2025 11:09:18 GMT+0000";

		// Expected results from `examples/common.js`.
		let expected_cids = vec![
			// raw + blake2b_256
			("bafk2bzacedvk4eijklisgdjijnxky24pmkg7jgk5vsct4mwndj3nmx7plzz7m", 0x55, 0xb220),
			// DAG-PB + blake2b_256
			("bafykbzacedvk4eijklisgdjijnxky24pmkg7jgk5vsct4mwndj3nmx7plzz7m", 0x70, 0xb220),
			// Raw + sha2_256
			("bafkreig5pw2of63kmkldboh6utfovo3o3czig4yj7eb2ragxwca4c4jlke", 0x55, 0x12),
			// DAG-PB + sha2_256
			("bafybeig5pw2of63kmkldboh6utfovo3o3czig4yj7eb2ragxwca4c4jlke", 0x70, 0x12),
			// Raw + keccak_256
			("bafkrwifr4p73tsatchlyp3hivjee4prqqpcqayikzen46bqldwmt5mzd6e", 0x55, 0x1b),
			// DAG-PB + keccak_256
			("bafybwifr4p73tsatchlyp3hivjee4prqqpcqayikzen46bqldwmt5mzd6e", 0x70, 0x1b),
		];

		for (expected_cid_str, codec, mh_code) in expected_cids {
			let cid = CidGeneric::<32>::from_str(expected_cid_str).expect("valid CID");
			// Check codec and multihash code
			assert_eq!(cid.codec(), codec);
			assert_eq!(cid.hash().code(), mh_code);

			// Test `calculate_cid`
			let calculated = calculate_cid(
				data.as_ref(),
				CidConfig { codec, hashing: from_multihash_code(mh_code) },
			)
			.expect("calculate_cid succeeded");

			assert_eq!(
				to_base32(Base::Base32Lower, calculated.to_bytes().expect("valid cid")),
				expected_cid_str
			);
			assert_eq!(calculated.codec, codec);
			assert_eq!(calculated.hashing.multihash_code(), mh_code);
			assert_eq!(calculated.content_hash, cid.hash().digest());
		}
	}
}
