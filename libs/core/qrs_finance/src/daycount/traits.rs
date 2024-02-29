use qrs_chrono::DateTime;
use qrs_math::num::Real;

// -----------------------------------------------------------------------------
// DayCount
//
/// Day count convention
pub trait DayCount: Sized {
    fn dcf(&self, from: &DateTime, to: &DateTime) -> f64;
}

// -----------------------------------------------------------------------------
// RateDayCount
//
/// Day count convention related to rate, such as Actual/360, Actual/365, etc.
pub trait RateDayCount: DayCount {
    type Rate<V: Real>;

    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V>;
}
