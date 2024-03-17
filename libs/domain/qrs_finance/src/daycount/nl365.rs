use std::ops::Neg;

use qrs_chrono::{DateExtensions, Datelike, NaiveDate};
use qrs_math::num::Real;

use super::{Dcf, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Nl365
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Nl365;

//
// display, serde
//
impl std::fmt::Display for Nl365 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NL/365")
    }
}

//
// methods
//
impl Dcf for Nl365 {
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Option<f64> {
        match from.cmp(&to) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => return Some(0.0),
            std::cmp::Ordering::Greater => return self.dcf(to, from).map(Neg::neg),
        };
        let mut leap_days = ((from.year() + 1)..to.year())
            .filter(|y| NaiveDate::from_ymd_opt(*y, 1, 1).unwrap().is_leap_year())
            .count();
        if from.year() == to.year() {
            if from.is_leap_year() && (from.month() <= 2) && (2 < to.month()) {
                leap_days += 1;
            }
        } else {
            if from.is_leap_year() && (from.month() <= 2) {
                leap_days += 1;
            }
            if to.is_leap_year() && (2 < to.month()) {
                leap_days += 1;
            }
        }
        const DAYS_PER_YEAR: f64 = 365.;
        Some(((to - from).num_days() as f64 - leap_days as f64) / DAYS_PER_YEAR)
    }
}

impl RateDcf for Nl365 {
    type Rate<V: Real> = Nl365Rate<V>;

    /// Create a Act365F rate from the given annual rate.
    /// Note that the unit of the argument is 1. Not percent nor bps.
    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        Nl365Rate {
            rate: annual_rate,
            cnv: *self,
        }
    }
}

// -----------------------------------------------------------------------------
// RateNl365
//
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nl365Rate<V> {
    rate: V,
    cnv: Nl365,
}

//
// methods
//
impl<V: Real> InterestRate for Nl365Rate<V> {
    type Value = V;
    type Convention = Nl365;

    #[inline]
    fn convention(&self) -> Self::Convention {
        self.cnv
    }

    #[inline]
    fn into_value(self) -> Self::Value {
        self.rate
    }
}

//
// operators
//
impl<K, V> std::ops::Mul<K> for Nl365Rate<V>
where
    V: std::ops::Mul<K, Output = V>,
{
    type Output = Nl365Rate<V>;

    #[inline]
    fn mul(self, rhs: K) -> Self::Output {
        Self {
            rate: self.rate * rhs,
            cnv: self.cnv,
        }
    }
}

impl<K, V> std::ops::MulAssign<K> for Nl365Rate<V>
where
    V: std::ops::MulAssign<K>,
{
    #[inline]
    fn mul_assign(&mut self, rhs: K) {
        self.rate *= rhs;
    }
}

impl<K, V> std::ops::Div<K> for Nl365Rate<V>
where
    V: std::ops::Div<K, Output = V>,
{
    type Output = Nl365Rate<V>;

    #[inline]
    fn div(self, rhs: K) -> Self::Output {
        Self {
            rate: self.rate / rhs,
            cnv: self.cnv,
        }
    }
}

impl<K, V> std::ops::DivAssign<K> for Nl365Rate<V>
where
    V: std::ops::DivAssign<K>,
{
    #[inline]
    fn div_assign(&mut self, rhs: K) {
        self.rate /= rhs;
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use approx::assert_abs_diff_eq;
    use rstest::rstest;

    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[rstest]
    #[case(ymd(2021, 1, 1), ymd(2021, 1, 31), 30. / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2020, 2, 28), 27. / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2020, 2, 29), 28. / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2020, 3, 1), 28. / 365.)]
    #[case(ymd(2020, 2, 28), ymd(2020, 3, 1), 1. / 365.)]
    #[case(ymd(2020, 2, 29), ymd(2020, 3, 1), 0. / 365.)]
    #[case(ymd(2020, 3, 1), ymd(2020, 3, 31), 30. / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 4, 1), (4. * 365. + 28. + 31.) / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 3, 1), (4. * 365. + 28.) / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 29), (4. * 365. + 28.) / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 28), (4. * 365. + 27.) / 365.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 1), (4. * 365.) / 365.)]
    #[case(ymd(2020, 2, 28), ymd(2024, 2, 1), (4. * 365. - 27.) / 365.)]
    #[case(ymd(2020, 2, 29), ymd(2024, 2, 1), (4. * 365. - 28.) / 365.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 1), (4. * 365. - 28.) / 365.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 28), (4. * 365. - 1.) / 365.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 29), (4. * 365.) / 365.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 3, 1), (4. * 365.) / 365.)]
    fn test_dcf(#[case] from: NaiveDate, #[case] to: NaiveDate, #[case] expected: f64) {
        let dcf = Nl365.dcf(from, to).unwrap();

        assert_abs_diff_eq!(dcf, expected);
    }
}
