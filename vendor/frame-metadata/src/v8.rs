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

//! Metadata Version 8. Chains old enough to contain this metadata need a way to decode it.

#![allow(missing_docs)]

use crate::decode_different::*;
use codec::{Encode, Output};

cfg_if::cfg_if! {
	if #[cfg(feature = "std")] {
		use codec::Decode;
		use serde::Serialize;

		type StringBuf = String;
	} else {
		extern crate alloc;
		use alloc::vec::Vec;

		/// On `no_std` we do not support `Decode` and thus `StringBuf` is just `&'static str`.
		/// So, if someone tries to decode this stuff on `no_std`, they will get a compilation error.
		type StringBuf = &'static str;
	}
}

/// Curent prefix of metadata
pub const META_RESERVED: u32 = 0x6174656d; // 'meta' warn endianness

/// All the metadata about a function.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct FunctionMetadata {
	/// Function name.
	pub name: DecodeDifferentStr,
	/// A list of arguments this function takes.
	pub arguments: DecodeDifferentArray<FunctionArgumentMetadata>,
	/// Function documentation.
	pub documentation: DecodeDifferentArray<&'static str, StringBuf>,
}

/// All the metadata about a function argument.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct FunctionArgumentMetadata {
	/// Name of the variable for the argument.
	pub name: DecodeDifferentStr,
	/// Type of the parameter.
	pub ty: DecodeDifferentStr,
}

/// All the metadata about an outer event.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct OuterEventMetadata {
	/// Name of the event.
	pub name: DecodeDifferentStr,
	/// A list of event details.
	pub events: DecodeDifferentArray<
		(&'static str, FnEncode<&'static [EventMetadata]>),
		(StringBuf, Vec<EventMetadata>),
	>,
}

/// All the metadata about an event.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct EventMetadata {
	/// Name of the event.
	pub name: DecodeDifferentStr,
	/// Arguments of the event.
	pub arguments: DecodeDifferentArray<&'static str, StringBuf>,
	/// Documentation of the event.
	pub documentation: DecodeDifferentArray<&'static str, StringBuf>,
}

/// All the metadata about one storage entry.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct StorageEntryMetadata {
	/// Variable name of the storage entry.
	pub name: DecodeDifferentStr,
	/// A storage modifier of the storage entry (is it optional? does it have a default value?).
	pub modifier: StorageEntryModifier,
	/// Type of the value stored in the entry.
	pub ty: StorageEntryType,
	/// Default value (SCALE encoded).
	pub default: ByteGetter,
	/// Storage entry documentation.
	pub documentation: DecodeDifferentArray<&'static str, StringBuf>,
}

/// All the metadata about one module constant.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct ModuleConstantMetadata {
	/// Name of the module constant.
	pub name: DecodeDifferentStr,
	/// Type of the module constant.
	pub ty: DecodeDifferentStr,
	/// Value stored in the constant (SCALE encoded).
	pub value: ByteGetter,
	/// Documentation of the constant.
	pub documentation: DecodeDifferentArray<&'static str, StringBuf>,
}

/// All the metadata about a module error.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct ErrorMetadata {
	/// Name of the error.
	pub name: DecodeDifferentStr,
	/// Error variant documentation.
	pub documentation: DecodeDifferentArray<&'static str, StringBuf>,
}

/// All the metadata about errors in a module.
pub trait ModuleErrorMetadata {
	/// Returns error metadata.
	fn metadata() -> &'static [ErrorMetadata];
}

impl ModuleErrorMetadata for &'static str {
	fn metadata() -> &'static [ErrorMetadata] {
		&[]
	}
}

/// A technical trait to store lazy initiated vec value as static dyn pointer.
pub trait DefaultByte: Send + Sync {
	/// A default value (SCALE encoded).
	fn default_byte(&self) -> Vec<u8>;
}

/// Wrapper over dyn pointer for accessing a cached once byte value.
#[derive(Clone)]
pub struct DefaultByteGetter(pub &'static dyn DefaultByte);

/// Decode different for static lazy initiated byte value.
pub type ByteGetter = DecodeDifferent<DefaultByteGetter, Vec<u8>>;

impl Encode for DefaultByteGetter {
	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		self.0.default_byte().encode_to(dest)
	}
}

impl codec::EncodeLike for DefaultByteGetter {}

impl PartialEq<DefaultByteGetter> for DefaultByteGetter {
	fn eq(&self, other: &DefaultByteGetter) -> bool {
		let left = self.0.default_byte();
		let right = other.0.default_byte();
		left.eq(&right)
	}
}

impl Eq for DefaultByteGetter {}

#[cfg(feature = "std")]
impl serde::Serialize for DefaultByteGetter {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.default_byte().serialize(serializer)
	}
}

impl core::fmt::Debug for DefaultByteGetter {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		self.0.default_byte().fmt(f)
	}
}

/// Hasher used by storage maps
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub enum StorageHasher {
	/// 128-bit Blake2 hash.
	Blake2_128,
	/// 256-bit Blake2 hash.
	Blake2_256,
	/// 128-bit XX hash.
	Twox128,
	/// 256-bit XX hash.
	Twox256,
	/// 64-bit XX hashes concatentation.
	Twox64Concat,
}

/// A storage entry type.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub enum StorageEntryType {
	/// Plain storage entry (just the value).
	Plain(DecodeDifferentStr),
	/// A storage map.
	Map {
		/// Hasher type for the keys.
		hasher: StorageHasher,
		/// Key type.
		key: DecodeDifferentStr,
		/// Value type.
		value: DecodeDifferentStr,
		is_linked: bool,
	},
	/// Storage Double Map.
	DoubleMap {
		/// Hasher type for the keys.
		hasher: StorageHasher,
		/// First key type.
		key1: DecodeDifferentStr,
		/// Second key type.
		key2: DecodeDifferentStr,
		/// Value type.
		value: DecodeDifferentStr,
		/// Hasher for the second key.
		key2_hasher: StorageHasher,
	},
}

/// A storage entry modifier indicates how a storage entry is returned when fetched and what the value will be if the key is not present.
/// Specifically this refers to the "return type" when fetching a storage entry, and what the value will be if the key is not present.
///
/// `Optional` means you should expect an `Option<T>`, with `None` returned if the key is not present.
/// `Default` means you should expect a `T` with the default value of default if the key is not present.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub enum StorageEntryModifier {
	/// The storage entry returns an `Option<T>`, with `None` if the key is not present.
	Optional,
	/// The storage entry returns `T::Default` if the key is not present.
	Default,
}

/// All metadata of the storage.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct StorageMetadata {
	/// The common prefix used by all storage entries.
	pub prefix: DecodeDifferent<&'static str, StringBuf>,
	pub entries: DecodeDifferent<&'static [StorageEntryMetadata], Vec<StorageEntryMetadata>>,
}

/// The metadata of a runtime.
#[derive(Eq, Encode, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct RuntimeMetadataV8 {
	pub modules: DecodeDifferentArray<ModuleMetadata>,
}

/// The latest version of the metadata.
pub type RuntimeMetadataLastVersion = RuntimeMetadataV8;

/// All metadata about a runtime module.
#[derive(Clone, PartialEq, Eq, Encode, Debug)]
#[cfg_attr(feature = "std", derive(Decode, Serialize))]
pub struct ModuleMetadata {
	/// Module name.
	pub name: DecodeDifferentStr,
	pub storage: Option<DecodeDifferent<FnEncode<StorageMetadata>, StorageMetadata>>,
	pub calls: ODFnA<FunctionMetadata>,
	pub event: ODFnA<EventMetadata>,
	pub constants: DFnA<ModuleConstantMetadata>,
	pub errors: DFnA<ErrorMetadata>,
}

type ODFnA<T> = Option<DFnA<T>>;
type DFnA<T> = DecodeDifferent<FnEncode<&'static [T]>, Vec<T>>;
