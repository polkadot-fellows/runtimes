#![allow(clippy::tabs_in_doc_comments)]
#![deny(missing_docs)]
#![no_std]

//! A collection of array/bytes/hex utilities.
//!
//! Completely optimized for blockchain development.
//! Especially the Substrate.

extern crate alloc;

#[cfg(test)] mod test;

// core
use core::{convert::TryInto, result::Result as CoreResult, str};
// alloc
use alloc::{format, string::String, vec::Vec};
// crates.io
#[cfg(feature = "serde")] use serde::{de::Error as DeError, Deserialize, Deserializer};
// use thiserror::Error as ThisError;

/// The main result of array-bytes.
pub type Result<T> = CoreResult<T, Error>;

/// Try to convert the given hex to a specific type.
///
/// # Examples
/// ```
/// use array_bytes::TryFromHex;
///
/// assert_eq!(u128::try_from_hex("0x1a2b3c4d5e6f"), Ok(28772997619311));
/// ```
pub trait TryFromHex
where
	Self: Sized,
{
	/// Try to convert [`Self`] from hex.
	fn try_from_hex<H>(hex: H) -> Result<Self>
	where
		H: AsRef<[u8]>;
}
macro_rules! impl_num_try_from_hex {
	($($t:ty,)+) => {
		$(impl TryFromHex for $t {
			fn try_from_hex<H>(hex: H) -> Result<Self>
			where
				H: AsRef<[u8]>,
			{
				let hex = strip_0x(hex.as_ref());
				let hex = str::from_utf8(hex).map_err(Error::Utf8Error)?;

				Self::from_str_radix(hex, 16).map_err(Error::ParseIntError)
			}
		})+
	};
}
impl_num_try_from_hex! {
	isize,
	i8,
	i16,
	i32,
	i64,
	i128,
	usize,
	u8,
	u16,
	u32,
	u64,
	u128,
}
macro_rules! impl_array_try_from_hex {
	($($t:ty,)+) => {
		$(impl TryFromHex for $t {
			fn try_from_hex<H>(hex: H) -> Result<Self>
			where
				H: AsRef<[u8]>,
			{
				hex2array(hex)
			}
		})+
	};
}
impl_array_try_from_hex! {
	[u8; 1],
	[u8; 2],
	[u8; 3],
	[u8; 4],
	[u8; 5],
	[u8; 6],
	[u8; 7],
	[u8; 8],
	[u8; 9],
	[u8; 10],
	[u8; 11],
	[u8; 12],
	[u8; 13],
	[u8; 14],
	[u8; 15],
	[u8; 16],
	[u8; 17],
	[u8; 18],
	[u8; 19],
	[u8; 20],
	[u8; 21],
	[u8; 22],
	[u8; 23],
	[u8; 24],
	[u8; 25],
	[u8; 26],
	[u8; 27],
	[u8; 28],
	[u8; 29],
	[u8; 30],
	[u8; 31],
	[u8; 32],
	[u8; 33],
	[u8; 34],
	[u8; 35],
	[u8; 36],
	[u8; 37],
	[u8; 38],
	[u8; 39],
	[u8; 40],
	[u8; 41],
	[u8; 42],
	[u8; 43],
	[u8; 44],
	[u8; 45],
	[u8; 46],
	[u8; 47],
	[u8; 48],
	[u8; 49],
	[u8; 50],
	[u8; 51],
	[u8; 52],
	[u8; 53],
	[u8; 54],
	[u8; 55],
	[u8; 56],
	[u8; 57],
	[u8; 58],
	[u8; 59],
	[u8; 60],
	[u8; 61],
	[u8; 62],
	[u8; 63],
	[u8; 64],
	[u8; 128],
	[u8; 256],
	[u8; 512],
}
impl TryFromHex for Vec<u8> {
	fn try_from_hex<H>(hex: H) -> Result<Self>
	where
		H: AsRef<[u8]>,
	{
		hex2bytes(hex)
	}
}

/// Convert the given type to hex.
///
/// # Examples
/// ```
/// use array_bytes::Hex;
///
/// assert_eq!(28772997619311_u128.hex("0x"), "0x1a2b3c4d5e6f");
/// ```
pub trait Hex {
	/// Convert [`Self`] to hex with the given prefix.
	fn hex(self, prefix: &str) -> String;
}
macro_rules! impl_num_hex {
	($($t:ty,)+) => {
		$(
			impl Hex for $t {
				fn hex(self, prefix: &str) -> String {
					format!("{prefix}{self:x}")
				}
			}
			impl Hex for &$t {
				fn hex(self, prefix: &str) -> String {
					format!("{prefix}{self:x}")
				}
			}
		)+
	};
}
impl_num_hex! {
	isize,
	i8,
	i16,
	i32,
	i64,
	i128,
	usize,
	u8,
	u16,
	u32,
	u64,
	u128,
}
macro_rules! impl_array_hex {
	($($t:ty,)+) => {
		$(
			impl Hex for $t {
				fn hex(self, prefix: &str) -> String {
					bytes2hex(prefix, self)
				}
			}
			impl Hex for &$t {
				fn hex(self, prefix: &str) -> String {
					bytes2hex(prefix, self)
				}
			}
		)+
	};
}
impl_array_hex! {
	Vec<u8>,
	[u8; 1],
	[u8; 2],
	[u8; 3],
	[u8; 4],
	[u8; 5],
	[u8; 6],
	[u8; 7],
	[u8; 8],
	[u8; 9],
	[u8; 10],
	[u8; 11],
	[u8; 12],
	[u8; 13],
	[u8; 14],
	[u8; 15],
	[u8; 16],
	[u8; 17],
	[u8; 18],
	[u8; 19],
	[u8; 20],
	[u8; 21],
	[u8; 22],
	[u8; 23],
	[u8; 24],
	[u8; 25],
	[u8; 26],
	[u8; 27],
	[u8; 28],
	[u8; 29],
	[u8; 30],
	[u8; 31],
	[u8; 32],
	[u8; 33],
	[u8; 34],
	[u8; 35],
	[u8; 36],
	[u8; 37],
	[u8; 38],
	[u8; 39],
	[u8; 40],
	[u8; 41],
	[u8; 42],
	[u8; 43],
	[u8; 44],
	[u8; 45],
	[u8; 46],
	[u8; 47],
	[u8; 48],
	[u8; 49],
	[u8; 50],
	[u8; 51],
	[u8; 52],
	[u8; 53],
	[u8; 54],
	[u8; 55],
	[u8; 56],
	[u8; 57],
	[u8; 58],
	[u8; 59],
	[u8; 60],
	[u8; 61],
	[u8; 62],
	[u8; 63],
	[u8; 64],
	[u8; 128],
	[u8; 256],
	[u8; 512],
}
impl Hex for &[u8] {
	fn hex(self, prefix: &str) -> String {
		bytes2hex(prefix, self)
	}
}

/// The main error of array-bytes.
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
	/// The length must not be odd.
	InvalidLength,
	/// Found the invalid character at `index`.
	InvalidCharacter {
		/// The invalid character.
		character: char,
		/// The invalid character's index.
		index: usize,
	},
	/// The data can not fit the array/slice length well.
	MismatchedLength {
		/// Expected length.
		expect: usize,
	},
	/// Failed to parse the hex number from hex string.
	Utf8Error(core::str::Utf8Error),
	/// Failed to parse the hex number from hex string.
	ParseIntError(core::num::ParseIntError),
}

/// `&[T]` to `[T; N]`.
///
/// # Examples
/// ```
/// assert_eq!(array_bytes::slice2array::<8, _>(&[0; 8]), Ok([0; 8]));
/// ```
pub fn slice2array<const N: usize, T>(slice: &[T]) -> Result<[T; N]>
where
	T: Copy,
{
	slice.try_into().map_err(|_| Error::MismatchedLength { expect: N })
}

/// Just like [`slice2array`] but without the checking.
///
/// # Examples
/// ```
/// assert_eq!(array_bytes::slice2array_unchecked::<8, _>(&[0; 8]), [0; 8]);
/// ```
pub fn slice2array_unchecked<const N: usize, T>(slice: &[T]) -> [T; N]
where
	T: Copy,
{
	slice2array(slice).unwrap()
}

/// Convert `&[T]` to a type directly.
///
/// # Examples
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF([u8; 17]);
/// impl From<[u8; 17]> for LJF {
/// 	fn from(array: [u8; 17]) -> Self {
/// 		Self(array)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::slice_n_into::<17, u8, LJF>(b"Love Jane Forever"),
/// 	Ok(LJF(*b"Love Jane Forever"))
/// );
/// ```
pub fn slice_n_into<const N: usize, T, V>(slice: &[T]) -> Result<V>
where
	T: Copy,
	V: From<[T; N]>,
{
	Ok(slice2array(slice)?.into())
}

/// Just like [`slice_n_into`] but without the checking.
///
/// # Examples
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF([u8; 17]);
/// impl From<[u8; 17]> for LJF {
/// 	fn from(array: [u8; 17]) -> Self {
/// 		Self(array)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::slice_n_into_unchecked::<17, u8, LJF>(b"Love Jane Forever"),
/// 	LJF(*b"Love Jane Forever")
/// );
/// ```
pub fn slice_n_into_unchecked<const N: usize, T, V>(slice: &[T]) -> V
where
	T: Copy,
	V: From<[T; N]>,
{
	slice2array_unchecked(slice).into()
}

/// [`Vec<T>`] to `[T; N]`.
///
/// # Examples
/// ```
/// assert_eq!(array_bytes::vec2array::<8, _>(vec![0; 8]), Ok([0; 8]));
/// ```
pub fn vec2array<const N: usize, T>(vec: Vec<T>) -> Result<[T; N]> {
	vec.try_into().map_err(|_| Error::MismatchedLength { expect: N })
}

/// Just like [`vec2array`] but without the checking.
///
/// # Examples
/// ```
/// assert_eq!(array_bytes::vec2array_unchecked::<8, _>(vec![0; 8]), [0; 8]);
/// ```
pub fn vec2array_unchecked<const N: usize, T>(vec: Vec<T>) -> [T; N] {
	vec2array(vec).unwrap()
}

/// Convert [`Vec<T>`] to a type directly.
///
/// # Examples
///
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF([u8; 17]);
/// impl From<[u8; 17]> for LJF {
/// 	fn from(array: [u8; 17]) -> Self {
/// 		Self(array)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::vec_n_into::<17, u8, LJF>(b"Love Jane Forever".to_vec()),
/// 	Ok(LJF(*b"Love Jane Forever"))
/// );
/// ```
pub fn vec_n_into<const N: usize, T, V>(vec: Vec<T>) -> Result<V>
where
	V: From<[T; N]>,
{
	Ok(vec2array(vec)?.into())
}

/// Just like [`vec_n_into`] but without the checking.
///
/// # Examples
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF([u8; 17]);
/// impl From<[u8; 17]> for LJF {
/// 	fn from(array: [u8; 17]) -> Self {
/// 		Self(array)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::vec_n_into_unchecked::<17, u8, LJF>(b"Love Jane Forever".to_vec()),
/// 	LJF(*b"Love Jane Forever")
/// );
/// ```
pub fn vec_n_into_unchecked<const N: usize, T, V>(vec: Vec<T>) -> V
where
	V: From<[T; N]>,
{
	vec2array_unchecked(vec).into()
}

/// Convert hex bytes to hex string.
///
/// This is useful when you are interacting with the IO.
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::hex_bytes2hex_str(b"0x4c6f7665204a616e6520466f7265766572"),
/// 	Ok("0x4c6f7665204a616e6520466f7265766572"),
/// );
/// ```
pub fn hex_bytes2hex_str(bytes: &[u8]) -> Result<&str> {
	for (i, byte) in bytes.iter().enumerate().skip(if bytes.starts_with(b"0x") { 2 } else { 0 }) {
		if !is_hex_ascii(byte) {
			Err(Error::InvalidCharacter { character: *byte as _, index: i })?;
		}
	}

	Ok(
		// Validated in previous step, never fails here; qed.
		unsafe { str::from_utf8_unchecked(bytes) },
	)
}

/// Just like [`hex_bytes2hex_str`] but without the checking.
///
/// # Safety
/// See the [`str::from_utf8_unchecked`].
///
/// # Examples
/// ```
/// unsafe {
/// 	assert_eq!(
/// 		array_bytes::hex_bytes2hex_str_unchecked(b"0x4c6f7665204a616e6520466f7265766572"),
/// 		"0x4c6f7665204a616e6520466f7265766572",
/// 	);
/// }
/// ```
pub unsafe fn hex_bytes2hex_str_unchecked(bytes: &[u8]) -> &str {
	str::from_utf8_unchecked(bytes)
}

/// `AsRef<[u8]>` to [`String`].
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::bytes2hex("0x", b"Love Jane Forever"),
/// 	String::from("0x4c6f7665204a616e6520466f7265766572")
/// );
/// ```
pub fn bytes2hex<B>(prefix: &str, bytes: B) -> String
where
	B: AsRef<[u8]>,
{
	let bytes = bytes.as_ref();
	let mut hex = String::with_capacity(prefix.len() + bytes.len() * 2);

	prefix.chars().for_each(|byte| hex.push(byte));
	bytes.iter().for_each(|byte| {
		hex.push(char::from_digit((byte >> 4) as _, 16).unwrap());
		hex.push(char::from_digit((byte & 0xf) as _, 16).unwrap());
	});

	hex
}

/// Just like [`hex2bytes`] but to a fixed length array.
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::hex2array("0x4c6f7665204a616e6520466f7265766572"),
/// 	Ok(*b"Love Jane Forever")
/// );
/// ```
pub fn hex2array<H, const N: usize>(hex: H) -> Result<[u8; N]>
where
	H: AsRef<[u8]>,
{
	vec2array(hex2bytes(hex.as_ref())?)
}

/// Just like [`hex2array`] but without the checking.
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::hex2array_unchecked("0x4c6f7665204a616e6520466f7265766572"),
/// 	*b"Love Jane Forever"
/// );
/// ```
pub fn hex2array_unchecked<H, const N: usize>(hex: H) -> [u8; N]
where
	H: AsRef<[u8]>,
{
	hex2bytes_unchecked(hex).try_into().unwrap()
}

/// `AsRef<[u8]>` to [`Vec<u8>`].
///
/// Return error if:
/// - length is odd
/// - encounter invalid hex ascii
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::hex2bytes("0x4c6f7665204a616e6520466f7265766572"),
/// 	Ok(b"Love Jane Forever".to_vec())
/// );
/// ```
pub fn hex2bytes<H>(hex: H) -> Result<Vec<u8>>
where
	H: AsRef<[u8]>,
{
	let hex = strip_0x(hex.as_ref());

	if hex.len() % 2 != 0 {
		Err(Error::InvalidLength)?;
	}

	let mut bytes = Vec::new();

	for i in (0..hex.len()).step_by(2) {
		bytes.push(hex2byte((&hex[i], i), (&hex[i + 1], i + 1))?);
	}

	Ok(bytes)
}

/// Just like [`hex2bytes`] but without checking.
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::hex2bytes_unchecked("0x4c6f7665204a616e6520466f7265766572"),
/// 	*b"Love Jane Forever"
/// );
/// ```
pub fn hex2bytes_unchecked<H>(hex: H) -> Vec<u8>
where
	H: AsRef<[u8]>,
{
	let hex = strip_0x(hex.as_ref());

	(0..hex.len()).step_by(2).map(|i| hex2byte_unchecked(&hex[i], &hex[i + 1])).collect()
}

/// `AsRef<[u8]>` to `&[u8]`.
///
/// This function will modify the given slice's source and return the revised result.
///
/// Return error if:
/// - length is odd
/// - encounter invalid hex ascii
/// - mismatched slice size
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::hex2bytes("0x4c6f7665204a616e6520466f7265766572"),
/// 	Ok(b"Love Jane Forever".to_vec())
/// );
/// ```
pub fn hex2slice<H>(hex: H, slice: &mut [u8]) -> Result<&[u8]>
where
	H: AsRef<[u8]>,
{
	let hex = strip_0x(hex.as_ref());

	if hex.len() % 2 != 0 {
		Err(Error::InvalidLength)?;
	}

	let expected_len = hex.len() >> 1;

	if expected_len != slice.len() {
		Err(Error::MismatchedLength { expect: expected_len })?;
	}

	for (byte, i) in slice.iter_mut().zip((0..hex.len()).step_by(2)) {
		*byte = hex2byte((&hex[i], i), (&hex[i + 1], i + 1))?;
	}

	Ok(slice)
}

/// Just like [`hex2slice`] but without checking.
///
/// # Examples
/// ```
/// assert_eq!(
/// 	array_bytes::hex2bytes("0x4c6f7665204a616e6520466f7265766572"),
/// 	Ok(b"Love Jane Forever".to_vec())
/// );
/// ```
pub fn hex2slice_unchecked<H>(hex: H, slice: &mut [u8]) -> &[u8]
where
	H: AsRef<[u8]>,
{
	let hex = strip_0x(hex.as_ref());

	slice
		.iter_mut()
		.zip((0..hex.len()).step_by(2))
		.for_each(|(byte, i)| *byte = hex2byte_unchecked(&hex[i], &hex[i + 1]));

	slice
}

/// Try to convert `AsRef<[u8]>` to `T` directly, where `T: From<Vec<u8>>`.
///
/// # Examples
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF(Vec<u8>);
/// impl From<Vec<u8>> for LJF {
/// 	fn from(vec: Vec<u8>) -> Self {
/// 		Self(vec)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::hex_into::<_, LJF>("0x4c6f7665204a616e6520466f7265766572"),
/// 	Ok(LJF(b"Love Jane Forever".to_vec()))
/// );
/// ```
pub fn hex_into<H, T>(hex: H) -> Result<T>
where
	H: AsRef<[u8]>,
	T: From<Vec<u8>>,
{
	Ok(hex2bytes(hex.as_ref())?.into())
}

/// Just like [`hex_into`] but without the checking.
///
/// # Examples
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF(Vec<u8>);
/// impl From<Vec<u8>> for LJF {
/// 	fn from(vec: Vec<u8>) -> Self {
/// 		Self(vec)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::hex_into_unchecked::<_, LJF>("0x4c6f7665204a616e6520466f7265766572"),
/// 	LJF(b"Love Jane Forever".to_vec())
/// );
/// ```
pub fn hex_into_unchecked<H, T>(hex: H) -> T
where
	H: AsRef<[u8]>,
	T: From<Vec<u8>>,
{
	hex2bytes_unchecked(hex).into()
}

/// Try to convert `AsRef<[u8]>` to `T` directly, where `T: From<[u8; N]>`.
///
/// # Examples
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF([u8; 17]);
/// impl From<[u8; 17]> for LJF {
/// 	fn from(array: [u8; 17]) -> Self {
/// 		Self(array)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::hex_n_into::<_, LJF, 17>("0x4c6f7665204a616e6520466f7265766572"),
/// 	Ok(LJF(*b"Love Jane Forever"))
/// );
/// ```
pub fn hex_n_into<H, T, const N: usize>(hex: H) -> Result<T>
where
	H: AsRef<[u8]>,
	T: From<[u8; N]>,
{
	Ok(hex2array(hex)?.into())
}

/// Just like [`hex_n_into`] but without the checking.
///
/// # Examples
/// ```
/// #[derive(Debug, PartialEq)]
/// struct LJF([u8; 17]);
/// impl From<[u8; 17]> for LJF {
/// 	fn from(array: [u8; 17]) -> Self {
/// 		Self(array)
/// 	}
/// }
///
/// assert_eq!(
/// 	array_bytes::hex_n_into_unchecked::<_, LJF, 17>("0x4c6f7665204a616e6520466f7265766572"),
/// 	LJF(*b"Love Jane Forever")
/// );
/// ```
pub fn hex_n_into_unchecked<H, T, const N: usize>(hex: H) -> T
where
	H: AsRef<[u8]>,
	T: From<[u8; N]>,
{
	hex2array_unchecked(hex).into()
}

/// Deserialize hex to `T`, where `T: From<Vec<u8>>`.
///
/// # Examples
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, PartialEq)]
/// struct LJF(Vec<u8>);
/// impl From<Vec<u8>> for LJF {
/// 	fn from(vec: Vec<u8>) -> Self {
/// 		Self(vec)
/// 	}
/// }
///
/// #[derive(Debug, PartialEq, Deserialize)]
/// struct WrappedLJF {
/// 	#[serde(deserialize_with = "array_bytes::hex_deserialize_into")]
/// 	ljf: LJF,
/// }
///
/// assert_eq!(
/// 	serde_json::from_str::<WrappedLJF>(r#"{
/// 		"ljf": "0x4c6f7665204a616e6520466f7265766572"
/// 	}"#).unwrap(),
/// 	WrappedLJF {
/// 		ljf: LJF(b"Love Jane Forever".to_vec())
/// 	}
/// );
#[cfg(feature = "serde")]
pub fn hex_deserialize_into<'de, D, T>(hex: D) -> CoreResult<T, D::Error>
where
	D: Deserializer<'de>,
	T: From<Vec<u8>>,
{
	Ok(hex2bytes_unchecked(<&str>::deserialize(hex)?).into())
}

/// Deserialize hex to `T`, where `T: From<[u8; N]>`.
///
/// # Examples
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, PartialEq)]
/// struct LJF([u8; 17]);
/// impl From<[u8; 17]> for LJF {
/// 	fn from(array: [u8; 17]) -> Self {
/// 		Self(array)
/// 	}
/// }
///
/// #[derive(Debug, PartialEq, Deserialize)]
/// struct WrappedLJF {
/// 	#[serde(deserialize_with = "array_bytes::hex_deserialize_n_into")]
/// 	ljf: LJF,
/// }
///
/// assert_eq!(
/// 	serde_json::from_str::<WrappedLJF>(r#"{
/// 		"ljf": "0x4c6f7665204a616e6520466f7265766572"
/// 	}"#).unwrap(),
/// 	WrappedLJF {
/// 		ljf: LJF(*b"Love Jane Forever")
/// 	}
/// );
#[cfg(feature = "serde")]
pub fn hex_deserialize_n_into<'de, D, T, const N: usize>(hex: D) -> CoreResult<T, D::Error>
where
	D: Deserializer<'de>,
	T: From<[u8; N]>,
{
	Ok(hex2array_unchecked(<&str>::deserialize(hex)?).into())
}

/// Deserialize hex to any Rust primitive num types.
///
/// # Examples
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, PartialEq, Deserialize)]
/// struct LJF {
/// 	#[serde(deserialize_with = "array_bytes::de_hex2num")]
/// 	_0: u8,
/// 	#[serde(deserialize_with = "array_bytes::de_hex2num")]
/// 	_1: u8,
/// 	#[serde(deserialize_with = "array_bytes::de_hex2num")]
/// 	_2: u8,
/// 	#[serde(deserialize_with = "array_bytes::de_hex2num")]
/// 	_3: u32,
/// }
///
/// assert_eq!(
/// 	serde_json::from_str::<LJF>(
/// 		r#"{
/// 		"_0": "0x5",
/// 		"_1": "0x2",
/// 		"_2": "0x0",
/// 		"_3": "0x522"
/// 	}"#
/// 	)
/// 	.unwrap(),
/// 	LJF { _0: 5, _1: 2, _2: 0, _3: 1314 }
/// );
/// ```
#[cfg(feature = "serde")]
pub fn de_hex2num<'de, D, T>(hex: D) -> CoreResult<T, D::Error>
where
	D: Deserializer<'de>,
	T: TryFromHex,
{
	let hex = <&str>::deserialize(hex)?;

	T::try_from_hex(hex).map_err(|_| D::Error::custom(alloc::format!("Invalid hex str `{}`", hex)))
}

/// Deserialize hex to [`Vec<u8>`].
///
/// # Examples
/// ```
/// use serde::Deserialize;
///
/// #[derive(Debug, PartialEq, Deserialize)]
/// struct LJF {
/// 	#[serde(deserialize_with = "array_bytes::de_hex2bytes")]
/// 	ljf: Vec<u8>,
/// }
///
/// assert_eq!(
/// 	serde_json::from_str::<LJF>(
/// 		r#"{
/// 		"ljf": "0x4c6f7665204a616e6520466f7265766572"
/// 	}"#
/// 	)
/// 	.unwrap(),
/// 	LJF { ljf: (*b"Love Jane Forever").to_vec() }
/// );
/// ```
#[cfg(feature = "serde")]
pub fn de_hex2bytes<'de, D>(hex: D) -> CoreResult<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	let hex = <&str>::deserialize(hex)?;

	hex2bytes(hex).map_err(|_| D::Error::custom(alloc::format!("Invalid hex str `{}`", hex)))
}

fn strip_0x(hex: &[u8]) -> &[u8] {
	if let Some(hex) = hex.strip_prefix(b"0x") {
		hex
	} else {
		hex
	}
}

fn is_hex_ascii(byte: &u8) -> bool {
	// Convert to lowercase.
	let byte = byte | 0b10_0000;

	matches!(byte, b'0'..=b'9' | b'a'..=b'f')
}

fn hex_ascii2digit(hex_ascii: &u8) -> Option<u8> {
	// Convert to lowercase.
	let hex_ascii = hex_ascii | 0b10_0000;

	match hex_ascii {
		b'0'..=b'9' => Some(hex_ascii - b'0'),
		b'a'..=b'f' => Some(hex_ascii - b'a' + 10),
		_ => None,
	}
}

fn hex2byte(hex_ascii_1: (&u8, usize), hex_ascii_2: (&u8, usize)) -> Result<u8> {
	let byte = hex_ascii2digit(hex_ascii_1.0)
		.ok_or(Error::InvalidCharacter { character: *hex_ascii_1.0 as _, index: hex_ascii_1.1 })?
		<< 4 | hex_ascii2digit(hex_ascii_2.0)
		.ok_or(Error::InvalidCharacter { character: *hex_ascii_2.0 as _, index: hex_ascii_2.1 })?;

	Ok(byte)
}

fn hex2byte_unchecked(hex_ascii_1: &u8, hex_ascii_2: &u8) -> u8 {
	hex_ascii2digit(hex_ascii_1).unwrap() << 4 | hex_ascii2digit(hex_ascii_2).unwrap()
}
