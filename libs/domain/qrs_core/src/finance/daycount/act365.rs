use crate::{chrono::GenericDateTime, finance::rate::RateAct365F, num::Real};

use super::DayCount;

// -----------------------------------------------------------------------------
// Act365F
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Act365F;

impl Default for Act365F {
    #[inline]
    fn default() -> Self {
        Self
    }
}

//
// methods
//
impl DayCount for Act365F {
    type Rate<V: Real> = RateAct365F<V>;

    #[inline]
    fn dcf<Tz>(&self, from: &GenericDateTime<Tz>, to: &GenericDateTime<Tz>) -> f64
    where
        Tz: chrono::TimeZone,
    {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        (to - from).millsecs() as f64 / MILSEC_PER_YEAR
    }

    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        RateAct365F::with_annual_rate(annual_rate)
    }
}
