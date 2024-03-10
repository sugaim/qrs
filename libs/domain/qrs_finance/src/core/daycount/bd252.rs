use chrono::Timelike;
use qrs_chrono::{Calendar, DateTime, Tz};
use qrs_math::num::Real;

use super::{Dcf, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Bd252
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bd252 {
    pub cal: Calendar,
    pub tz: Tz,
}

//
// methods
//
impl Dcf for Bd252 {
    fn dcf(&self, from: &DateTime, to: &DateTime) -> f64 {
        let from = from.with_timezone(&self.tz);
        let to = to.with_timezone(&self.tz);
        let from_t = from.time();
        let to_t = to.time();

        let num_bds = self.cal.num_bizdays(from.date(), to.date());
        let millsecs = (num_bds as f64 * 1000.0 * 60.0 * 60.0 * 24.0)
            + (to_t.num_seconds_from_midnight() as f64 * 1000.0
                + (to_t.nanosecond() % 1_000_000 / 1_000) as f64)
            - (from_t.num_seconds_from_midnight() as f64 * 1000.0
                + (from_t.nanosecond() % 1_000_000 / 1_000) as f64);

        const MILLSECS_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 252.0;
        millsecs / MILLSECS_PER_YEAR
    }
}

impl RateDcf for Bd252 {
    type Rate<V: Real> = Bd252Rate<V>;

    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        Bd252Rate::from_rate(annual_rate, self.cal.clone(), self.tz)
    }
}

// -----------------------------------------------------------------------------
// Bd252Rate
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bd252Rate<V> {
    rate: V,
    cal: Calendar,
    tz: Tz,
}

//
// construction
//
impl<V> Bd252Rate<V> {
    #[inline]
    pub fn from_rate(rate: V, cal: Calendar, tz: Tz) -> Self {
        Self { rate, cal, tz }
    }
}

impl<V: Real> InterestRate for Bd252Rate<V> {
    type Convention = Bd252;
    type Value = V;

    #[inline]
    fn convention(&self) -> Self::Convention {
        Bd252 {
            cal: self.cal.clone(),
            tz: self.tz,
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
        Self::from_rate(self.rate * rhs, self.cal, self.tz)
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
        Self::from_rate(self.rate / rhs, self.cal, self.tz)
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
