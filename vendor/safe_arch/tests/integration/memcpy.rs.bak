#![cfg(feature = "nightly")]
#![cfg(target_feature = "avx")]
#![cfg(feature = "bytemuck")]
#![feature(test)]
#![feature(slice_iter_mut_as_slice)]
#![feature(fixed_size_array)]
use std::array::FixedSizeArray; /* const generics pls */

#[allow(unused_must_use)]
#[allow(unused_variables)]
/* for &[u8] -> &[u8; 16/32] conversion */
use std::convert::TryInto;

use bytemuck;
use safe_arch::*;

fn memcpy_bytes(src: &[u8], dst: &mut [u8]) {
  if src.len() != dst.len() {
    return;
  }

  for (d, s) in dst.iter_mut().zip(src.iter()) {
    *d = *s;
  }
}

fn memcpy_avx(src: &[u8], dst: &mut [u8]) {
  if src.len() != dst.len() {
    return;
  }

  let (src_begin, src_meat, src_end) = bytemuck::pod_align_to(src);

  let mut dst_it = dst.iter_mut();
  /* Order of this zip is important, as src_begin.len() <= dst.len()
   * and zip first checks for first iterator for None, then the second.
   * So, swapping them around would result in dst_it being one byte
   * further than it should
   */
  for (s, d) in src_begin.iter().zip(dst_it.by_ref()) {
    *d = *s;
  }
  let mut dst_chunks = dst_it.into_slice().chunks_exact_mut(32);

  for (d, s) in dst_chunks.by_ref().zip(src_meat.iter()) {
    let d: &mut [i8] = bytemuck::cast_slice_mut(d);
    let d: &mut [i8; 32] = d.try_into().expect("Impossible!");
    store_unaligned_m256i(d, *s);
  }

  memcpy_bytes(src_end, dst_chunks.into_remainder());
}

fn memcpy_sse(src: &[u8], dst: &mut [u8]) {
  if src.len() != dst.len() {
    return;
  }

  let (src_begin, src_meat, src_end) = bytemuck::pod_align_to(src);

  let mut dst_it = dst.iter_mut();
  /* Order of this zip is important, as src_begin.len() <= dst.len()
   * and zip first checks for first iterator for None, then the second.
   * So, swapping them around would result in dst_it being one byte
   * further than it should
   */
  for (s, d) in src_begin.iter().zip(dst_it.by_ref()) {
    *d = *s;
  }
  let mut dst_chunks = dst_it.into_slice().chunks_exact_mut(16);

  for (d, s) in dst_chunks.by_ref().zip(src_meat.iter()) {
    let d: &mut [u8; 16] = d.try_into().expect("Impossible!");
    store_unaligned_m128i(d, *s);
  }

  memcpy_bytes(src_end, dst_chunks.into_remainder());
}

fn poor_rng(x: u64) -> u64 {
  let x = x ^ 0xDEADBEEFFEEBDAED;
  (x >> 3) | (x << 1)
}

fn random_bytes(n: usize) -> Vec<u8> {
  let mut vec = Vec::with_capacity(n);
  let mut rng = 0;

  for _ in 0..n {
    rng = poor_rng(rng);
    vec.push(rng as u8);
  }

  return vec;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test0() {
    let s = b"aoisjdiouwgowimecwieohffiowejfiowenofiweiofji".as_slice();
    let mut d = Vec::new();
    d.resize(s.len(), 0u8);

    memcpy_avx(s, &mut d);

    assert_eq!(s, d.as_slice());
  }

  #[test]
  fn test1() {
    let s = b"aoisjdiouwgowimecwieohffiowejfiowenofiweiofji".as_slice();
    let mut d = Vec::new();
    d.resize(s.len(), 0u8);

    memcpy_sse(s, &mut d);

    assert_eq!(s, d.as_slice());
  }
}

#[cfg(test)]
mod benches {
  extern crate test;
  use super::*;
  use test::{black_box, Bencher};

  const N: usize = 1 << 20;

  #[bench]
  fn bench_memcpy_avx(b: &mut Bencher) {
    let from = random_bytes(N);
    let mut into = Vec::new();
    into.resize(N, 0u8);

    b.iter(|| {
      let mut a = black_box(into[0]);
      memcpy_avx(&from, &mut into);
      a += into[0];
      let _b = black_box(a);
    });
  }

  #[bench]
  fn bench_memcpy_bytes(b: &mut Bencher) {
    let from = random_bytes(N);
    let mut into = Vec::new();
    into.resize(N, 0u8);

    b.iter(|| {
      let mut a = black_box(into[0]);
      memcpy_bytes(&from, &mut into);
      a += into[0];
      let _b = black_box(a);
    });
  }
}
