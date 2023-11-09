#![cfg(target_feature = "avx")]
#![cfg(feature = "bytemuck")]
#[cfg_attr(feature = "nightly", feature(test))]
#[allow(unused_must_use)]
#[allow(unused_variables)]

mod definitions {
  pub type Point2D = [f32; 2];
  pub const PLAYER_POS: Point2D = [128.0, 128.0];
  pub const MAX_DISTANCE: f32 = 16.0;
}

#[cfg(target_feature = "sse3")]
mod sse {
  use super::{definitions::*, scalar};

  use bytemuck;
  use safe_arch::*;

  fn sub_and_square(xyxy: m128, player_pos: m128) -> m128 {
    let xyxy = xyxy - player_pos;
    xyxy * xyxy
  }

  fn is_close(xyxy: m128, max_distance: m128) -> i32 {
    let results = cmp_lt_mask_m128(xyxy, max_distance);
    return move_mask_m128(results);
  }

  pub fn objects_close(x: &[Point2D]) -> usize {
    let player_pos: m128 =
      [PLAYER_POS[0], PLAYER_POS[1], PLAYER_POS[0], PLAYER_POS[1]].into();
    let max_distances = load_f32_splat_m128(&MAX_DISTANCE);
    let max_distances_squared = max_distances * max_distances;

    let (begin, meat, end) = bytemuck::pod_align_to(x);
    let mut it = meat.chunks_exact(2);
    let mut result = scalar::objects_close(begin);

    for chunk in it.by_ref() {
      let distances_squared = add_horizontal_m128(
        sub_and_square(chunk[0], player_pos),
        sub_and_square(chunk[1], player_pos),
      );
      let results = is_close(distances_squared, max_distances_squared);
      result += results.count_ones() as usize;
    }

    if let Some(remainder) = it.remainder().get(0) {
      let xyxy = sub_and_square(*remainder, player_pos);
      let distances_squared = add_horizontal_m128(xyxy, xyxy);
      let results = is_close(distances_squared, max_distances_squared);
      result += results.count_ones() as usize / 2;
    }

    return result + scalar::objects_close(end);
  }

  #[test]
  fn test_points_pythagoras() {
    use super::testutils::*;

    let mut rng = 0;
    for _ in 0..128 {
      rng = poor_rng(rng);
      let pos = random_positions(rng as usize);
      assert_eq!(scalar::objects_close(&pos), objects_close(&pos));
    }
  }
}

#[cfg(target_feature = "avx")]
mod avx {
  use super::{definitions::*, scalar};

  use bytemuck;
  use safe_arch::*;

  fn sub_and_square(xyxyxyxy: m256, player_pos: m256) -> m256 {
    let xyxyxyxy = xyxyxyxy - player_pos;
    xyxyxyxy * xyxyxyxy
  }

  fn is_close(xyxyxyxy: m256, max_distance: m256) -> i32 {
    let results = cmp_op_mask_m256!(xyxyxyxy, LessThanOrdered, max_distance);
    return move_mask_m256(results);
  }

  pub fn objects_close(x: &[Point2D]) -> usize {
    let player_pos: m256 = [
      PLAYER_POS[0],
      PLAYER_POS[1],
      PLAYER_POS[0],
      PLAYER_POS[1],
      PLAYER_POS[0],
      PLAYER_POS[1],
      PLAYER_POS[0],
      PLAYER_POS[1],
    ]
    .into();
    let max_distances = load_f32_splat_m256(&MAX_DISTANCE);
    let max_distances_squared = max_distances * max_distances;

    let (begin, meat, end) = bytemuck::pod_align_to(x);
    let mut it = meat.chunks_exact(2);
    let mut result = scalar::objects_close(begin);

    for chunk in it.by_ref() {
      let distances_squared = add_horizontal_m256(
        sub_and_square(chunk[0], player_pos),
        sub_and_square(chunk[1], player_pos),
      );
      let results = is_close(distances_squared, max_distances_squared);
      result += results.count_ones() as usize;
    }

    if let Some(remainder) = it.remainder().get(0) {
      let xyxy = sub_and_square(*remainder, player_pos);
      let distances_squared = add_horizontal_m256(xyxy, xyxy);
      let results = is_close(distances_squared, max_distances_squared);
      result += results.count_ones() as usize / 2;
    }

    return result + scalar::objects_close(end);
  }

  #[test]
  fn test_points_pythagoras() {
    use super::testutils::*;

    let mut rng = 0;
    for _ in 0..128 {
      rng = poor_rng(rng);
      let pos = random_positions(rng as usize);
      assert_eq!(scalar::objects_close(&pos), objects_close(&pos));
    }
  }
}

pub mod scalar {
  use super::definitions::*;

  fn sub_and_square(xy: Point2D, player_pos: Point2D) -> Point2D {
    let xy = [xy[0] - player_pos[0], xy[1] - player_pos[1]];
    return [xy[0] * xy[0], xy[1] * xy[1]];
  }

  fn is_close(xy: &Point2D) -> bool {
    let squared = sub_and_square(*xy, PLAYER_POS);
    let distance_squared = squared[0] + squared[1];
    return distance_squared < MAX_DISTANCE * MAX_DISTANCE;
  }

  pub fn objects_close(x: &[Point2D]) -> usize {
    x.iter().copied().filter(is_close).count()
  }
}

pub mod testutils {
  use super::definitions::*;

  pub fn poor_rng(x: u16) -> u16 {
    let x = x ^ 0xC0DE;
    (x >> 3) | (x << 1)
  }

  pub fn random_positions(n: usize) -> Vec<Point2D> {
    let mut vec = Vec::with_capacity(n);
    let mut rng = 0;
    let mut pos: Point2D = Default::default();

    for _ in 0..n {
      for i in 0..2 {
        rng = poor_rng(rng);
        pos[i] = rng as f32 / 256.0;
      }

      vec.push(pos);
    }

    return vec;
  }
}

#[cfg(feature = "nightly")]
#[cfg(test)]
mod benches {
  const N: usize = 1 << 20;

  extern crate test;
  use super::{definitions::*, testutils::*};
  use test::{black_box, Bencher};

  #[bench]
  fn bench_scalar_objects_close(b: &mut Bencher) {
    use super::scalar::*;
    let pos = random_positions(N);
    b.iter(|| {
      let mut x = black_box(0);
      x += objects_close(&pos);
      let _n = black_box(x);
    });
  }

  #[cfg(target_feature = "sse")]
  #[bench]
  fn bench_sse_objects_close(b: &mut Bencher) {
    use super::sse::*;
    let pos = random_positions(N);
    b.iter(|| {
      let mut x = black_box(0);
      x += objects_close(&pos);
      let _n = black_box(x);
    });
  }

  #[cfg(target_feature = "avx")]
  #[bench]
  fn bench_avx_objects_close(b: &mut Bencher) {
    use super::avx::*;
    let pos = random_positions(N);
    b.iter(|| {
      let mut x = black_box(0);
      x += objects_close(&pos);
      let _n = black_box(x);
    });
  }
}
