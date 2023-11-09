[![crates.io](https://img.shields.io/crates/v/separator.svg)](https://crates.io/crates/separator) [![Build Status](https://travis-ci.org/saghm/rust-separator.svg?branch=master)](https://travis-ci.org/saghm/rust-separator)

rust-separator
==============

**NOTE**: This crate is entirely superseded by the very excellent [num-format](https://crates.io/crates/num-format), which has all the features of `separator` and much, much more. I highly recommend using `num-format` instead of this crate.

Formats numbers into strings with thousands separators for readability. It currently supports floating-points (`f32` and `f64`), unsigned integers (`u16`, `u32`, `u64`, `u128`), signed integers (`i16`, `i32`, `i64`, `i128`), and size types (`isize` and `usize`).

Usage
-----

First, put `separator` as a dependency in your `Cargo.toml` as usual:

```
[dependencies]
separator = "0.3.1"
```

Then, import the `Separatable` trait, and call the `separated_string` on a number:

```
extern crate separator;

use separator::Separatable;

fn main() {
  let f = -120000000.34345;

  // Prints "-12,000,000.34345"
  println!("{}", f.separated_string());
}
```
