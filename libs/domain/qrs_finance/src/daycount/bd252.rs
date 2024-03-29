use qrs_chrono::{Calendar, NaiveDate};
use qrs_math::num::Real;

use super::{Dcf, DcfError, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Bd252
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bd252 {
    pub cal: Calendar,
}

//
// methods
//
impl Dcf for Bd252 {
    #[inline]
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Result<f64, DcfError> {
        if to < from {
            let rev_dcf = self.dcf(to, from)?;
            return Err(DcfError::ReverseOrder { from, to, rev_dcf });
        }
        const DAYS_PER_YEAR: f64 = 252.;
        Ok(self.cal.num_bizdays(from..to)? as f64 / DAYS_PER_YEAR)
    }
}

impl RateDcf for Bd252 {
    type Rate<V: Real> = Bd252Rate<V>;

    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        Bd252Rate::from_rate(annual_rate, self.cal.clone())
    }
}

// -----------------------------------------------------------------------------
// Bd252Rate
//
#[derive(Debug, Clone, PartialEq)]
pub struct Bd252Rate<V> {
    rate: V,
    cal: Calendar,
}

//
// construction
//
impl<V> Bd252Rate<V> {
    #[inline]
    pub fn from_rate(rate: V, cal: Calendar) -> Self {
        Self { rate, cal }
    }
}

impl<V: Real> InterestRate for Bd252Rate<V> {
    type Convention = Bd252;
    type Value = V;

    #[inline]
    fn convention(&self) -> Self::Convention {
        Bd252 {
            cal: self.cal.clone(),
        }
    }

    #[inline]
    fn into_value(self) -> Self::Value {
        self.rate
    }
}

//
// operators
//
impl<K, V> std::ops::Mul<K> for Bd252Rate<V>
where
    V: std::ops::Mul<K, Output = V>,
{
    type Output = Bd252Rate<V>;

    #[inline]
    fn mul(self, rhs: K) -> Self::Output {
        Self::from_rate(self.rate * rhs, self.cal)
    }
}

impl<K, V> std::ops::MulAssign<K> for Bd252Rate<V>
where
    V: std::ops::MulAssign<K>,
{
    #[inline]
    fn mul_assign(&mut self, rhs: K) {
        self.rate *= rhs;
    }
}

impl<K, V> std::ops::Div<K> for Bd252Rate<V>
where
    V: std::ops::Div<K, Output = V>,
{
    type Output = Bd252Rate<V>;

    #[inline]
    fn div(self, rhs: K) -> Self::Output {
        Self::from_rate(self.rate / rhs, self.cal)
    }
}

impl<K, V> std::ops::DivAssign<K> for Bd252Rate<V>
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
    use rstest::rstest;

    use super::*;

    fn ymd(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn cal() -> Calendar {
        Calendar::builder()
            .with_valid_period(ymd(2000, 1, 1), ymd(2100, 12, 31))
            .with_extra_business_days(Default::default())
            .with_extra_holidays(vec![ymd(2021, 2, 3), ymd(2021, 2, 5)])
            .build()
            .unwrap()
    }

    fn cnv() -> Bd252 {
        Bd252 { cal: cal() }
    }

    #[rstest]
    #[case(ymd(2021, 1, 19), ymd(2021, 1, 22), Some(3. / 252.))] // only weekdays
    #[case(ymd(2021, 1, 22), ymd(2021, 1, 25), Some(1. / 252.))] // over weekends
    #[case(ymd(2021, 2, 1), ymd(2021, 2, 8), Some(3. / 252.))] // over holidays
    #[case(ymd(2021, 2, 3), ymd(2021, 2, 4), Some(0.))] // holiday
    #[case(ymd(1999, 12, 30), ymd(2000, 1, 3), None)] // out of valid period
    fn test_dcf(#[case] from: NaiveDate, #[case] to: NaiveDate, #[case] expected: Option<f64>) {
        let cnv = cnv();

        let dcf = cnv.dcf(from, to);
        let rev_dcf = cnv.dcf(to, from);

        match expected {
            Some(expected) => {
                assert_eq!(dcf.unwrap(), expected);
                let DcfError::ReverseOrder {
                    from: f,
                    to: t,
                    rev_dcf,
                } = rev_dcf.unwrap_err()
                else {
                    panic!("unexpected result");
                };
                assert_eq!(f, to);
                assert_eq!(t, from);
                assert_eq!(rev_dcf, expected);
            }
            None => {
                assert!(dcf.is_err());
                assert!(rev_dcf.is_err());
            }
        }
    }
}
