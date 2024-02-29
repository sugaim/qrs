use qrs_chrono::DateTime;
use qrs_math::num::{Real, Scalar};

use crate::daycount::{DayCount, RateDayCount};

// -----------------------------------------------------------------------------
// Rate
//
pub trait Rate: Sized {
    type Value: Real;
    type Convention: RateDayCount;

    /// Get day count convention which this rate obeys.
    fn convention(&self) -> Self::Convention;

    /// Value of annual rate. Unit is 1. Not percent nor bps.
    fn value(&self) -> Self::Value;

    /// Value of annual rate. Unit is percent.
    #[inline]
    fn value_in_percent(&self) -> Self::Value {
        const MULT: f64 = 1e2;
        let mult = <Self::Value as Scalar>::nearest_value_of(MULT);
        self.value() * &mult
    }

    /// Value of annual rate. Unit is bps.
    #[inline]
    fn value_in_bps(&self) -> Self::Value {
        const MULT: f64 = 1e4;
        let mult = <Self::Value as Scalar>::nearest_value_of(MULT);
        self.value() * &mult
    }

    /// Calculate change ratio between two dates.
    #[inline]
    fn ratio_between(&self, from: &DateTime, to: &DateTime) -> Self::Value {
        let dcf = self.convention().dcf(from, to);
        let dcf = <Self::Value as Scalar>::nearest_value_of(dcf);
        self.value() * &dcf
    }
}
