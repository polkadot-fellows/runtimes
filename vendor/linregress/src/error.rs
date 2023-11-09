use std::error;
use std::fmt;

/// An error that can occur in this crate.
///
/// Generally this error corresponds to problems with input data or fitting
/// a regression model.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Error {
    /// Number of slopes and output names is inconsistent.
    InconsistentSlopes(InconsistentSlopes),
    /// Cannot fit model without data.
    NoData,
    /// Cannot fit model without formula or data columns.
    NoFormula,
    /// Given formula is invalid.
    InvalidFormula,
    /// Given data columns are invalid.
    InvalidDataColumns,
    /// You must specify either a formula or data columns.
    BothFormulaAndDataColumnsGiven,
    /// Requested column is not in data. (Column given as String)
    ColumnNotInData(String),
    /// A column used in the model is misising from the provided data
    ModelColumnNotInData(String),
    /// Regressor and regressand dimensions do not match. (Column given as String)
    RegressorRegressandDimensionMismatch(String),
    /// Error while processing the regression data. (Details given as String)
    RegressionDataError(String),
    /// Error while fitting the model. (Details given as String)
    ModelFittingError(String),
    /// The given vectors have inconsistent lengths
    InconsistentVectors,
    /// The RegressionModel internal state is inconsistent
    InconsistentRegressionModel,
}

#[derive(Debug, Clone, Copy)]
pub struct InconsistentSlopes {
    output_name_count: usize,
    slope_count: usize,
}

impl InconsistentSlopes {
    pub(crate) fn new(output_name_count: usize, slope_count: usize) -> Self {
        Self {
            output_name_count,
            slope_count,
        }
    }

    pub fn get_output_name_count(&self) -> usize {
        self.output_name_count
    }

    pub fn get_slope_count(&self) -> usize {
        self.slope_count
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InconsistentSlopes(inconsistent_slopes) => write!(
                f,
                "Number of slopes and output names is inconsistent. {} outputs != {} sloped",
                inconsistent_slopes.get_output_name_count(),
                inconsistent_slopes.get_slope_count()
            ),
            Error::NoData => write!(f, "Cannot fit model without data"),
            Error::NoFormula => write!(f, "Cannot fit model without formula"),
            Error::InvalidDataColumns => write!(f, "Invalid data columns"),
            Error::InvalidFormula => write!(
                f,
                "Invalid formula. Expected formula of the form 'y ~ x1 + x2'"
            ),
            Error::BothFormulaAndDataColumnsGiven => {
                write!(f, "You must specify either a formula or data columns")
            }
            Error::ColumnNotInData(column) => {
                write!(f, "Requested column {} is not in the data", column)
            }
            Error::RegressorRegressandDimensionMismatch(column) => write!(
                f,
                "Regressor dimensions for {} do not match regressand dimensions",
                column
            ),
            Error::RegressionDataError(detail) => {
                write!(f, "Error while processing the regression data: {}", detail)
            }
            Error::ModelFittingError(detail) => {
                write!(f, "Error while fitting the model: {}", detail)
            }
            Error::ModelColumnNotInData(column) => write!(
                f,
                "The column {} used in the model is misising from the provided data",
                column
            ),
            Error::InconsistentVectors => write!(f, "The given vectors have inconsistent lengths"),
            Error::InconsistentRegressionModel => write!(
                f,
                concat!(
                    "The RegressionModel internal state is inconsistent:",
                    " The number of regressor names and values differ."
                )
            ),
        }
    }
}
