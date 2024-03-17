use std::ops::{Div, Mul, MulAssign};

use qrs_chrono::{Duration, NaiveDate, Velocity};
use qrs_math::num::{FloatBased, Real, RelPos, Vector};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Dcf, InterestRate, RateDcf, _ops::define_vector_behavior};

// -----------------------------------------------------------------------------
// Act365f
//
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Act365f;

//
// display, serde
//
impl std::fmt::Display for Act365f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Act/365f")
    }
}

//
// methods
//
impl Dcf for Act365f {
    #[inline]
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Option<f64> {
        const DAYS_PER_YEAR: f64 = 365.;
        Some((to - from).num_days() as f64 / DAYS_PER_YEAR)
    }
}

impl RateDcf for Act365f {
    type Rate<V: Real> = Act365fRate<V>;

    /// Create a Act365F rate from the given annual rate.
    /// Note that the unit of the argument is 1. Not percent nor bps.
    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        Act365fRate::from_rate(annual_rate)
    }
}

// -----------------------------------------------------------------------------
// Act365fRate
//
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize, JsonSchema)]
pub struct Act365fRate<V>(V);

//
// methods
//
impl<V> Act365fRate<V> {
    /// Create a new `Act365fRate` instance with the given annual rate.
    ///
    /// Unit of the argument is 1. Not percent nor bps.
    /// Note that user must ensure that the given value is rate in Act/365F convention.
    #[inline]
    pub fn from_rate(value: V) -> Self {
        Self(value)
    }

    #[inline]
    pub fn from_ratio(ratio: V, dur: Duration) -> Self
    where
        V: Real,
    {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        let dcf = V::nearest_value_of(dur.millsecs() as f64 / MILSEC_PER_YEAR);
        Self(ratio / &dcf)
    }
}

impl<V: Real> InterestRate for Act365fRate<V> {
    type Value = V;
    type Convention = Act365f;

    #[inline]
    fn convention(&self) -> Self::Convention {
        Act365f
    }

    #[inline]
    fn into_value(self) -> Self::Value {
        self.0
    }
}

//
// operators
//
define_vector_behavior!(Act365fRate);

impl<V: Real> RelPos for Act365fRate<V> {
    type Output = V;

    #[inline]
    fn relpos_between(&self, left: &Self, right: &Self) -> Self::Output {
        self.0.relpos_between(&left.0, &right.0)
    }
}

impl<V: FloatBased + Vector<V::BaseFloat>> Mul<Duration> for Act365fRate<V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: Duration) -> Self::Output {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        let milsec = rhs.millsecs() as f64;
        let dcf = V::nearest_base_float_of(milsec / MILSEC_PER_YEAR);
        self.0 * &dcf
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;

    use super::*;

    #[test]
    fn test_dcf() {
        let from = NaiveDate::from_ymd_opt(2021, 1, 1).unwrap();
        let to = NaiveDate::from_ymd_opt(2021, 1, 31).unwrap();

        let dcf = Act365f.dcf(from, to).unwrap();

        assert_eq!(dcf, 30. / 365.);
    }

    #[test]
    fn test_rate_from_ratio() {
        let rate = Act365fRate::from_ratio(0.05, Duration::with_days(730));
        assert_eq!(rate.into_value(), 0.025);
    }

    #[test]
    fn test_rate_relpos() {
        let left = Act365fRate::from_rate(0.02);
        let right = Act365fRate::from_rate(0.03);
        let rate = Act365fRate::from_rate(0.025);

        let pos = rate.relpos_between(&left, &right);

        assert_abs_diff_eq!(
            pos,
            rate.into_value()
                .relpos_between(&left.into_value(), &right.into_value()),
            epsilon = 1e-10
        );
    }

    #[test]
    fn test_rate_mul_duration() {
        let rate = Act365fRate::from_rate(0.025);
        let dur = Duration::with_days(730);

        let ratio = rate * dur;

        assert_abs_diff_eq!(ratio, 0.05, epsilon = 1e-10);
    }
}
