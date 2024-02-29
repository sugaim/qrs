#[cfg(feature = "chrono")]
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

#[cfg(feature = "chrono")]
impl<Tz1: TimeZone, Tz2: TimeZone> RelPos<DateTime<Tz1>> for DateTime<Tz2> {
    type Output = f64;

    fn relpos_between(&self, left: &DateTime<Tz1>, right: &DateTime<Tz1>) -> f64 {
        let left = left.timestamp_millis() as f64;
        let right = right.timestamp_millis() as f64;
        let self_ = self.timestamp_millis() as f64;
        (self_ - left) / (right - left)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relpos_between() {
        let left = 0.0;
        let right = 10.0;
        assert_eq!(5.0.relpos_between(&left, &right), 0.5);
        assert_eq!(0.0.relpos_between(&left, &right), 0.0);
        assert_eq!(10.0.relpos_between(&left, &right), 1.0);
        assert_eq!(15.0.relpos_between(&left, &right), 1.5);
        assert_eq!((-5.0).relpos_between(&left, &right), -0.5);
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn test_relpos_between_datetime() {
        let left = chrono::Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap();
        let right = chrono::Utc.with_ymd_and_hms(2021, 1, 2, 0, 0, 0).unwrap();

        let x = chrono::Utc.with_ymd_and_hms(2021, 1, 1, 12, 0, 0).unwrap();
        assert_eq!(x.relpos_between(&left, &right), 0.5);
        let x = chrono::Utc.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap();
        assert_eq!(x.relpos_between(&left, &right), 0.0);
        let x = chrono::Utc.with_ymd_and_hms(2021, 1, 2, 0, 0, 0).unwrap();
        assert_eq!(x.relpos_between(&left, &right), 1.0);
        let x = chrono::Utc.with_ymd_and_hms(2021, 1, 3, 0, 0, 0).unwrap();
        assert_eq!(x.relpos_between(&left, &right), 2.0);
        let x = chrono::Utc.with_ymd_and_hms(2020, 12, 31, 0, 0, 0).unwrap();
        assert_eq!(x.relpos_between(&left, &right), -1.0);
    }
}
