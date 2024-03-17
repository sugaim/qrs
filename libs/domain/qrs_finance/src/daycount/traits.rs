use qrs_chrono::{CalendarOutOfRangeError, NaiveDate};
use qrs_math::num::{Real, Scalar};

// -----------------------------------------------------------------------------
// DcfError
//
#[derive(Debug, thiserror::Error)]
pub enum DcfError {
    #[error("`from`({}) is later than `to`({}). `dcf(to, from)` is {}", .from, .to, .rev_dcf)]
    ReverseOrder {
        from: NaiveDate,
        to: NaiveDate,
        rev_dcf: f64,
    },
    #[error(transparent)]
    CalendarError(#[from] CalendarOutOfRangeError),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

// -----------------------------------------------------------------------------
// Dcf
//
/// Day count convention
pub trait Dcf: Sized {
    /// Calculate day count fraction between two dates.
    /// `from` is included, but `to` is not included.
    ///
    /// If `from` is later than `to`, it returns an error [`DcfError::ReverseOrder`].
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Result<f64, DcfError>;
}

// -----------------------------------------------------------------------------
// RateDcf
//
/// Day count convention related to rate, such as Actual/360, Actual/365, etc.
pub trait RateDcf: Dcf {
    type Rate<V: Real>;

    /// Create a rate object from the given annual rate value.
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V>;
}

// -----------------------------------------------------------------------------
// InterestRate
//
/// Trait for financial rate.
///
/// Rate is not just a number because it obeys some day count convention.
/// So this trait provides access to rate value and day count convention consistently.
/// Also, this provides a static relationship between rate and the convention.
pub trait InterestRate: Sized {
    type Value: Real;
    type Convention: RateDcf;

    /// Get day count convention which this rate obeys.
    fn convention(&self) -> Self::Convention;

    /// Value of annual rate. Unit is 1. Not percent nor bps.
    fn into_value(self) -> Self::Value;

    /// Value of annual rate. Unit is percent.
    #[inline]
    fn into_value_in_percent(self) -> Self::Value {
        const MULT: f64 = 1e2;
        let mult = <Self::Value as Scalar>::nearest_value_of(MULT);
        self.into_value() * &mult
    }

    /// Value of annual rate. Unit is bps.
    #[inline]
    fn into_value_in_bps(self) -> Self::Value {
        const MULT: f64 = 1e4;
        let mult = <Self::Value as Scalar>::nearest_value_of(MULT);
        self.into_value() * &mult
    }
}
