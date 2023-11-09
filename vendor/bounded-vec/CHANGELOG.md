# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

<!-- next-header -->
## [Unreleased] - ReleaseDate
## [0.7.1] - 2022-08-01
### Added
- fix `Abrbitrary` impl to honor upper(U) and lower(L) bounds;

## [0.7.0] - 2022-07-26


### Added
- `Abrbitrary` impl behind `arbitrary` feature;
- `BoundedVec::opt_empty_vec()` - empty `Vec<T>` to `Option<BoundedVec<T>>`;
- `Option<BoundedVec>::to_vec()` to get `Vec<T>`;

## [0.6.0] - 2022-04-21

### Added
- `NonEmptyVec` type alias;

## [0.5.0] - 2021-10-14

### Added
- `serde` optional feature;

## [0.4.0] - 2021-08-04

### Added
- `AsRef`, `AsMut`, `Hash`, `Ord`, `PartialOrd` impl for BoundedVec;
- `Error` impl for BoundedVecOutOfBounds;

## [0.3.0] - 2021-06-09

### Added 
- `IntoIter` impl for `BoundedVec`;
- `BoundedVec::enumerated` (return new instance with indices included);
- `BoundedVec::split_last` (return last and all the rest of the elements);

## [0.2.0] - 2021-05-11

### Added
- `BoundedVec::try_mapped`,  `BoundedVec::try_mapped_ref`;

## [0.1.0] - 2021-05-10

- initial `BoundedVec` implementation including `mapped()`, `first()`, `last()`, to/from array/`Vec`;

<!-- next-url -->
[Unreleased]: https://github.com/ergoplatform/bounded-vec/compare/v0.7.1...HEAD
[0.7.1]: https://github.com/ergoplatform/bounded-vec/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/ergoplatform/bounded-vec/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/ergoplatform/bounded-vec/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/ergoplatform/bounded-vec/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ergoplatform/bounded-vec/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/ergoplatform/bounded-vec/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ergoplatform/bounded-vec/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/ergoplatform/bounded-vec/compare/v0.0.0...v0.1.0
