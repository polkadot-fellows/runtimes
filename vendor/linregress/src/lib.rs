/*!
  `linregress` provides an easy to use implementation of ordinary
  least squared linear regression with some basic statistics.
  See [`RegressionModel`] for details.

  The builder [`FormulaRegressionBuilder`] is used to construct a model from a
  table of data and an R-style formula or a list of columns to use.
  Currently only very simple formulae are supported,
  see [`FormulaRegressionBuilder.formula`] for details.

  # Example

  ```
  use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};

  # use linregress::Error;
  # fn main() -> Result<(), Error> {
  let y = vec![1., 2. ,3. , 4., 5.];
  let x1 = vec![5., 4., 3., 2., 1.];
  let x2 = vec![729.53, 439.0367, 42.054, 1., 0.];
  let x3 = vec![258.589, 616.297, 215.061, 498.361, 0.];
  let data = vec![("Y", y), ("X1", x1), ("X2", x2), ("X3", x3)];
  let data = RegressionDataBuilder::new().build_from(data)?;
  let formula = "Y ~ X1 + X2 + X3";
  let model = FormulaRegressionBuilder::new()
      .data(&data)
      .formula(formula)
      .fit()?;
  let parameters: Vec<_> = model.iter_parameter_pairs().collect();
  let pvalues: Vec<_> = model.iter_p_value_pairs().collect();
  let standard_errors: Vec<_> = model.iter_se_pairs().collect();
  assert_eq!(
      parameters,
      vec![
          ("X1", -0.9999999999999745),
          ("X2", 1.5872719805187785e-15),
          ("X3", -1.4246416546459528e-15),
      ]
  );
  assert_eq!(
      standard_errors,
      vec![
          ("X1", 9.799066977595267e-13),
          ("X2", 4.443774660560714e-15),
          ("X3", 2.713389610740135e-15),
      ]
  );
  assert_eq!(
      pvalues,
      vec![
          ("X1", 6.238279788691533e-13),
          ("X2", 0.7815975465725482),
          ("X3", 0.6922074604135647),
      ]
  );
  # Ok(())
  # }
  ```

  [`RegressionModel`]: struct.RegressionModel.html
  [`FormulaRegressionBuilder`]: struct.FormulaRegressionBuilder.html
  [`FormulaRegressionBuilder.formula`]: struct.FormulaRegressionBuilder.html#method.formula
*/

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::iter;
use std::ops::Neg;

use nalgebra::{DMatrix, DVector};

pub use crate::error::{Error, InconsistentSlopes};
use crate::stats::students_t_cdf;

mod error;
mod stats;
#[cfg(test)]
mod tests;

macro_rules! ensure {
    ($predicate:expr, $error:expr) => {
        if !$predicate {
            return Err($error);
        }
    };
}

/// Only exposed for use in doc comments. This macro is not considered part of this crate's stable API.
#[macro_export]
macro_rules! assert_almost_eq {
    ($a:expr, $b:expr) => {
        $crate::assert_almost_eq!($a, $b, 1.0E-14);
    };
    ($a:expr, $b:expr, $prec:expr) => {
        if !$crate::almost_equal($a, $b, $prec) {
            panic!("assert_almost_eq failed:\n{:?} vs\n{:?}", $a, $b);
        }
    };
}

/// Only exposed for use in doc comments. This macro is not considered part of this crate's stable API.
#[macro_export]
macro_rules! assert_slices_almost_eq {
    ($a:expr, $b:expr) => {
        $crate::assert_slices_almost_eq!($a, $b, 1.0E-14);
    };
    ($a:expr, $b:expr, $prec:expr) => {
        if !$crate::slices_almost_equal($a, $b, $prec) {
            panic!("assert_slices_almost_eq failed:\n{:?} vs\n{:?}", $a, $b);
        }
    };
}

/// Only exposed for use in doc comments. This function is not considered part of this crate's stable API.
#[doc(hidden)]
pub fn almost_equal(a: f64, b: f64, precision: f64) -> bool {
    if a.is_infinite() || b.is_infinite() || a.is_nan() || b.is_nan() {
        false
    } else {
        (a - b).abs() <= precision
    }
}

/// Only exposed for use in doc comments. This function is not considered part of this crate's stable API.
#[doc(hidden)]
pub fn slices_almost_equal(a: &[f64], b: &[f64], precision: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    for (&x, &y) in a.iter().zip(b.iter()) {
        if !almost_equal(x, y, precision) {
            return false;
        }
    }
    true
}

/// Compares `a` and `b` approximately.
///
/// They are considered equal if
/// `(a-b).abs() <= epsilon` or they differ by at most `max_ulps`
/// `units of least precision` i.e. there are at most `max_ulps`
/// other representable floating point numbers between `a` and `b`
fn ulps_eq(a: f64, b: f64, epsilon: f64, max_ulps: u32) -> bool {
    if (a - b).abs() <= epsilon {
        return true;
    }
    if a.signum() != b.signum() {
        return false;
    }
    let a: u64 = a.to_bits();
    let b: u64 = b.to_bits();
    a.abs_diff(b) <= max_ulps as u64
}

/// A builder to create and fit a linear regression model.
///
/// Given a dataset and a set of columns to use this builder
/// will produce an ordinary least squared linear regression model.
///
/// See [`formula`] and [`data`] for details on how to configure this builder.
///
/// The pseudo inverse method is used to fit the model.
///
/// # Usage
///
/// ```
/// use linregress::{FormulaRegressionBuilder, RegressionDataBuilder, assert_almost_eq};
///
/// # use linregress::Error;
/// # fn main() -> Result<(), Error> {
/// let y = vec![1., 2. ,3., 4.];
/// let x = vec![4., 3., 2., 1.];
/// let data = vec![("Y", y), ("X", x)];
/// let data = RegressionDataBuilder::new().build_from(data)?;
/// let model = FormulaRegressionBuilder::new().data(&data).formula("Y ~ X").fit()?;
/// // Alternatively
/// let model = FormulaRegressionBuilder::new().data(&data).data_columns("Y", ["X"]).fit()?;
/// let params = model.parameters();
/// assert_almost_eq!(params[0], 4.999999999999998);
/// assert_almost_eq!(params[1], -0.9999999999999989);
/// assert_eq!(model.regressor_names()[0], "X");
/// # Ok(())
/// # }
/// ```
///
/// [`formula`]: struct.FormulaRegressionBuilder.html#method.formula
/// [`data`]: struct.FormulaRegressionBuilder.html#method.data
#[derive(Debug, Clone)]
pub struct FormulaRegressionBuilder<'a> {
    data: Option<&'a RegressionData<'a>>,
    formula: Option<Cow<'a, str>>,
    columns: Option<(Cow<'a, str>, Vec<Cow<'a, str>>)>,
}

impl<'a> Default for FormulaRegressionBuilder<'a> {
    fn default() -> Self {
        FormulaRegressionBuilder::new()
    }
}

impl<'a> FormulaRegressionBuilder<'a> {
    /// Create as new FormulaRegressionBuilder with no data or formula set.
    pub fn new() -> Self {
        FormulaRegressionBuilder {
            data: None,
            formula: None,
            columns: None,
        }
    }

    /// Set the data to be used for the regression.
    ///
    /// The data has to be given as a reference to a [`RegressionData`] struct.
    /// See [`RegressionDataBuilder`] for details.
    ///
    /// [`RegressionData`]: struct.RegressionData.html
    /// [`RegressionDataBuilder`]: struct.RegressionDataBuilder.html
    pub fn data(mut self, data: &'a RegressionData<'a>) -> Self {
        self.data = Some(data);
        self
    }

    /// Set the formula to use for the regression.
    ///
    /// The expected format is `<regressand> ~ <regressor 1> + <regressor 2>`.
    ///
    /// E.g. for a regressand named Y and three regressors named A, B and C
    /// the correct format would be `Y ~ A + B + C`.
    ///
    /// Note that there is currently no special support for categorical variables.
    /// So if you have a categorical variable with more than two distinct values
    /// or values that are not `0` and `1` you will need to perform "dummy coding" yourself.
    ///
    /// Alternatively you can use [`data_columns`][Self::data_columns].
    pub fn formula<T: Into<Cow<'a, str>>>(mut self, formula: T) -> Self {
        self.formula = Some(formula.into());
        self
    }

    /// Set the columns to be used as regressand and regressors for the regression.
    ///
    /// Note that there is currently no special support for categorical variables.
    /// So if you have a categorical variable with more than two distinct values
    /// or values that are not `0` and `1` you will need to perform "dummy coding" yourself.
    ///
    /// Alternatively you can use [`formula`][Self::formula].
    pub fn data_columns<I, S1, S2>(mut self, regressand: S1, regressors: I) -> Self
    where
        I: IntoIterator<Item = S2>,
        S1: Into<Cow<'a, str>>,
        S2: Into<Cow<'a, str>>,
    {
        let regressand = regressand.into();
        let regressors: Vec<_> = regressors.into_iter().map(|i| i.into()).collect();
        self.columns = Some((regressand, regressors));
        self
    }

    /// Fits the model and returns a [`RegressionModel`] if successful.
    /// You need to set the data with [`data`] and a formula with [`formula`]
    /// before you can use it.
    ///
    /// [`RegressionModel`]: struct.RegressionModel.html
    /// [`data`]: struct.FormulaRegressionBuilder.html#method.data
    /// [`formula`]: struct.FormulaRegressionBuilder.html#method.formula
    pub fn fit(self) -> Result<RegressionModel, Error> {
        let FittingData(input_vector, output_matrix, outputs) =
            Self::get_matrices_and_regressor_names(self)?;
        RegressionModel::try_from_matrices_and_regressor_names(input_vector, output_matrix, outputs)
    }

    /// Like [`fit`] but does not perfom any statistics on the resulting model.
    /// Returns a [`Vec`] containing the model parameters
    /// (in the order `intercept, column 1, column 2, …`) if successfull.
    ///
    /// This is usefull if you do not care about the statistics or the model and data
    /// you want to fit result in too few residual degrees of freedom to perform
    /// statistics.
    ///
    /// [`fit`]: struct.FormulaRegressionBuilder.html#method.fit
    pub fn fit_without_statistics(self) -> Result<Vec<f64>, Error> {
        let FittingData(input_vector, output_matrix, _output_names) =
            Self::get_matrices_and_regressor_names(self)?;
        let low_level_result = fit_ols_pinv(input_vector, output_matrix)?;
        let parameters = low_level_result.params;
        Ok(parameters.iter().copied().collect())
    }

    fn get_matrices_and_regressor_names(self) -> Result<FittingData, Error> {
        let (input, outputs) = self.get_data_columns()?;
        let data = &self.data.ok_or(Error::NoData)?.data;
        let input_vector: Vec<f64> = data
            .get(&input)
            .cloned()
            .ok_or_else(|| Error::ColumnNotInData(input.into()))?;
        let mut output_matrix = Vec::new();
        // Add column of all ones as the first column of the matrix
        output_matrix.resize(input_vector.len(), 1.);
        // Add each input as a new column of the matrix
        for output in &outputs {
            let output_vec = data
                .get(output.as_ref())
                .ok_or_else(|| Error::ColumnNotInData(output.to_string()))?;
            ensure!(
                output_vec.len() == input_vector.len(),
                Error::RegressorRegressandDimensionMismatch(output.to_string())
            );
            output_matrix.extend_from_slice(output_vec);
        }
        let output_matrix = DMatrix::from_vec(input_vector.len(), outputs.len() + 1, output_matrix);
        let outputs: Vec<_> = outputs.iter().map(|x| x.to_string()).collect();
        Ok(FittingData(input_vector, output_matrix, outputs))
    }

    fn get_data_columns(&self) -> Result<(Cow<'_, str>, Vec<Cow<'_, str>>), Error> {
        match (self.formula.as_ref(), self.columns.as_ref()) {
            (Some(..), Some(..)) => Err(Error::BothFormulaAndDataColumnsGiven),
            (Some(formula), None) => Self::parse_formula(formula),
            (None, Some((regressand, regressors))) => {
                ensure!(!regressors.is_empty(), Error::InvalidDataColumns);
                Ok((regressand.clone(), regressors.clone()))
            }
            (None, None) => Err(Error::NoFormula),
        }
    }

    fn parse_formula(formula: &str) -> Result<(Cow<'_, str>, Vec<Cow<'_, str>>), Error> {
        let (input, outputs) = formula.split_once('~').ok_or(Error::InvalidFormula)?;
        let input = input.trim();
        let outputs: Vec<_> = outputs
            .split('+')
            .map(str::trim)
            .filter(|x| !x.is_empty())
            .map(|i| i.into())
            .collect();
        ensure!(!outputs.is_empty(), Error::InvalidFormula);
        Ok((input.into(), outputs))
    }
}

/// A simple tuple struct to reduce the type complxity of the
/// return type of get_matrices_and_regressor_names.
struct FittingData(Vec<f64>, DMatrix<f64>, Vec<String>);

/// A container struct for the regression data.
///
/// This struct is obtained using a [`RegressionDataBuilder`].
///
/// [`RegressionDataBuilder`]: struct.RegressionDataBuilder.html
#[derive(Debug, Clone)]
pub struct RegressionData<'a> {
    data: HashMap<Cow<'a, str>, Vec<f64>>,
}

impl<'a> RegressionData<'a> {
    /// Constructs a new `RegressionData` struct from any collection that
    /// implements the `IntoIterator` trait.
    ///
    /// The iterator must consist of tupels of the form `(S, Vec<f64>)` where
    /// `S` is a type that can be converted to a `Cow<'a, str>`.
    ///
    /// `invalid_value_handling` specifies what to do if invalid data is encountered.
    fn new<I, S>(
        data: I,
        invalid_value_handling: InvalidValueHandling,
    ) -> Result<RegressionData<'a>, Error>
    where
        I: IntoIterator<Item = (S, Vec<f64>)>,
        S: Into<Cow<'a, str>>,
    {
        let temp: HashMap<_, _> = data
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect();
        let first_key = temp.keys().next();
        ensure!(
            first_key.is_some(),
            Error::RegressionDataError("The data contains no columns.".into())
        );
        let first_key = first_key.unwrap();
        let first_len = temp[first_key].len();
        ensure!(
            first_len > 0,
            Error::RegressionDataError("The data contains an empty column.".into())
        );
        for key in temp.keys() {
            let this_len = temp[key].len();
            ensure!(
                this_len == first_len,
                Error::RegressionDataError(
                    "The lengths of the columns in the given data are inconsistent.".into()
                )
            );
            ensure!(
                !key.contains('~') && !key.contains('+'),
                Error::RegressionDataError(
                    "The column names may not contain `~` or `+`, because they are used \
                             as separators in the formula."
                        .into()
                )
            );
        }
        if Self::check_if_all_columns_are_equal(&temp) {
            return Err(Error::RegressionDataError(
                "All input columns contain only equal values. Fitting this model would lead \
                     to invalid statistics."
                    .into(),
            ));
        }
        if Self::check_if_data_is_valid(&temp) {
            return Ok(Self { data: temp });
        }
        match invalid_value_handling {
            InvalidValueHandling::ReturnError => Err(Error::RegressionDataError(
                "The data contains a non real value (NaN or infinity or negative infinity). \
                 If you would like to silently drop these values configure the builder with \
                 InvalidValueHandling::DropInvalid."
                    .into(),
            )),
            InvalidValueHandling::DropInvalid => {
                let temp = Self::drop_invalid_values(temp);
                let first_key = temp.keys().next().expect("Cleaned data has no columns.");
                let first_len = temp[first_key].len();
                ensure!(
                    first_len > 0,
                    Error::RegressionDataError("The cleaned data is empty.".into())
                );
                Ok(Self { data: temp })
            }
        }
    }

    fn check_if_all_columns_are_equal(data: &HashMap<Cow<'a, str>, Vec<f64>>) -> bool {
        for column in data.values() {
            if column.is_empty() {
                return false;
            }
            let first_iter = iter::repeat(&column[0]).take(column.len());
            if !first_iter.eq(column.iter()) {
                return false;
            }
        }
        true
    }

    fn check_if_data_is_valid(data: &HashMap<Cow<'a, str>, Vec<f64>>) -> bool {
        for column in data.values() {
            if column.iter().any(|x| !x.is_finite()) {
                return false;
            }
        }
        true
    }

    fn drop_invalid_values(
        data: HashMap<Cow<'a, str>, Vec<f64>>,
    ) -> HashMap<Cow<'a, str>, Vec<f64>> {
        let mut invalid_rows: BTreeSet<usize> = BTreeSet::new();
        for column in data.values() {
            for (index, value) in column.iter().enumerate() {
                if !value.is_finite() {
                    invalid_rows.insert(index);
                }
            }
        }
        let mut cleaned_data = HashMap::new();
        for (key, mut column) in data {
            for index in invalid_rows.iter().rev() {
                column.remove(*index);
            }
            cleaned_data.insert(key, column);
        }
        cleaned_data
    }
}

/// A builder to create a RegressionData struct for use with a [`FormulaRegressionBuilder`].
///
/// [`FormulaRegressionBuilder`]: struct.FormulaRegressionBuilder.html
#[derive(Debug, Clone, Copy, Default)]
pub struct RegressionDataBuilder {
    handle_invalid_values: InvalidValueHandling,
}

impl RegressionDataBuilder {
    /// Create a new [`RegressionDataBuilder`].
    ///
    /// [`RegressionDataBuilder`]: struct.RegressionDataBuilder.html
    pub fn new() -> Self {
        Self::default()
    }

    /// Configure how to handle non real `f64` values (NaN or infinity or negative infinity) using
    /// a variant of the [`InvalidValueHandling`] enum.
    ///
    /// The default value is [`ReturnError`].
    ///
    /// # Example
    /// ```
    /// use linregress::{InvalidValueHandling, RegressionDataBuilder};
    ///
    /// # use linregress::Error;
    /// # fn main() -> Result<(), Error> {
    /// let builder = RegressionDataBuilder::new();
    /// let builder = builder.invalid_value_handling(InvalidValueHandling::DropInvalid);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`InvalidValueHandling`]: enum.InvalidValueHandling.html
    /// [`ReturnError`]: enum.InvalidValueHandling.html#variant.ReturnError
    pub fn invalid_value_handling(mut self, setting: InvalidValueHandling) -> Self {
        self.handle_invalid_values = setting;
        self
    }

    /// Build a [`RegressionData`] struct from the given data.
    ///
    /// Any type that implements the [`IntoIterator`] trait can be used for the data.
    /// This could for example be a [`Hashmap`] or a [`Vec`].
    ///
    /// The iterator must consist of tupels of the form `(S, Vec<f64>)` where
    /// `S` is a type that implements `Into<Cow<str>>`, such as [`String`] or [`str`].
    ///
    /// You can think of this format as the representation of a table of data where
    /// each tuple `(S, Vec<f64>)` represents a column. The `S` is the header or label of the
    /// column and the `Vec<f64>` contains the data of the column.
    ///
    /// Because `~` and `+` are used as separators in the formula they may not be used in the name
    /// of a data column.
    ///
    /// # Example
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use linregress::RegressionDataBuilder;
    ///
    /// # use linregress::Error;
    /// # fn main() -> Result<(), Error> {
    /// let mut data1 = HashMap::new();
    /// data1.insert("Y", vec![1., 2., 3., 4.]);
    /// data1.insert("X", vec![4., 3., 2., 1.]);
    /// let regression_data1 = RegressionDataBuilder::new().build_from(data1)?;
    ///
    /// let y = vec![1., 2., 3., 4.];
    /// let x = vec![4., 3., 2., 1.];
    /// let data2 = vec![("X", x), ("Y", y)];
    /// let regression_data2 = RegressionDataBuilder::new().build_from(data2)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`RegressionData`]: struct.RegressionData.html
    /// [`IntoIterator`]: https://doc.rust-lang.org/std/iter/trait.IntoIterator.html
    /// [`Hashmap`]: https://doc.rust-lang.org/std/collections/struct.HashMap.html
    /// [`Vec`]: https://doc.rust-lang.org/std/vec/struct.Vec.html
    /// [`String`]: https://doc.rust-lang.org/std/string/struct.String.html
    /// [`str`]: https://doc.rust-lang.org/std/primitive.str.html
    pub fn build_from<'a, I, S>(self, data: I) -> Result<RegressionData<'a>, Error>
    where
        I: IntoIterator<Item = (S, Vec<f64>)>,
        S: Into<Cow<'a, str>>,
    {
        RegressionData::new(data, self.handle_invalid_values)
    }
}

/// How to proceed if given non real `f64` values (NaN or infinity or negative infinity).
///
/// Used with [`RegressionDataBuilder.invalid_value_handling`]
///
/// The default is [`ReturnError`].
///
/// [`RegressionDataBuilder.invalid_value_handling`]: struct.RegressionDataBuilder.html#method.invalid_value_handling
/// [`ReturnError`]: enum.InvalidValueHandling.html#variant.ReturnError
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum InvalidValueHandling {
    /// Return an error to the caller.
    ReturnError,
    /// Drop the columns containing the invalid values.
    DropInvalid,
}

impl Default for InvalidValueHandling {
    fn default() -> InvalidValueHandling {
        InvalidValueHandling::ReturnError
    }
}

/// A fitted regression model.
///
/// Is the result of [`FormulaRegressionBuilder.fit()`].
///
///[`FormulaRegressionBuilder.fit()`]: struct.FormulaRegressionBuilder.html#method.fit
#[derive(Debug, Clone)]
pub struct RegressionModel {
    regressor_names: Vec<String>,
    model: LowLevelRegressionModel,
}

impl RegressionModel {
    /// The names of the regressor columns
    #[inline]
    pub fn regressor_names(&self) -> &[String] {
        &self.regressor_names
    }

    /// The two-tailed p-values for the t-statistics of the parameters
    #[inline]
    pub fn p_values(&self) -> &[f64] {
        self.model.p_values()
    }

    /// Iterates over pairs of regressor columns and their associated p-values
    ///
    /// # Note
    ///
    /// This does not include the value for the intercept.
    ///
    /// # Usage
    ///
    /// ```
    /// # use linregress::Error;
    /// # fn main() -> Result<(), Error> {
    /// use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
    ///
    /// let y = vec![1.,2. ,3. , 4.];
    /// let x1 = vec![4., 3., 2., 1.];
    /// let x2 = vec![1., 2., 3., 4.];
    /// let data = vec![("Y", y), ("X1", x1), ("X2", x2)];
    /// let data = RegressionDataBuilder::new().build_from(data)?;
    /// let model = FormulaRegressionBuilder::new().data(&data).formula("Y ~ X1 + X2").fit()?;
    /// let pairs: Vec<(&str, f64)> = model.iter_p_value_pairs().collect();
    /// assert_eq!(pairs[0], ("X1", 1.7052707580549508e-28));
    /// assert_eq!(pairs[1], ("X2", 2.522589878779506e-31));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn iter_p_value_pairs(&self) -> impl Iterator<Item = (&str, f64)> + '_ {
        self.regressor_names
            .iter()
            .zip(self.model.p_values().iter().skip(1))
            .map(|(r, &v)| (r.as_str(), v))
    }

    /// The residuals of the model
    #[inline]
    pub fn residuals(&self) -> &[f64] {
        self.model.residuals()
    }

    /// The model's intercept and slopes (also known as betas)
    #[inline]
    pub fn parameters(&self) -> &[f64] {
        self.model.parameters()
    }

    /// Iterates over pairs of regressor columns and their associated slope values
    ///
    /// # Note
    ///
    /// This does not include the value for the intercept.
    ///
    /// # Usage
    ///
    /// ```
    /// # use linregress::Error;
    /// # fn main() -> Result<(), Error> {
    /// use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
    ///
    /// let y = vec![1.,2. ,3. , 4.];
    /// let x1 = vec![4., 3., 2., 1.];
    /// let x2 = vec![1., 2., 3., 4.];
    /// let data = vec![("Y", y), ("X1", x1), ("X2", x2)];
    /// let data = RegressionDataBuilder::new().build_from(data)?;
    /// let model = FormulaRegressionBuilder::new().data(&data).formula("Y ~ X1 + X2").fit()?;
    /// let pairs: Vec<(&str, f64)> = model.iter_parameter_pairs().collect();
    /// assert_eq!(pairs[0], ("X1", -0.03703703703703709));
    /// assert_eq!(pairs[1], ("X2", 0.9629629629629626));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn iter_parameter_pairs(&self) -> impl Iterator<Item = (&str, f64)> + '_ {
        self.regressor_names
            .iter()
            .zip(self.model.parameters().iter().skip(1))
            .map(|(r, &v)| (r.as_str(), v))
    }

    /// The standard errors of the parameter estimates
    #[inline]
    pub fn se(&self) -> &[f64] {
        self.model.se()
    }

    /// Iterates over pairs of regressor columns and their associated standard errors
    ///
    /// # Note
    ///
    /// This does not include the value for the intercept.
    ///
    /// # Usage
    ///
    /// ```
    /// # use linregress::Error;
    /// # fn main() -> Result<(), Error> {
    /// use linregress::{FormulaRegressionBuilder, RegressionDataBuilder};
    ///
    /// let y = vec![1.,2. ,3. , 4.];
    /// let x1 = vec![4., 3., 2., 1.];
    /// let x2 = vec![1., 2., 3., 4.];
    /// let data = vec![("Y", y), ("X1", x1), ("X2", x2)];
    /// let data = RegressionDataBuilder::new().build_from(data)?;
    /// let model = FormulaRegressionBuilder::new().data(&data).formula("Y ~ X1 + X2").fit()?;
    /// let pairs: Vec<(&str, f64)> = model.iter_parameter_pairs().collect();
    /// assert_eq!(pairs[0], ("X1", -0.03703703703703709));
    /// assert_eq!(pairs[1], ("X2", 0.9629629629629626));
    /// # Ok(())
    /// # }
    /// ```
    #[inline]
    pub fn iter_se_pairs(&self) -> impl Iterator<Item = (&str, f64)> + '_ {
        self.regressor_names
            .iter()
            .zip(self.model.se().iter().skip(1))
            .map(|(r, &v)| (r.as_str(), v))
    }

    /// Sum of squared residuals
    #[inline]
    pub fn ssr(&self) -> f64 {
        self.model.ssr()
    }

    /// R-squared of the model
    #[inline]
    pub fn rsquared(&self) -> f64 {
        self.model.rsquared()
    }

    /// Adjusted R-squared of the model
    #[inline]
    pub fn rsquared_adj(&self) -> f64 {
        self.model.rsquared_adj()
    }

    /// A scale factor for the covariance matrix
    ///
    /// Note that the square root of `scale` is often
    /// called the standard error of the regression.
    #[inline]
    pub fn scale(&self) -> f64 {
        self.model.scale()
    }
    /// Evaluates the model on given new input data and returns the predicted values.
    ///
    /// The new data is expected to have the same columns as the original data.
    /// See [`RegressionDataBuilder.build`] for details on the type of the `new_data` parameter.
    ///
    /// ## Note
    ///
    /// This function does *no* special handling of non real values (NaN or infinity or negative infinity).
    /// Such a value in `new_data` will result in a corresponding meaningless prediction.
    ///
    /// ## Example
    ///
    /// ```
    /// # use linregress::{RegressionDataBuilder, FormulaRegressionBuilder, assert_slices_almost_eq};
    /// # use linregress::Error;
    /// # fn main() -> Result<(), Error> {
    /// let y = vec![1., 2., 3., 4., 5.];
    /// let x1 = vec![5., 4., 3., 2., 1.];
    /// let x2 = vec![729.53, 439.0367, 42.054, 1., 0.];
    /// let x3 = vec![258.589, 616.297, 215.061, 498.361, 0.];
    /// let data = vec![("Y", y), ("X1", x1), ("X2", x2), ("X3", x3)];
    /// let data = RegressionDataBuilder::new().build_from(data).unwrap();
    /// let formula = "Y ~ X1 + X2 + X3";
    /// let model = FormulaRegressionBuilder::new()
    ///     .data(&data)
    ///     .formula(formula)
    ///     .fit()?;
    /// let new_data = vec![
    ///     ("X1", vec![2.5, 3.5]),
    ///     ("X2", vec![2.0, 8.0]),
    ///     ("X3", vec![2.0, 1.0]),
    /// ];
    /// let prediction: Vec<f64> = model.predict(new_data)?;
    /// assert_slices_almost_eq!(&prediction, &[3.500000000000028, 2.5000000000000644]);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// [`RegressionDataBuilder.build`]: struct.RegressionDataBuilder.html#method.build_from
    pub fn predict<'a, I, S>(&self, new_data: I) -> Result<Vec<f64>, Error>
    where
        I: IntoIterator<Item = (S, Vec<f64>)>,
        S: Into<Cow<'a, str>>,
    {
        let new_data: HashMap<Cow<'_, _>, Vec<f64>> = new_data
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect();
        self.check_variables(&new_data)?;
        let input_len = new_data.values().next().unwrap().len();
        let mut new_data_values: Vec<f64> = vec![];
        for key in &self.regressor_names {
            new_data_values.extend_from_slice(new_data[&Cow::from(key)].as_slice());
        }

        let num_regressors = self.model.parameters.len() - 1;
        let new_data_matrix = DMatrix::from_vec(input_len, num_regressors, new_data_values);
        let param_matrix = DMatrix::from_iterator(
            num_regressors,
            1,
            self.model.parameters.iter().skip(1).copied(),
        );
        let intercept = self.model.parameters[0];
        let intercept_matrix =
            DMatrix::from_iterator(input_len, 1, std::iter::repeat(intercept).take(input_len));
        let predictions = (new_data_matrix * param_matrix) + intercept_matrix;
        let predictions: Vec<f64> = predictions.into_iter().copied().collect();
        Ok(predictions)
    }

    fn check_variables(
        &self,
        given_parameters: &HashMap<Cow<'_, str>, Vec<f64>>,
    ) -> Result<(), Error> {
        ensure!(!given_parameters.is_empty(), Error::NoData);
        let first_len = given_parameters.values().next().unwrap().len();
        ensure!(first_len > 0, Error::NoData);
        let model_parameters: HashSet<_> = self.regressor_names.iter().map(Cow::from).collect();
        for param in &model_parameters {
            if !given_parameters.contains_key(param) {
                return Err(Error::ColumnNotInData(param.to_string()));
            }
        }
        for (param, values) in given_parameters {
            ensure!(values.len() == first_len, Error::InconsistentVectors);
            if !model_parameters.contains(param) {
                return Err(Error::ModelColumnNotInData(param.to_string()));
            }
        }
        Ok(())
    }

    fn try_from_matrices_and_regressor_names<I: IntoIterator<Item = String>>(
        inputs: Vec<f64>,
        outputs: DMatrix<f64>,
        output_names: I,
    ) -> Result<Self, Error> {
        let low_level_result = fit_ols_pinv(inputs, outputs)?;
        let model = LowLevelRegressionModel::from_low_level_regression(low_level_result)?;
        let regressor_names: Vec<String> = output_names.into_iter().collect();
        let num_slopes = model.parameters.len() - 1;
        ensure!(
            regressor_names.len() == num_slopes,
            Error::InconsistentSlopes(InconsistentSlopes::new(regressor_names.len(), num_slopes))
        );
        Ok(Self {
            regressor_names,
            model,
        })
    }
}

/// A fitted regression model
///
/// Is the result of [`fit_low_level_regression_model`].
///
#[derive(Debug, Clone)]
pub struct LowLevelRegressionModel {
    /// The model's intercept and slopes (also known as betas).
    parameters: Vec<f64>,
    /// The standard errors of the parameter estimates.
    se: Vec<f64>,
    /// Sum of squared residuals.
    ssr: f64,
    /// R-squared of the model.
    rsquared: f64,
    /// Adjusted R-squared of the model.
    rsquared_adj: f64,
    /// The two-tailed p-values for the t-statistics of the params.
    pvalues: Vec<f64>,
    /// The residuals of the model.
    residuals: Vec<f64>,
    ///  A scale factor for the covariance matrix.
    ///
    ///  Note that the square root of `scale` is often
    ///  called the standard error of the regression.
    scale: f64,
}

impl LowLevelRegressionModel {
    fn from_low_level_regression(
        low_level_result: InternalLowLevelRegressionResult,
    ) -> Result<Self, Error> {
        let parameters = low_level_result.params;
        let singular_values = low_level_result.singular_values;
        let normalized_cov_params = low_level_result.normalized_cov_params;
        let diag = DMatrix::from_diagonal(&singular_values);
        let rank = &diag.rank(0.0);
        let input_vec = low_level_result.inputs.to_vec();
        let input_matrix = DMatrix::from_vec(low_level_result.inputs.len(), 1, input_vec);
        let residuals = &input_matrix - (low_level_result.outputs * parameters.to_owned());
        let ssr = residuals.dot(&residuals);
        let n = low_level_result.inputs.len();
        let df_resid = n - rank;
        ensure!(
            df_resid >= 1,
            Error::ModelFittingError(
                "There are not enough residual degrees of freedom to perform statistics on this model".into()));
        let scale = residuals.dot(&residuals) / df_resid as f64;
        let cov_params = normalized_cov_params * scale;
        let se = get_se_from_cov_params(&cov_params);
        let mean = input_matrix.mean();
        let mut centered_input_matrix = input_matrix;
        subtract_value_from_matrix(&mut centered_input_matrix, mean);
        let centered_tss = centered_input_matrix.dot(&centered_input_matrix);
        let rsquared = 1. - (ssr / centered_tss);
        let rsquared_adj = 1. - ((n - 1) as f64 / df_resid as f64 * (1. - rsquared));
        let tvalues = parameters
            .iter()
            .zip(se.iter())
            .map(|(&x, &y)| x / y.max(std::f64::EPSILON));
        let pvalues: Vec<f64> = tvalues
            .map(|x| students_t_cdf(x.abs().neg(), df_resid as i64).map(|i| i * 2.))
            .collect::<Option<_>>()
            .ok_or_else(|| {
                Error::ModelFittingError(
                    "Failed to calculate p-values: students_t_cdf failed due to invalid parameters"
                        .into(),
                )
            })?;
        // Convert these from internal Matrix types to user facing types
        let parameters: Vec<f64> = parameters.iter().copied().collect();
        let residuals: Vec<f64> = residuals.iter().copied().collect();
        Ok(Self {
            parameters,
            se,
            ssr,
            rsquared,
            rsquared_adj,
            pvalues,
            residuals,
            scale,
        })
    }

    /// The two-tailed p-values for the t-statistics of the parameters
    #[inline]
    pub fn p_values(&self) -> &[f64] {
        &self.pvalues
    }

    /// The residuals of the model
    #[inline]
    pub fn residuals(&self) -> &[f64] {
        &self.residuals
    }

    /// The model's intercept and slopes (also known as betas)
    #[inline]
    pub fn parameters(&self) -> &[f64] {
        &self.parameters
    }

    /// The standard errors of the parameter estimates
    #[inline]
    pub fn se(&self) -> &[f64] {
        &self.se
    }

    /// Sum of squared residuals
    #[inline]
    pub fn ssr(&self) -> f64 {
        self.ssr
    }

    /// R-squared of the model
    #[inline]
    pub fn rsquared(&self) -> f64 {
        self.rsquared
    }

    /// Adjusted R-squared of the model
    #[inline]
    pub fn rsquared_adj(&self) -> f64 {
        self.rsquared_adj
    }

    /// A scale factor for the covariance matrix
    ///
    /// Note that the square root of `scale` is often
    /// called the standard error of the regression.
    #[inline]
    pub fn scale(&self) -> f64 {
        self.scale
    }
}

/// Fit a regression model directly on a matrix of input data
///
/// Expects a matrix in the format
///
/// | regressand | intercept | regressor 1 | regressor 2 | …   |
/// |------------|-----------|-------------|-------------|-----|
/// | value      | 1.0       | value       | value       | …   |
/// | ⋮          | ⋮         | ⋮           | ⋮           | ⋮   |
///
/// in row major order.
///
/// # Note
/// - The matrix should already contain the `intercept` column consisting of only the value `1.0`.
/// - No validation of the data is performed, except for a simple dimension consistency check.
///
/// # Example
/// ```
/// # fn main() -> Result<(), linregress::Error> {
/// use linregress::{fit_low_level_regression_model, assert_slices_almost_eq};
///
/// let data_row_major: Vec<f64> = vec![
///     1., 1.0, 1., 7.,
///     3., 1.0, 2., 6.,
///     4., 1.0, 3., 5.,
///     5., 1.0, 4., 4.,
///     2., 1.0, 5., 3.,
///     3., 1.0, 6., 2.,
///     4., 1.0, 7., 1.,
/// ];
/// let model = fit_low_level_regression_model(&data_row_major, 7, 4)?;
/// let params = [
///     0.09523809523809518f64,
///     0.5059523809523807,
///     0.2559523809523811,
/// ];
/// assert_slices_almost_eq!(model.parameters(), &params);
/// # Ok(())
/// # }
/// ```
pub fn fit_low_level_regression_model(
    data_row_major: &[f64],
    num_rows: usize,
    num_columns: usize,
) -> Result<LowLevelRegressionModel, Error> {
    let regression = get_low_level_regression(data_row_major, num_rows, num_columns)?;
    let model = LowLevelRegressionModel::from_low_level_regression(regression)?;
    Ok(model)
}

/// Like [`fit_low_level_regression_model`] but does not compute any statistics after
/// fitting the model.
///
/// Returns a `Vec<f64>` analogous to the `parameters` field of [`LowLevelRegressionModel`].
pub fn fit_low_level_regression_model_without_statistics(
    data_row_major: &[f64],
    num_rows: usize,
    num_columns: usize,
) -> Result<Vec<f64>, Error> {
    let regression = get_low_level_regression(data_row_major, num_rows, num_columns)?;
    Ok(regression.params.iter().copied().collect())
}

fn get_low_level_regression(
    data_row_major: &[f64],
    num_rows: usize,
    num_columns: usize,
) -> Result<InternalLowLevelRegressionResult, Error> {
    ensure!(
        !data_row_major.is_empty() && num_rows * num_columns == data_row_major.len(),
        Error::InconsistentVectors
    );
    let data = DMatrix::from_row_slice(num_rows, num_columns, data_row_major);
    let inputs = data.view((0, 0), (num_rows, 1));
    let inputs: Vec<f64> = inputs.iter().copied().collect();
    let outputs: DMatrix<f64> = data.view((0, 1), (num_rows, num_columns - 1)).into_owned();
    fit_ols_pinv(inputs, outputs)
}

/// Result of fitting a low level matrix based model
#[derive(Debug, Clone)]
struct InternalLowLevelRegressionResult {
    inputs: Vec<f64>,
    outputs: DMatrix<f64>,
    params: DMatrix<f64>,
    singular_values: DVector<f64>,
    normalized_cov_params: DMatrix<f64>,
}

/// Performs ordinary least squared linear regression using the pseudo inverse method
fn fit_ols_pinv(
    inputs: Vec<f64>,
    outputs: DMatrix<f64>,
) -> Result<InternalLowLevelRegressionResult, Error> {
    ensure!(
        !inputs.is_empty(),
        Error::ModelFittingError(
            "Fitting the model failed because the input vector is empty".into()
        )
    );
    ensure!(
        outputs.nrows() >= 1 && outputs.ncols() >= 1,
        Error::ModelFittingError(
            "Fitting the model failed because the output matrix is empty".into()
        )
    );
    let singular_values = outputs
        .to_owned()
        .try_svd(false, false, std::f64::EPSILON, 0)
        .ok_or_else(|| {
            Error::ModelFittingError(
                "Computing the singular-value decomposition of the output matrix failed".into(),
            )
        })?
        .singular_values;
    let pinv = outputs.clone().pseudo_inverse(0.).map_err(|_| {
        Error::ModelFittingError("Taking the pinv of the output matrix failed".into())
    });
    let pinv = pinv?;
    let normalized_cov_params = &pinv * &pinv.transpose();
    let params = get_sum_of_products(&pinv, &inputs);
    ensure!(
        params.len() >= 2,
        Error::ModelFittingError("Invalid parameter matrix".into())
    );
    Ok(InternalLowLevelRegressionResult {
        inputs,
        outputs,
        params,
        singular_values,
        normalized_cov_params,
    })
}

fn subtract_value_from_matrix(matrix: &mut DMatrix<f64>, sub: f64) {
    for i in matrix.iter_mut() {
        *i -= sub;
    }
}

/// Calculates the standard errors given a model's covariate parameters
fn get_se_from_cov_params(matrix: &DMatrix<f64>) -> Vec<f64> {
    matrix
        .row_iter()
        .enumerate()
        .map(|(n, row)| row.get(n).expect("BUG: Matrix is not square").sqrt())
        .collect()
}

fn get_sum_of_products(matrix: &DMatrix<f64>, vector: &[f64]) -> DMatrix<f64> {
    DMatrix::from_iterator(
        matrix.nrows(),
        1,
        matrix
            .row_iter()
            .map(|row| row.iter().zip(vector.iter()).map(|(x, y)| x * y).sum()),
    )
}
