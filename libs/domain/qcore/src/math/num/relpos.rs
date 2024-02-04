use chrono::{DateTime, TimeZone};

use super::Real;

/// Trait to compute relative position between two points on a 1-dim line.
pub trait RelPos<X = Self>: PartialOrd<X> {
    type Output: Real;

    fn relpos_between(&self, left: &X, right: &X) -> Self::Output;
}

impl<T: Real> RelPos<T> for T {
    type Output = T;

    fn relpos_between(&self, left: &T, right: &T) -> T {
        (self.clone() - left) / &(right.clone() - left)
    }
}

impl<Tz1: TimeZone, Tz2: TimeZone> RelPos<DateTime<Tz1>> for DateTime<Tz2> {
    type Output = f64;

    fn relpos_between(&self, left: &DateTime<Tz1>, right: &DateTime<Tz1>) -> f64 {
        let left = left.timestamp_millis() as f64;
        let right = right.timestamp_millis() as f64;
        let self_ = self.timestamp_millis() as f64;
        (self_ - left) / (right - left)
    }
}
