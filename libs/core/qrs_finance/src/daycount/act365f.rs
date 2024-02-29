use qrs_math::num::Real;

use crate::rate::RateAct365f;

use super::{DayCount, RateDayCount};

// -----------------------------------------------------------------------------
// Act365f
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Act365f;

impl Default for Act365f {
    #[inline]
    fn default() -> Self {
        Self
    }
}

//
// display, serde
//
impl std::fmt::Display for Act365f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Act/365F")
    }
}

//
// methods
//
impl DayCount for Act365f {
    #[inline]
    fn dcf(&self, from: &qrs_chrono::DateTime, to: &qrs_chrono::DateTime) -> f64 {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        (to - from).millsecs() as f64 / MILSEC_PER_YEAR
    }
}

impl RateDayCount for Act365f {
    type Rate<V: Real> = RateAct365f<V>;

    /// Create a Act365F rate from the given annual rate.
    /// Note that the unit of the argument is 1. Not percent nor bps.
    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        RateAct365f::from_rate(annual_rate)
    }
}
