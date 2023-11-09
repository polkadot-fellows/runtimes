//! The origin benchmark comes from [rust-hex](https://github.com/KokaKiwi/rust-hex/blob/main/benches/hex.rs).
//! Thanks for their previous works.

// crates.io
use criterion::Criterion;
use rustc_hex::{FromHex, ToHex};

const DATA: &[u8] = include_bytes!("../LICENSE-GPL3");

fn bench_encode(c: &mut Criterion) {
	c.bench_function("array_bytes::bytes2hex", |b| b.iter(|| array_bytes::bytes2hex("", DATA)));

	c.bench_function("hex::encode", |b| b.iter(|| hex::encode(DATA)));

	c.bench_function("rustc_hex::to_hex", |b| b.iter(|| DATA.to_hex::<String>()));

	c.bench_function("faster_hex::hex_string", |b| b.iter(|| faster_hex::hex_string(DATA)));

	c.bench_function("faster_hex::hex_encode_fallback", |b| {
		b.iter(|| {
			let mut dst = vec![0; DATA.len() * 2];

			faster_hex::hex_encode_fallback(DATA, &mut dst);

			dst
		})
	});
}

fn bench_decode(c: &mut Criterion) {
	c.bench_function("array_bytes::hex2bytes", |b| {
		let hex = array_bytes::bytes2hex("", DATA);

		b.iter(|| array_bytes::hex2bytes(&hex).unwrap())
	});

	c.bench_function("array_bytes::hex2bytes_unchecked", |b| {
		let hex = array_bytes::bytes2hex("", DATA);

		b.iter(|| array_bytes::hex2bytes_unchecked(&hex))
	});

	c.bench_function("array_bytes::hex2slice", |b| {
		let hex = array_bytes::bytes2hex("", DATA);

		b.iter(|| {
			let mut v = vec![0; DATA.len()];

			array_bytes::hex2slice(&hex, &mut v).unwrap();

			v
		})
	});

	c.bench_function("array_bytes::hex2slice_unchecked", |b| {
		let hex = array_bytes::bytes2hex("", DATA);

		b.iter(|| {
			let mut v = vec![0; DATA.len()];

			array_bytes::hex2slice_unchecked(&hex, &mut v);

			v
		})
	});

	c.bench_function("hex::decode", |b| {
		let hex = hex::encode(DATA);

		b.iter(|| hex::decode(&hex).unwrap())
	});

	c.bench_function("hex::decode_to_slice", |b| {
		let hex = array_bytes::bytes2hex("", DATA);

		b.iter(|| {
			let mut v = vec![0; DATA.len()];

			hex::decode_to_slice(&hex, &mut v).unwrap();

			v
		})
	});

	c.bench_function("rustc_hex::from_hex", |b| {
		let hex = DATA.to_hex::<String>();

		b.iter(|| hex.from_hex::<Vec<u8>>().unwrap())
	});

	c.bench_function("faster_hex::hex_decode", move |b| {
		let hex = faster_hex::hex_string(DATA);
		let len = DATA.len();
		let mut dst = vec![0; len];

		b.iter(|| faster_hex::hex_decode(hex.as_bytes(), &mut dst).unwrap())
	});

	c.bench_function("faster_hex::hex_decode_unchecked", |b| {
		let hex = faster_hex::hex_string(DATA);
		let len = DATA.len();
		let mut dst = vec![0; len];

		b.iter(|| faster_hex::hex_decode_unchecked(hex.as_bytes(), &mut dst))
	});

	c.bench_function("faster_hex::hex_decode_fallback", |b| {
		let hex = faster_hex::hex_string(DATA);
		let len = DATA.len();
		let mut dst = vec![0; len];

		b.iter(|| faster_hex::hex_decode_fallback(hex.as_bytes(), &mut dst))
	});
}

criterion::criterion_group!(benches, bench_encode, bench_decode);
criterion::criterion_main!(benches);
