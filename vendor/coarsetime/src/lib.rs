//! A crate to make time measurements that focuses on speed.
//!
//! This crate is a partial replacement for the `Time` and `Duration` structures
//! from the standard library, with the following differences:
//!
//! * Speed is privileged over accuracy. In particular, `CLOCK_MONOTONIC_COARSE`
//!   is used to
//! retrieve the clock value on Linux systems, and transformations avoid
//! operations that can be slow on non-Intel systems.
//! * The number of system calls can be kept to a minimum. The "most recent
//!   timestamp" is
//! always kept in memory. It can be read with just a load operation, and can be
//! updated only as frequently as necessary.
//!
//! # Installation
//!
//! `coarsetime` is available on [crates.io](https://crates.io/crates/coarsetime) and works on
//! Rust stable, beta, and nightly.
//!
//! Windows and Unix-like systems are supported.
//!
//! Available features:
//!
//! * `nightly`: rust-nightly is being used; only required to run benchmarks.

#![allow(clippy::trivially_copy_pass_by_ref)]
#![cfg_attr(feature = "nightly", feature(test))]

mod clock;
mod duration;
mod helpers;
mod instant;
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
mod updater;

#[cfg(test)]
mod tests;

pub use self::clock::*;
pub use self::duration::*;
pub use self::instant::*;
#[cfg(not(any(target_arch = "wasm32", target_arch = "wasm64")))]
pub use self::updater::*;
