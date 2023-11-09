#![cfg(target_feature = "avx")]
#![cfg(feature = "bytemuck")]
#[cfg_attr(feature = "nightly", feature(test))]
use bytemuck;
#[allow(unused_must_use)]
#[allow(unused_variables)]
use safe_arch::*;
use std::ops::BitXor;

fn poor_rng(x: u64) -> u64 {
  let x = x ^ 0xDEADBEEFFEEBDAED;
  (x >> 3) | (x << 1)
}

fn random_bytes(n: usize) -> Vec<u64> {
  let mut vec = Vec::with_capacity(n);
  let mut rng = 0;

  for _ in 0..n {
    rng = poor_rng(rng);
    vec.push(rng);
  }

  return vec;
}

fn simple_xor_hash(s: &[u64]) -> u64 {
  s.iter().copied().fold(0, BitXor::bitxor)
}

fn simd_xor_hash(s: &[u64]) -> u64 {
  let mut ret = 0;
  let (begin, meat, end) = bytemuck::pod_align_to(s);

  ret ^= begin.iter().copied().fold(0, BitXor::bitxor);

  let zero: m256i = [0u64; 4].into();
  let x: [u64; 4] = meat.iter().copied().fold(zero, BitXor::bitxor).into();
  ret ^= x.iter().copied().fold(0, BitXor::bitxor);

  ret ^= end.iter().copied().fold(0, BitXor::bitxor);

  return ret;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_xor_hash() {
    let bytes = random_bytes(1024 * 1024 * 16 + 1);
    let a = simple_xor_hash(&bytes);
    let b = simd_xor_hash(&bytes);
    assert_eq!(a, b);
  }
}

#[cfg(feature = "nightly")]
#[cfg(test)]
mod benches {
  const BYTES: usize = 1 << 20;

  extern crate test;
  use super::*;
  use test::{black_box, Bencher};

  #[bench]
  fn bench_xor_hash_simd(b: &mut Bencher) {
    let bytes = random_bytes(BYTES);
    b.iter(|| {
      let mut x = black_box(0);
      x ^= simd_xor_hash(&bytes);
      let _n = black_box(x);
    });
  }

  #[bench]
  fn bench_xor_hash_simple(b: &mut Bencher) {
    let bytes = random_bytes(BYTES);
    b.iter(|| {
      let mut x = black_box(0);
      x ^= simple_xor_hash(&bytes);
      let _n = black_box(x);
    });
  }
}
