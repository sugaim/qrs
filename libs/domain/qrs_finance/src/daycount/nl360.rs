use qrs_chrono::{DateExtensions, Datelike, NaiveDate};
use qrs_math::num::Real;

use super::{Dcf, DcfError, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Nl360
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Nl360;

//
// methods
//
impl Dcf for Nl360 {
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Result<f64, DcfError> {
        if to < from {
            let rev_dcf = self.dcf(to, from)?;
            return Err(DcfError::ReverseOrder { from, to, rev_dcf });
        }
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
        Ok(((to - from).num_days() as f64 - leap_days as f64) / DAYS_PER_YEAR)
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
    #[case(ymd(2021, 1, 1), ymd(2021, 1, 31))]
    #[case(ymd(2020, 2, 1), ymd(2020, 2, 28))]
    #[case(ymd(2020, 2, 1), ymd(2020, 2, 29))]
    #[case(ymd(2020, 2, 1), ymd(2020, 3, 1))]
    #[case(ymd(2020, 2, 28), ymd(2020, 3, 1))]
    #[case(ymd(2020, 2, 29), ymd(2020, 3, 1))]
    #[case(ymd(2020, 3, 1), ymd(2020, 3, 31))]
    #[case(ymd(2020, 2, 1), ymd(2024, 4, 1))]
    #[case(ymd(2020, 2, 1), ymd(2024, 3, 1))]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 29))]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 28))]
    #[case(ymd(2020, 2, 1), ymd(2024, 2, 1))]
    #[case(ymd(2020, 2, 28), ymd(2024, 2, 1))]
    #[case(ymd(2020, 2, 29), ymd(2024, 2, 1))]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 1))]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 28))]
    #[case(ymd(2020, 3, 1), ymd(2024, 2, 29))]
    #[case(ymd(2020, 3, 1), ymd(2024, 3, 1))]
    fn test_dcf(#[case] from: NaiveDate, #[case] to: NaiveDate) {
        let expected = from
            .iter_days()
            .take_while(|d| *d < to)
            .filter(|d| !d.is_leap_day())
            .count() as f64
            / 360.;

        let dcf = Nl360.dcf(from, to).unwrap();
        let rev_dcf = Nl360.dcf(to, from).unwrap_err();

        assert_abs_diff_eq!(dcf, expected, epsilon = 1e-10);
        let DcfError::ReverseOrder {
            from: f,
            to: t,
            rev_dcf,
        } = rev_dcf
        else {
            panic!("unexpected result: {:?}", rev_dcf);
        };
        assert_eq!(f, to);
        assert_eq!(t, from);
        assert_abs_diff_eq!(rev_dcf, expected, epsilon = 1e-10);
    }
}
