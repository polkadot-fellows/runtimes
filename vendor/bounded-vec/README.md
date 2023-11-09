[![Coverage Status](https://coveralls.io/repos/github/ergoplatform/bounded-vec/badge.svg?branch=develop)](https://coveralls.io/github/ergoplatform/bounded-vec?branch=develop)
[![Latest Version](https://img.shields.io/crates/v/bounded-vec.svg)](https://crates.io/crates/bounded-vec) [![Documentation](https://docs.rs/bounded-vec/badge.svg)](https://docs.rs/crate/bounded-vec)

## bounded-vec
`BoundedVec<T, L, U>` - Non-empty rust `std::vec::Vec` wrapper with type guarantees on lower(`L`) and upper(`U`) bounds for items quantity. Inspired by [vec1](https://github.com/rustonaut/vec1).

## Example

```rust
use bounded_vec::BoundedVec;

let data: BoundedVec<u8, 2, 4> = [1u8,2].into();

assert_eq!(*data.first(), 1);
assert_eq!(*data.last(), 2);

// creates a new BoundedVec by mapping each element
let data = data.mapped(|x|x*2);
assert_eq!(data, [2u8,4].into());
```

## Crate features
- optional(non-default) `serde` feature that adds serialization to `BoundedVec`.
- optional(non-default) `arbitrary` feature that adds `proptest::Arbitrary` implementation to `BoundedVec`.

## Changelog
See [CHANGELOG.md](CHANGELOG.md).

## Contributing
See [Contributing](CONTRIBUTING.md) guide.
