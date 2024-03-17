use std::ops::Neg;

use qrs_chrono::{Calendar, NaiveDate};
use qrs_math::num::Real;

use super::{Dcf, InterestRate, RateDcf};

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
    fn dcf(&self, from: NaiveDate, to: NaiveDate) -> Option<f64> {
        match from.cmp(&to) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => {
                if !self.cal.is_valid_for(from) || !self.cal.is_valid_for(to) {
                    return None;
                } else {
                    return Some(0.0);
                }
            }
            std::cmp::Ordering::Greater => return self.dcf(to, from).map(Neg::neg),
        };
        let num_bds = self.cal.num_bizdays(from, to)?;
        const DAYS_PER_YEAR: f64 = 252.;
        Some(num_bds as f64 / DAYS_PER_YEAR)
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    #[test]
    fn test_dcf() {
        let cnv = cnv();

        // only weekdays
        let from = ymd(2021, 1, 19);
        let to = ymd(2021, 1, 22);

        let dcf = cnv.dcf(from, to).unwrap();

        assert_eq!(dcf, 3.0 / 252.0);

        // over weekends
        let from = ymd(2021, 1, 22);
        let to = ymd(2021, 1, 25);

        let dcf = cnv.dcf(from, to).unwrap();

        assert_eq!(dcf, 1.0 / 252.0);

        // over holidays
        let from = ymd(2021, 2, 1);
        let to = ymd(2021, 2, 8);

        let dcf = cnv.dcf(from, to).unwrap();

        assert_eq!(dcf, 3.0 / 252.0); // Feb 3rd and 5th are holidays
    }
}
