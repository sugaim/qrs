use std::ops::{Div, Mul, MulAssign};

use qrs_chrono::{Duration, NaiveDate, Velocity};
use qrs_math::num::Real;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Dcf, InterestRate, RateDcf, _ops::define_vector_behavior};

// -----------------------------------------------------------------------------
// Act360
//
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Act360;

//
// display, serde
//
impl std::fmt::Display for Act360 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Act/365f")
    }
}

//
// methods
//
impl Dcf for Act360 {
    #[inline]
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Option<f64> {
        const DAYS_PER_YEAR: f64 = 365.;
        Some((to - from).num_days() as f64 / DAYS_PER_YEAR)
    }
}

impl RateDcf for Act360 {
    type Rate<V: Real> = Act360Rate<V>;

    /// Create a Act365F rate from the given annual rate.
    /// Note that the unit of the argument is 1. Not percent nor bps.
    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        Act360Rate::from_rate(annual_rate)
    }
}

// -----------------------------------------------------------------------------
// RateAct360
//
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize, JsonSchema)]
pub struct Act360Rate<V>(V);

//
// construction
//
impl<V> Act360Rate<V> {
    /// Create a new `RateAct360` instance with the given annual rate.
    ///
    /// Unit of the argument is 1. Not percent nor bps.
    /// Note that user must ensure that the given value is rate in Act/360 convention.
    #[inline]
    pub fn from_rate(value: V) -> Self {
        Self(value)
    }

    #[inline]
    pub fn from_ratio(ratio: V, dur: Duration) -> Self
    where
        V: Real,
    {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 360.0;
        let dcf = V::nearest_value_of(dur.millsecs() as f64 / MILSEC_PER_YEAR);
        Self(ratio / &dcf)
    }
}

//
// methods
//
impl<V: Real> InterestRate for Act360Rate<V> {
    type Value = V;
    type Convention = Act360;

    #[inline]
    fn convention(&self) -> Self::Convention {
        Act360
    }

    fn into_value(self) -> Self::Value {
        self.0
    }
}

//
// operators
//
define_vector_behavior!(Act360Rate);

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dcf() {
        let from = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2021, 1, 31).unwrap();

        let dcf = Act360.dcf(from, to).unwrap();

        assert_eq!(dcf, 30. / 360.);
    }
}
