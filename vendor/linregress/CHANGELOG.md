## Unreleased

## 0.5.2
### Changed
 - Update `nalgebra` to `0.32.2`
 - Replace dev dependency `criterion` with `tiny-bench`
 - Minor performance improvements

## 0.5.1
### Changed
- Update `nalgebra` to version 0.32.1
- Optimize various calculations (see [ef94ca0](https://github.com/n1m3/linregress/commit/ef94ca07ededb5d551309d581555778f71bf5136))
### Bug fixes
- Fix model fitting failure when standard error is equal to zero

## 0.5.0
### Changed
- Update `nalgebra` to `0.31.0`
- Fully replace `Cephes` special functions with new implementation based on implementation in `statrs`
- Remove `statrs` dependency. All statistics related code is now implemented in this crate
- Remove quickcheck related dev-dependencies
- Port benchmarks to criterion

### Added
- Added `assert_almost_eq` and `assert_slices_almost_eq` macros for use in doc tests

## 0.5.0-alpha.1
### Breaking changes
- Rework API to remove `RegressionParameters` struct
- `FormulaRegressionBuilder::fit_without_statistics` returns a `Vec`
- The fields of `RegressionModel` and `LowLevelRegressionModel` are now private.
- Appropriate accessor methods have been added.
- `RegressionParameters::pairs` has been replaced with `iter_` methods on `RegressionModel`

## 0.4.4
### Added
- Add `data_columns` method to `FormulaRegressionBuilder`
  It allows setting the regressand a regressors without using a formula string.
- Add `fit_low_level_regression_model` and `fit_low_level_regression_model_without_statistics`
  functions for performing a regression directly on a matrix of input data

## 0.4.3
### Changed
- Update `statrs` dependency to `0.15.0` to avoid multiple versions of `nalgebra` in out dependency tree

## 0.4.2
### Changed
- Update `nalgebra` to `0.27.1` in response to RUSTSEC-2021-0070
- Update `statrs` to `0.14.0`
