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

//! Decodable variant of the RuntimeMetadata.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]
#[cfg(all(
	any(feature = "decode", feature = "serde_full"),
	feature = "legacy",
	not(feature = "std")
))]
compile_error!("decode and serde_full features prior to v14 require std");

#[cfg(feature = "serde_full")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "decode")]
use codec::{Decode, Error, Input};

cfg_if::cfg_if! {
	if #[cfg(not(feature = "std"))] {
		extern crate alloc;
		use alloc::vec::Vec;
	}
}

use codec::{Encode, Output};

/// A type that decodes to a different type than it encodes.
#[cfg(feature = "legacy")]
pub mod decode_different;

/// Metadata v8
#[cfg(feature = "legacy")]
pub mod v8;

/// Metadata v9
#[cfg(feature = "legacy")]
pub mod v9;

/// Metadata v10
#[cfg(feature = "legacy")]
pub mod v10;

/// Metadata v11
#[cfg(feature = "legacy")]
pub mod v11;

/// Metadata v12
#[cfg(feature = "legacy")]
pub mod v12;

/// Metadata v13
#[cfg(feature = "legacy")]
pub mod v13;

/// Metadata v14
#[cfg(feature = "current")]
pub mod v14;

/// Metadata v15
#[cfg(feature = "current")]
pub mod v15;

/// Metadata prefix.
pub const META_RESERVED: u32 = 0x6174656d; // 'meta' warning for endianness.

/// Metadata prefixed by a u32 for reserved usage
#[derive(Eq, Encode, PartialEq, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
pub struct RuntimeMetadataPrefixed(pub u32, pub RuntimeMetadata);

impl From<RuntimeMetadataPrefixed> for Vec<u8> {
	fn from(value: RuntimeMetadataPrefixed) -> Self {
		value.encode()
	}
}

/// The metadata of a runtime.
/// The version ID encoded/decoded through
/// the enum nature of `RuntimeMetadata`.
#[derive(Eq, Encode, PartialEq, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize))]
pub enum RuntimeMetadata {
	/// Unused; enum filler.
	V0(RuntimeMetadataDeprecated),
	/// Version 1 for runtime metadata. No longer used.
	V1(RuntimeMetadataDeprecated),
	/// Version 2 for runtime metadata. No longer used.
	V2(RuntimeMetadataDeprecated),
	/// Version 3 for runtime metadata. No longer used.
	V3(RuntimeMetadataDeprecated),
	/// Version 4 for runtime metadata. No longer used.
	V4(RuntimeMetadataDeprecated),
	/// Version 5 for runtime metadata. No longer used.
	V5(RuntimeMetadataDeprecated),
	/// Version 6 for runtime metadata. No longer used.
	V6(RuntimeMetadataDeprecated),
	/// Version 7 for runtime metadata. No longer used.
	V7(RuntimeMetadataDeprecated),
	/// Version 8 for runtime metadata.
	#[cfg(feature = "legacy")]
	V8(v8::RuntimeMetadataV8),
	/// Version 8 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "legacy"))]
	V8(OpaqueMetadata),
	/// Version 9 for runtime metadata.
	#[cfg(feature = "legacy")]
	V9(v9::RuntimeMetadataV9),
	/// Version 9 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "legacy"))]
	V9(OpaqueMetadata),
	/// Version 10 for runtime metadata.
	#[cfg(feature = "legacy")]
	V10(v10::RuntimeMetadataV10),
	/// Version 10 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "legacy"))]
	V10(OpaqueMetadata),
	/// Version 11 for runtime metadata.
	#[cfg(feature = "legacy")]
	V11(v11::RuntimeMetadataV11),
	/// Version 11 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "legacy"))]
	V11(OpaqueMetadata),
	/// Version 12 for runtime metadata
	#[cfg(feature = "legacy")]
	V12(v12::RuntimeMetadataV12),
	/// Version 12 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "legacy"))]
	V12(OpaqueMetadata),
	/// Version 13 for runtime metadata.
	#[cfg(feature = "legacy")]
	V13(v13::RuntimeMetadataV13),
	/// Version 13 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "legacy"))]
	V13(OpaqueMetadata),
	/// Version 14 for runtime metadata.
	#[cfg(feature = "current")]
	V14(v14::RuntimeMetadataV14),
	/// Version 14 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "current"))]
	V14(OpaqueMetadata),
	/// Version 15 for runtime metadata.
	#[cfg(feature = "current")]
	V15(v15::RuntimeMetadataV15),
	/// Version 15 for runtime metadata, as raw encoded bytes.
	#[cfg(not(feature = "current"))]
	V15(OpaqueMetadata),
}

impl RuntimeMetadata {
	/// Get the version number of the metadata.
	pub fn version(&self) -> u32 {
		match self {
			RuntimeMetadata::V0(_) => 0,
			RuntimeMetadata::V1(_) => 1,
			RuntimeMetadata::V2(_) => 2,
			RuntimeMetadata::V3(_) => 3,
			RuntimeMetadata::V4(_) => 4,
			RuntimeMetadata::V5(_) => 5,
			RuntimeMetadata::V6(_) => 6,
			RuntimeMetadata::V7(_) => 7,
			RuntimeMetadata::V8(_) => 8,
			RuntimeMetadata::V9(_) => 9,
			RuntimeMetadata::V10(_) => 10,
			RuntimeMetadata::V11(_) => 11,
			RuntimeMetadata::V12(_) => 12,
			RuntimeMetadata::V13(_) => 13,
			RuntimeMetadata::V14(_) => 14,
			RuntimeMetadata::V15(_) => 15,
		}
	}
}

/// Stores the encoded `RuntimeMetadata` as raw bytes.
#[derive(Encode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "decode", derive(Decode))]
#[cfg_attr(feature = "serde_full", derive(Serialize, Deserialize))]
pub struct OpaqueMetadata(pub Vec<u8>);

/// Enum that should fail.
#[derive(Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde_full", derive(Serialize, Deserialize))]
pub enum RuntimeMetadataDeprecated {}

impl Encode for RuntimeMetadataDeprecated {
	fn encode_to<W: Output + ?Sized>(&self, _dest: &mut W) {}
}

impl codec::EncodeLike for RuntimeMetadataDeprecated {}

#[cfg(feature = "decode")]
impl Decode for RuntimeMetadataDeprecated {
	fn decode<I: Input>(_input: &mut I) -> Result<Self, Error> {
		Err("Decoding is not supported".into())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use std::fs;

	fn load_metadata(version: u32) -> Vec<u8> {
		fs::read(format!("./test_data/ksm_metadata_v{}.bin", version)).unwrap()
	}

	#[test]
	fn should_decode_metadatav9() {
		let meta: RuntimeMetadataPrefixed =
			Decode::decode(&mut load_metadata(9).as_slice()).unwrap();
		assert!(matches!(meta.1, RuntimeMetadata::V9(_)));
	}

	#[test]
	fn should_decode_metadatav10() {
		let meta: RuntimeMetadataPrefixed =
			Decode::decode(&mut load_metadata(10).as_slice()).unwrap();
		assert!(matches!(meta.1, RuntimeMetadata::V10(_)));
	}

	#[test]
	fn should_decode_metadatav11() {
		let meta: RuntimeMetadataPrefixed =
			Decode::decode(&mut load_metadata(11).as_slice()).unwrap();
		assert!(matches!(meta.1, RuntimeMetadata::V11(_)));
	}

	#[test]
	fn should_decode_metadatav12() {
		let meta: RuntimeMetadataPrefixed =
			Decode::decode(&mut load_metadata(12).as_slice()).unwrap();
		assert!(matches!(meta.1, RuntimeMetadata::V12(_)));
	}

	#[test]
	fn should_decode_metadatav13() {
		let meta: RuntimeMetadataPrefixed =
			Decode::decode(&mut load_metadata(13).as_slice()).unwrap();
		assert!(matches!(meta.1, RuntimeMetadata::V13(_)));
	}

	#[test]
	fn should_decode_metadatav14() {
		let meta: RuntimeMetadataPrefixed =
			Decode::decode(&mut load_metadata(14).as_slice()).unwrap();
		assert!(matches!(meta.1, RuntimeMetadata::V14(_)));
	}
}
