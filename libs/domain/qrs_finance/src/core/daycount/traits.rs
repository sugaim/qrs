use qrs_chrono::DateTime;
use qrs_math::num::{Real, Scalar};

// -----------------------------------------------------------------------------
// Dcf
//
/// Day count convention
pub trait Dcf: Sized {
    fn dcf(&self, from: &DateTime, to: &DateTime) -> f64;
}

// -----------------------------------------------------------------------------
// RateDcf
//
/// Day count convention related to rate, such as Actual/360, Actual/365, etc.
pub trait RateDcf: Dcf {
    type Rate<V: Real>;

    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V>;

    #[inline]
    fn ratio_to_rate<V: Real>(
        &self,
        ratio: V,
        from: &DateTime,
        to: &DateTime,
    ) -> Option<Self::Rate<V>> {
        if from == to {
            return None;
        }
        let dcf = self.dcf(from, to);
        let dcf = <V as Scalar>::nearest_value_of(dcf);
        Some(self.to_rate(ratio / &dcf))
    }
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

    /// Calculate change ratio between two dates.
    #[inline]
    fn into_ratio_between(self, from: &DateTime, to: &DateTime) -> Self::Value {
        let dcf = self.convention().dcf(from, to);
        let dcf = <Self::Value as Scalar>::nearest_value_of(dcf);
        self.into_value() * &dcf
    }
}
