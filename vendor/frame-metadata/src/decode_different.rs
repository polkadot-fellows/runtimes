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

use codec::{Encode, Output};

cfg_if::cfg_if! {
	if #[cfg(feature = "std")] {
		use codec::{Decode, Error, Input};
		/// On `std` the `StringBuf` used by [`DecodeDifferent`] is just a `String`.
		pub type StringBuf = String;
	} else {
		extern crate alloc;
		use alloc::vec::Vec;

		/// On `no_std` we do not support `Decode` and thus `StringBuf` is just `&'static str`.
		/// So, if someone tries to decode this stuff on `no_std`, they will get a compilation error.
		pub type StringBuf = &'static str;
	}
}

/// A type that decodes to a different type than it encodes.
/// The user needs to make sure that both types use the same encoding.
///
/// For example a `&'static [ &'static str ]` can be decoded to a `Vec<String>`.
#[derive(Clone)]
pub enum DecodeDifferent<B, O>
where
	B: 'static,
	O: 'static,
{
	/// Encodable variant of the value (doesn't need to be decodeable).
	Encode(B),
	/// Encodable & decodeable variant of the value.
	Decoded(O),
}

impl<B, O> Encode for DecodeDifferent<B, O>
where
	B: Encode + 'static,
	O: Encode + 'static,
{
	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		match self {
			DecodeDifferent::Encode(b) => b.encode_to(dest),
			DecodeDifferent::Decoded(o) => o.encode_to(dest),
		}
	}
}

impl<B, O> codec::EncodeLike for DecodeDifferent<B, O>
where
	B: Encode + 'static,
	O: Encode + 'static,
{
}

#[cfg(feature = "std")]
impl<B, O> Decode for DecodeDifferent<B, O>
where
	B: 'static,
	O: Decode + 'static,
{
	fn decode<I: Input>(input: &mut I) -> Result<Self, Error> {
		<O>::decode(input).map(|val| DecodeDifferent::Decoded(val))
	}
}

impl<B, O> PartialEq for DecodeDifferent<B, O>
where
	B: Encode + Eq + PartialEq + 'static,
	O: Encode + Eq + PartialEq + 'static,
{
	fn eq(&self, other: &Self) -> bool {
		self.encode() == other.encode()
	}
}

impl<B, O> Eq for DecodeDifferent<B, O>
where
	B: Encode + Eq + PartialEq + 'static,
	O: Encode + Eq + PartialEq + 'static,
{
}

impl<B, O> core::fmt::Debug for DecodeDifferent<B, O>
where
	B: core::fmt::Debug + Eq + 'static,
	O: core::fmt::Debug + Eq + 'static,
{
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		match self {
			DecodeDifferent::Encode(b) => b.fmt(f),
			DecodeDifferent::Decoded(o) => o.fmt(f),
		}
	}
}

#[cfg(feature = "std")]
impl<B, O> serde::Serialize for DecodeDifferent<B, O>
where
	B: serde::Serialize + 'static,
	O: serde::Serialize + 'static,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			DecodeDifferent::Encode(b) => b.serialize(serializer),
			DecodeDifferent::Decoded(o) => o.serialize(serializer),
		}
	}
}

/// An array type that decodes as a `Vec`.
pub type DecodeDifferentArray<B, O = B> = DecodeDifferent<&'static [B], Vec<O>>;

/// A string type that decodes as a [`StringBuf`].
pub type DecodeDifferentStr = DecodeDifferent<&'static str, StringBuf>;

/// Newtype wrapper for support encoding functions (actual the result of the function).
#[derive(Clone, Eq)]
pub struct FnEncode<E>(pub fn() -> E)
where
	E: Encode + 'static;

impl<E: Encode> Encode for FnEncode<E> {
	fn encode_to<W: Output + ?Sized>(&self, dest: &mut W) {
		self.0().encode_to(dest);
	}
}

impl<E: Encode> codec::EncodeLike for FnEncode<E> {}

impl<E: Encode + PartialEq> PartialEq for FnEncode<E> {
	fn eq(&self, other: &Self) -> bool {
		self.0().eq(&other.0())
	}
}

impl<E: Encode + core::fmt::Debug> core::fmt::Debug for FnEncode<E> {
	fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
		self.0().fmt(f)
	}
}

#[cfg(feature = "std")]
impl<E: Encode + serde::Serialize> serde::Serialize for FnEncode<E> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0().serialize(serializer)
	}
}
