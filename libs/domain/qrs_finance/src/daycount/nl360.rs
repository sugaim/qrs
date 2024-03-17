use std::ops::Neg;

use qrs_chrono::{DateExtensions, Datelike, NaiveDate};
use qrs_math::num::Real;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Dcf, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Nl360
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Nl360;

//
// display, serde
//
impl std::fmt::Display for Nl360 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NL/360")
    }
}

//
// methods
//
impl Dcf for Nl360 {
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
        const DAYS_PER_YEAR: f64 = 360.;
        Some(((to - from).num_days() as f64 - leap_days as f64) / DAYS_PER_YEAR)
    }
}

impl RateDcf for Nl360 {
    type Rate<V: Real> = Nl360Rate<V>;

    /// Create a Act360F rate from the given annual rate.
    /// Note that the unit of the argument is 1. Not percent nor bps.
    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        Nl360Rate {
            rate: annual_rate,
            cnv: *self,
        }
    }
}

// -----------------------------------------------------------------------------
// Nl360Rate
//
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nl360Rate<V> {
    rate: V,
    cnv: Nl360,
}

//
// methods
//
impl<V: Real> InterestRate for Nl360Rate<V> {
    type Value = V;
    type Convention = Nl360;

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
impl<K, V> std::ops::Mul<K> for Nl360Rate<V>
where
    V: std::ops::Mul<K, Output = V>,
{
    type Output = Nl360Rate<V>;

    #[inline]
    fn mul(self, rhs: K) -> Self::Output {
        Self {
            rate: self.rate * rhs,
            cnv: self.cnv,
        }
    }
}

impl<K, V> std::ops::MulAssign<K> for Nl360Rate<V>
where
    V: std::ops::MulAssign<K>,
{
    #[inline]
    fn mul_assign(&mut self, rhs: K) {
        self.rate *= rhs;
    }
}

impl<K, V> std::ops::Div<K> for Nl360Rate<V>
where
    V: std::ops::Div<K, Output = V>,
{
    type Output = Nl360Rate<V>;

    #[inline]
    fn div(self, rhs: K) -> Self::Output {
        Self {
            rate: self.rate / rhs,
            cnv: self.cnv,
        }
    }
}

impl<K, V> std::ops::DivAssign<K> for Nl360Rate<V>
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
    #[case(ymd(2021, 1, 1), ymd(2021, 1, 31), 30. / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2020, 2, 28), 27. / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2020, 2, 29), 28. / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2020, 3, 1), 28. / 360.)]
    #[case(ymd(2020, 2, 28), ymd(2020, 3, 1), 1. / 360.)]
    #[case(ymd(2020, 2, 29), ymd(2020, 3, 1), 0. / 360.)]
    #[case(ymd(2020, 3, 1), ymd(2020, 3, 31), 30. / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 4, 1), (4. * 365. + 28. + 31.) / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 3, 1), (4. * 365. + 28.) / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 29), (4. * 365. + 28.) / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 28), (4. * 365. + 27.) / 360.)]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 1), (4. * 365.) / 360.)]
    #[case(ymd(2020, 2, 28), ymd(2024, 2, 1), (4. * 365. - 27.) / 360.)]
    #[case(ymd(2020, 2, 29), ymd(2024, 2, 1), (4. * 365. - 28.) / 360.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 1), (4. * 365. - 28.) / 360.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 28), (4. * 365. - 1.) / 360.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 29), (4. * 365.) / 360.)]
    #[case(ymd(2020, 3, 1), ymd(2024, 3, 1), (4. * 365.) / 360.)]
    fn test_dcf(#[case] from: NaiveDate, #[case] to: NaiveDate, #[case] expected: f64) {
        let dcf = Nl360.dcf(from, to).unwrap();

        assert_abs_diff_eq!(dcf, expected);
    }
}
