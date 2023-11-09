#![allow(clippy::upper_case_acronyms)]

// array-bytes
use crate::*;

macro_rules! bytes {
	($v:expr; $n:expr) => {{
		let mut v = Vec::new();

		for _ in 0..$n {
			v.push($v);
		}

		v
	}};
}

#[derive(Debug, PartialEq)]
struct LJF(Vec<u8>);
impl From<Vec<u8>> for LJF {
	fn from(bytes: Vec<u8>) -> Self {
		Self(bytes)
	}
}

#[derive(Debug, PartialEq)]
struct LJFN([u8; 17]);
impl From<[u8; 17]> for LJFN {
	fn from(array: [u8; 17]) -> Self {
		Self(array)
	}
}

#[test]
fn try_from_hex_should_work() {
	assert_eq!(u8::try_from_hex("0x8"), Ok(8));
	assert_eq!(u8::try_from_hex("8"), Ok(8));
	assert_eq!(u16::try_from_hex("0x10"), Ok(16));
	assert_eq!(u16::try_from_hex("10"), Ok(16));
	assert_eq!(u32::try_from_hex("0x20"), Ok(32));
	assert_eq!(u32::try_from_hex("20"), Ok(32));
	assert_eq!(u64::try_from_hex("0x40"), Ok(64));
	assert_eq!(u64::try_from_hex("40"), Ok(64));
	assert_eq!(u128::try_from_hex("0x80"), Ok(128));
	assert_eq!(u128::try_from_hex("80"), Ok(128));
	assert_eq!(u128::try_from_hex("0xabcdef"), Ok(11259375));
	assert_eq!(u128::try_from_hex("abcdef"), Ok(11259375));
	assert_eq!(u128::try_from_hex("0x1a2b3c4d5e6f"), Ok(28772997619311));
	assert_eq!(u128::try_from_hex("1a2b3c4d5e6f"), Ok(28772997619311));
}

#[test]
fn hex_should_work() {
	assert_eq!(8_u8.hex("0x"), "0x8");
	assert_eq!(8_u8.hex(""), "8");
	assert_eq!(16_u16.hex("0x"), "0x10");
	assert_eq!(16_u16.hex(""), "10");
	assert_eq!(32_u32.hex("0x"), "0x20");
	assert_eq!(32_u32.hex(""), "20");
	assert_eq!(64_u64.hex("0x"), "0x40");
	assert_eq!(64_u64.hex(""), "40");
	assert_eq!(128_u128.hex("0x"), "0x80");
	assert_eq!(128_u128.hex(""), "80");
	assert_eq!(11259375_u128.hex("0x"), "0xabcdef");
	assert_eq!(11259375_u128.hex(""), "abcdef");
	assert_eq!(28772997619311_u128.hex("0x"), "0x1a2b3c4d5e6f");
	assert_eq!(28772997619311_u128.hex(""), "1a2b3c4d5e6f");
}

#[test]
fn slice2array_should_work() {
	assert_eq!(slice2array::<8, _>(&[0; 8]), Ok([0; 8]));
}

#[test]
fn slice_n_into_should_work() {
	assert_eq!(slice_n_into::<17, u8, LJFN>(b"Love Jane Forever"), Ok(LJFN(*b"Love Jane Forever")));
}

#[test]
fn slice_n_into_unchecked_should_work() {
	assert_eq!(
		slice_n_into_unchecked::<17, u8, LJFN>(b"Love Jane Forever"),
		LJFN(*b"Love Jane Forever")
	);
}

#[test]
fn vec2array_should_work() {
	assert_eq!(vec2array::<8, _>(bytes![0; 8]), Ok([0; 8]));
}

#[test]
fn vec_n_into_should_work() {
	assert_eq!(
		vec_n_into::<17, u8, LJFN>(b"Love Jane Forever".to_vec()),
		Ok(LJFN(*b"Love Jane Forever"))
	);
}

#[test]
fn vec_n_into_unchecked_should_work() {
	assert_eq!(
		vec_n_into_unchecked::<17, u8, LJFN>(b"Love Jane Forever".to_vec()),
		LJFN(*b"Love Jane Forever")
	);
}

#[test]
fn bytes2hex_should_work() {
	assert_eq!(
		bytes2hex("0x", b"Love Jane Forever"),
		String::from("0x4c6f7665204a616e6520466f7265766572")
	);
	assert_eq!(
		bytes2hex("", b"Love Jane Forever"),
		String::from("4c6f7665204a616e6520466f7265766572")
	);
}

#[test]
fn hex_bytes2hex_str_should_work() {
	assert_eq!(
		hex_bytes2hex_str(b"0x4c6f7665204a616e6520466f7265766572"),
		Ok("0x4c6f7665204a616e6520466f7265766572"),
	);
	assert_eq!(
		hex_bytes2hex_str(b"4c6f7665204a616e6520466f7265766572"),
		Ok("4c6f7665204a616e6520466f7265766572"),
	);

	assert_eq!(
		hex_bytes2hex_str(b"4c6f766 5204a616e6520466f7265766572"),
		Err(Error::InvalidCharacter { character: ' ', index: 7 }),
	);
	assert_eq!(
		hex_bytes2hex_str(b"4c6f766520 4a616e6520466f7265766572"),
		Err(Error::InvalidCharacter { character: ' ', index: 10 }),
	);
}

#[test]
fn hex_bytes2hex_str_unchecked_should_work() {
	unsafe {
		assert_eq!(
			hex_bytes2hex_str_unchecked(b"0x4c6f7665204a616e6520466f7265766572"),
			"0x4c6f7665204a616e6520466f7265766572",
		);
		assert_eq!(
			hex_bytes2hex_str_unchecked(b"4c6f7665204a616e6520466f7265766572"),
			"4c6f7665204a616e6520466f7265766572",
		);
	}
}

#[test]
fn hex2array_should_work() {
	assert_eq!(hex2array("0x4c6f7665204a616e6520466f7265766572"), Ok(*b"Love Jane Forever"));
	assert_eq!(
		hex2array("0x4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(*b"Love Jane Forever")
	);
	assert_eq!(hex2array("4c6f7665204a616e6520466f7265766572"), Ok(*b"Love Jane Forever"));
	assert_eq!(
		hex2array("4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(*b"Love Jane Forever")
	);
}

#[test]
fn hex2bytes_should_work() {
	assert_eq!(
		hex2bytes("0x4c6f7665204a616e6520466f7265766572"),
		Ok(b"Love Jane Forever".to_vec())
	);
	assert_eq!(
		hex2bytes("0x4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(b"Love Jane Forever".to_vec())
	);
	assert_eq!(hex2bytes("4c6f7665204a616e6520466f7265766572"), Ok(b"Love Jane Forever".to_vec()));
	assert_eq!(
		hex2bytes("4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(b"Love Jane Forever".to_vec())
	);

	assert_eq!(hex2bytes("我爱你"), Err(Error::InvalidLength));
	assert_eq!(hex2bytes("0x我爱你"), Err(Error::InvalidLength));

	assert_eq!(hex2bytes("我爱你 "), Err(Error::InvalidCharacter { character: 'æ', index: 0 }));
	assert_eq!(hex2bytes(" 我爱你"), Err(Error::InvalidCharacter { character: ' ', index: 0 }));
}

#[test]
fn hex2bytes_unchecked_should_work() {
	assert_eq!(hex2bytes_unchecked("0x4c6f7665204a616e6520466f7265766572"), *b"Love Jane Forever");
	assert_eq!(
		hex2bytes_unchecked("0x4c6f7665204a616e6520466f7265766572".as_bytes()),
		*b"Love Jane Forever"
	);
	assert_eq!(hex2bytes_unchecked("4c6f7665204a616e6520466f7265766572"), *b"Love Jane Forever");
	assert_eq!(
		hex2bytes_unchecked("4c6f7665204a616e6520466f7265766572".as_bytes()),
		*b"Love Jane Forever"
	);
}

#[test]
fn hex2slice_should_work() {
	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice("0x4c6f7665204a616e6520466f7265766572", &mut bytes),
			Ok(b"Love Jane Forever".as_slice())
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}
	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice("0x4c6f7665204a616e6520466f7265766572".as_bytes(), &mut bytes),
			Ok(b"Love Jane Forever".as_slice())
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}

	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice("4c6f7665204a616e6520466f7265766572", &mut bytes),
			Ok(b"Love Jane Forever".as_slice())
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}
	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice("4c6f7665204a616e6520466f7265766572".as_bytes(), &mut bytes),
			Ok(b"Love Jane Forever".as_slice())
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}

	assert_eq!(hex2slice("0", &mut []), Err(Error::InvalidLength));
	assert_eq!(hex2slice("0x0", &mut []), Err(Error::InvalidLength));

	assert_eq!(hex2slice("00", &mut []), Err(Error::MismatchedLength { expect: 1 }));
	assert_eq!(hex2slice("0x0001", &mut []), Err(Error::MismatchedLength { expect: 2 }));

	assert_eq!(
		hex2slice("fg", &mut [0]),
		Err(Error::InvalidCharacter { character: 'g', index: 1 })
	);
	assert_eq!(
		hex2slice("0xyz", &mut [0]),
		Err(Error::InvalidCharacter { character: 'y', index: 0 })
	);
}

#[test]
fn hex2slice_unchecked_should_work() {
	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice_unchecked("0x4c6f7665204a616e6520466f7265766572", &mut bytes),
			b"Love Jane Forever"
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}
	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice_unchecked("0x4c6f7665204a616e6520466f7265766572".as_bytes(), &mut bytes),
			b"Love Jane Forever"
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}

	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice_unchecked("4c6f7665204a616e6520466f7265766572", &mut bytes),
			b"Love Jane Forever"
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}
	{
		let mut bytes = [0; 17];

		assert_eq!(
			hex2slice_unchecked("4c6f7665204a616e6520466f7265766572".as_bytes(), &mut bytes),
			b"Love Jane Forever"
		);
		assert_eq!(bytes, *b"Love Jane Forever");
	}
}

#[test]
fn hex_into_should_work() {
	assert_eq!(
		hex_into::<_, LJF>("0x4c6f7665204a616e6520466f7265766572"),
		Ok(LJF(b"Love Jane Forever".to_vec()))
	);
	assert_eq!(
		hex_into::<_, LJF>("0x4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(LJF(b"Love Jane Forever".to_vec()))
	);
	assert_eq!(
		hex_into::<_, LJF>("4c6f7665204a616e6520466f7265766572"),
		Ok(LJF(b"Love Jane Forever".to_vec()))
	);
	assert_eq!(
		hex_into::<_, LJF>("4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(LJF(b"Love Jane Forever".to_vec()))
	);
}

#[test]
fn hex_n_into_should_work() {
	assert_eq!(
		hex_n_into::<_, LJFN, 17>("0x4c6f7665204a616e6520466f7265766572"),
		Ok(LJFN(*b"Love Jane Forever"))
	);
	assert_eq!(
		hex_n_into::<_, LJFN, 17>("0x4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(LJFN(*b"Love Jane Forever"))
	);
	assert_eq!(
		hex_n_into::<_, LJFN, 17>("4c6f7665204a616e6520466f7265766572"),
		Ok(LJFN(*b"Love Jane Forever"))
	);
	assert_eq!(
		hex_n_into::<_, LJFN, 17>("4c6f7665204a616e6520466f7265766572".as_bytes()),
		Ok(LJFN(*b"Love Jane Forever"))
	);
}

#[test]
fn hex_into_unchecked_should_work() {
	assert_eq!(
		hex_into_unchecked::<_, LJF>("0x4c6f7665204a616e6520466f7265766572"),
		LJF(b"Love Jane Forever".to_vec())
	);
	assert_eq!(
		hex_into_unchecked::<_, LJF>("0x4c6f7665204a616e6520466f7265766572".as_bytes()),
		LJF(b"Love Jane Forever".to_vec())
	);
	assert_eq!(
		hex_into_unchecked::<_, LJF>("4c6f7665204a616e6520466f7265766572"),
		LJF(b"Love Jane Forever".to_vec())
	);
	assert_eq!(
		hex_into_unchecked::<_, LJF>("4c6f7665204a616e6520466f7265766572".as_bytes()),
		LJF(b"Love Jane Forever".to_vec())
	);
}

#[test]
fn hex_n_into_unchecked_should_work() {
	assert_eq!(
		hex_n_into_unchecked::<_, LJFN, 17>("0x4c6f7665204a616e6520466f7265766572"),
		LJFN(*b"Love Jane Forever")
	);
	assert_eq!(
		hex_n_into_unchecked::<_, LJFN, 17>("0x4c6f7665204a616e6520466f7265766572".as_bytes()),
		LJFN(*b"Love Jane Forever")
	);
	assert_eq!(
		hex_n_into_unchecked::<_, LJFN, 17>("4c6f7665204a616e6520466f7265766572"),
		LJFN(*b"Love Jane Forever")
	);
	assert_eq!(
		hex_n_into_unchecked::<_, LJFN, 17>("4c6f7665204a616e6520466f7265766572".as_bytes()),
		LJFN(*b"Love Jane Forever")
	);
}

#[cfg(feature = "serde")]
#[test]
fn hex_deserialize_into_should_work() {
	#[derive(Debug, PartialEq, Deserialize)]
	struct WrappedLJF {
		#[serde(deserialize_with = "hex_deserialize_into")]
		ljf: LJF,
	}

	assert_eq!(
		serde_json::from_str::<WrappedLJF>(
			r#"{
				"ljf": "0x4c6f7665204a616e6520466f7265766572"
			}"#
		)
		.unwrap(),
		WrappedLJF { ljf: LJF(b"Love Jane Forever".to_vec()) }
	);
	assert_eq!(
		serde_json::from_str::<WrappedLJF>(
			r#"{
				"ljf": "4c6f7665204a616e6520466f7265766572"
			}"#
		)
		.unwrap(),
		WrappedLJF { ljf: LJF(b"Love Jane Forever".to_vec()) }
	);
}

#[cfg(feature = "serde")]
#[test]
fn hex_deserialize_n_into_should_work() {
	#[derive(Debug, PartialEq, Deserialize)]
	struct WrappedLJF {
		#[serde(deserialize_with = "hex_deserialize_n_into")]
		ljf: LJFN,
	}

	assert_eq!(
		serde_json::from_str::<WrappedLJF>(
			r#"{
				"ljf": "0x4c6f7665204a616e6520466f7265766572"
			}"#
		)
		.unwrap(),
		WrappedLJF { ljf: LJFN(*b"Love Jane Forever") }
	);
	assert_eq!(
		serde_json::from_str::<WrappedLJF>(
			r#"{
				"ljf": "4c6f7665204a616e6520466f7265766572"
			}"#
		)
		.unwrap(),
		WrappedLJF { ljf: LJFN(*b"Love Jane Forever") }
	);
}

#[cfg(feature = "serde")]
#[test]
fn de_hex2num_should_work() {
	macro_rules! assert_de_hex2num {
		($num_type:ty) => {{
			#[derive(Debug, PartialEq, Deserialize)]
			struct LJFN {
				#[serde(deserialize_with = "de_hex2num")]
				_0: $num_type,
				#[serde(deserialize_with = "de_hex2num")]
				_1: $num_type,
				#[serde(deserialize_with = "de_hex2num")]
				_2: $num_type,
				#[serde(deserialize_with = "de_hex2num")]
				_3: u32,
			}

			assert_eq!(
				serde_json::from_str::<LJFN>(
					r#"{
						"_0": "0x5",
						"_1": "0x2",
						"_2": "0x0",
						"_3": "0x522"
					}"#
				)
				.unwrap(),
				LJFN { _0: 5, _1: 2, _2: 0, _3: 1314 }
			);
			assert_eq!(
				serde_json::from_str::<LJFN>(
					r#"{
						"_0": "5",
						"_1": "2",
						"_2": "0",
						"_3": "522"
					}"#
				)
				.unwrap(),
				LJFN { _0: 5, _1: 2, _2: 0, _3: 1314 }
			);
		}};
	}

	assert_de_hex2num!(isize);
	assert_de_hex2num!(i8);
	assert_de_hex2num!(i16);
	assert_de_hex2num!(i32);
	assert_de_hex2num!(i64);
	assert_de_hex2num!(i128);
	assert_de_hex2num!(usize);
	assert_de_hex2num!(u8);
	assert_de_hex2num!(u16);
	assert_de_hex2num!(u32);
	assert_de_hex2num!(u64);
	assert_de_hex2num!(u128);
}

#[cfg(feature = "serde")]
#[test]
fn de_hex2bytes_should_work() {
	#[derive(Debug, PartialEq, Deserialize)]
	struct LJFN {
		#[serde(deserialize_with = "de_hex2bytes")]
		ljf: Vec<u8>,
	}

	assert_eq!(
		serde_json::from_str::<LJFN>(
			r#"{
				"ljf": "0x4c6f7665204a616e6520466f7265766572"
			}"#
		)
		.unwrap(),
		LJFN { ljf: (*b"Love Jane Forever").to_vec() }
	);
	assert_eq!(
		serde_json::from_str::<LJFN>(
			r#"{
				"ljf": "4c6f7665204a616e6520466f7265766572"
			}"#
		)
		.unwrap(),
		LJFN { ljf: (*b"Love Jane Forever").to_vec() }
	);
}

#[test]
fn random_input_should_work() {
	const DATA_1: &[u8] = include_bytes!("lib.rs");
	const DATA_2: &[u8] = include_bytes!("test.rs");

	let data = [DATA_1, DATA_2].concat();

	for chunks_size in [8, 16, 32, 64, 128, 256, 512, 1024] {
		let mut data_pieces = Vec::new();

		data.chunks(chunks_size).enumerate().for_each(|(i, chunk)| {
			data_pieces.push(bytes2hex(if i % 2 == 0 { "0x" } else { "" }, chunk))
		});

		let data_pieces = data_pieces
			.into_iter()
			.map(|piece| match strip_0x(piece.as_bytes()).len() {
				8 => hex2array_unchecked::<_, 4>(&piece).to_vec(),
				32 => hex2array_unchecked::<_, 16>(&piece).to_vec(),
				64 => hex2array_unchecked::<_, 32>(&piece).to_vec(),
				128 => hex2array_unchecked::<_, 64>(&piece).to_vec(),
				256 => hex2array_unchecked::<_, 128>(&piece).to_vec(),
				512 => hex2array_unchecked::<_, 256>(&piece).to_vec(),
				1024 => hex2array_unchecked::<_, 512>(&piece).to_vec(),
				2048 => hex2array_unchecked::<_, 1024>(&piece).to_vec(),
				_ => hex2bytes_unchecked(&piece),
			})
			.collect::<Vec<_>>();

		assert_eq!(data_pieces.concat(), data)
	}
}
