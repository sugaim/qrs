use crate::{chrono::GenericDateTime, num::Real};

// -----------------------------------------------------------------------------
// DayCount
//
pub trait DayCount: Sized {
    type Rate<V: Real>;

    fn dcf<Tz>(&self, from: &GenericDateTime<Tz>, to: &GenericDateTime<Tz>) -> f64
    where
        Tz: chrono::TimeZone;

    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V>;
}
