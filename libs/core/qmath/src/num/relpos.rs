use chrono::Datelike;

use super::Real;

/// Trait to compute relative position between two points on a 1-dim line.
pub trait RelPos<X = Self>: PartialOrd<X> {
    type Output: Real;

    /// Compute relative position of `self` between `left` and `right`.
    ///
    /// Returns [None] if and only if `left` and `right` are equal.
    fn relpos_between(&self, left: &X, right: &X) -> Option<Self::Output>;
}

impl<T: Real> RelPos<T> for T {
    type Output = T;

    #[inline]
    fn relpos_between(&self, left: &T, right: &T) -> Option<T> {
        let den = right.clone() - left;
        if den.is_zero() {
            None
        } else {
            Some((self.clone() - left) / &den)
        }
    }
}

impl<Tz: chrono::TimeZone> RelPos for chrono::DateTime<Tz> {
    type Output = f64;

    #[inline]
    fn relpos_between(
        &self,
        left: &chrono::DateTime<Tz>,
        right: &chrono::DateTime<Tz>,
    ) -> Option<f64> {
        let left = left.timestamp_micros() as f64;
        let right = right.timestamp_micros() as f64;
        if left == right {
            None
        } else {
            let self_ = self.timestamp_micros() as f64;
            Some((self_ - left) / (right - left))
        }
    }
}

impl RelPos for chrono::NaiveDate {
    type Output = f64;

    #[inline]
    fn relpos_between(&self, left: &chrono::NaiveDate, right: &chrono::NaiveDate) -> Option<f64> {
        let left = left.num_days_from_ce() as f64;
        let right = right.num_days_from_ce() as f64;
        if left == right {
            None
        } else {
            let self_ = self.num_days_from_ce() as f64;
            Some((self_ - left) / (right - left))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(unused_imports)]
    use rstest::rstest;

    fn datetime(
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
    ) -> chrono::DateTime<chrono::FixedOffset> {
        use chrono::TimeZone;
        chrono::FixedOffset::east_opt(9 * 60 * 60)
            .unwrap()
            .with_ymd_and_hms(year, month, day, hour, 0, 0)
            .unwrap()
    }

    #[rstest]
    #[case::ok(0.0, 10.0, 5.0, Some(0.5))]
    #[case::ok(0.0, 10.0, 0.0, Some(0.0))]
    #[case::ok(0.0, 10.0, 10.0, Some(1.0))]
    #[case::ok(0.0, 10.0, 15.0, Some(1.5))]
    #[case::ok(0.0, 10.0, -5.0, Some(-0.5))]
    #[case::ok(0.0, 10.0, 20.0, Some(2.0))]
    #[case::ok(1.5, 2.5, 2.0, Some(0.5))]
    #[case::ok(1.5, 2.5, 1.0, Some(-0.5))]
    #[case::ok(1.5, 2.5, 3.0, Some(1.5))]
    #[case::ok(1.5, 2.5, 0.0, Some(-1.5))]
    #[case::err(0.0, 0.0, 0.0, None)]
    #[case::err(0.0, 0.0, 1.0, None)]
    #[case::err(0.0, 0.0, -1.0, None)]
    #[case::err(0.5, 0.5, 0.5, None)]
    #[case::err(0.5, 0.5, 1.0, None)]
    #[case::err(0.5, 0.5, -1.0, None)]
    fn test_relpos_between(
        #[case] left: f64,
        #[case] right: f64,
        #[case] point: f64,
        #[case] expected: Option<f64>,
    ) {
        let tested = point.relpos_between(&left, &right);
        let invtested = point.relpos_between(&right, &left);

        assert_eq!(tested, expected);
        assert_eq!(invtested, expected.map(|x| 1.0 - x));
    }

    #[rstest]
    #[case(
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 11).unwrap(),
        Some(0.0)
    )]
    #[case(
        chrono::NaiveDate::from_ymd_opt(2021, 1, 6).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 11).unwrap(),
        Some(0.5)
    )]
    #[case(
        chrono::NaiveDate::from_ymd_opt(2020, 12, 27).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 11).unwrap(),
        Some(-0.5)
    )]
    #[case(
        chrono::NaiveDate::from_ymd_opt(2021, 1, 16).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 11).unwrap(),
        Some(1.5)
    )]
    #[case(
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        chrono::NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
        None
    )]
    fn test_relpos_naive_date(
        #[case] point: chrono::NaiveDate,
        #[case] from: chrono::NaiveDate,
        #[case] to: chrono::NaiveDate,
        #[case] expected: Option<f64>,
    ) {
        let tested = point.relpos_between(&from, &to);
        let invtested = point.relpos_between(&to, &from);

        assert_eq!(tested, expected);
        assert_eq!(invtested, expected.map(|x| 1.0 - x));
    }

    #[rstest]
    #[case(
        datetime(2021, 1, 1, 0),
        datetime(2021, 1, 1, 0),
        datetime(2021, 1, 11, 0),
        Some(0.0)
    )]
    #[case(
        datetime(2021, 1, 6, 0),
        datetime(2021, 1, 1, 0),
        datetime(2021, 1, 11, 0),
        Some(0.5)
    )]
    #[case(
        datetime(2020, 12, 27, 0),
        datetime(2021, 1, 1, 0),
        datetime(2021, 1, 11, 0),
        Some(-0.5)
    )]
    #[case(
        datetime(2021, 1, 16, 0),
        datetime(2021, 1, 1, 0),
        datetime(2021, 1, 11, 0),
        Some(1.5)
    )]
    #[case(
        datetime(2021, 1, 1, 6),
        datetime(2021, 1, 1, 1),
        datetime(2021, 1, 1, 21),
        Some(0.25)
    )]
    #[case(
        datetime(2021, 1, 1, 6),
        datetime(2021, 1, 1, 1),
        datetime(2021, 1, 1, 1),
        None
    )]
    fn test_relpos_chrono(
        #[case] point: chrono::DateTime<chrono::FixedOffset>,
        #[case] from: chrono::DateTime<chrono::FixedOffset>,
        #[case] to: chrono::DateTime<chrono::FixedOffset>,
        #[case] expected: Option<f64>,
    ) {
        let tested = point.relpos_between(&from, &to);
        let invtested = point.relpos_between(&to, &from);

        assert_eq!(tested, expected);
        assert_eq!(invtested, expected.map(|x| 1.0 - x));
    }
}
