use chrono::{Datelike, NaiveDate};
use qrs_chrono::{DateExtensions, Tz};
use qrs_math::num::Real;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Dcf, InterestRate, RateDcf};

// -----------------------------------------------------------------------------
// Nl365
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Nl365 {
    #[serde(rename = "timezone")]
    pub tz: Tz,
}

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
    fn dcf(&self, from: &qrs_chrono::DateTime, to: &qrs_chrono::DateTime) -> f64 {
        match from.cmp(to) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => return 0.0,
            std::cmp::Ordering::Greater => return -self.dcf(to, from),
        };
        let from = from.with_timezone(&self.tz);
        let to = to.with_timezone(&self.tz);

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
        const MILSEC_PER_DAY: f64 = 1000.0 * 60.0 * 60.0 * 24.0;
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 360.0;
        ((to - from).millsecs() as f64 - leap_days as f64 * MILSEC_PER_DAY) / MILSEC_PER_YEAR
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
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct Nl365Rate<V> {
    rate: V,
    cnv: Nl365,
}

//
// display, serde
//
impl<V: schemars::JsonSchema> schemars::JsonSchema for Nl365Rate<V> {
    fn schema_name() -> String {
        format!("RateNl365_for_{}", V::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_finance::daycount::RateNl365<{}>", V::schema_id()).into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = V::json_schema(gen);
        if let schemars::schema::Schema::Object(ref mut schema) = schema {
            schema.metadata().description = Some(
                "Annual rate with NL/365 convention. Unit is 1. Not percentage nor bps."
                    .to_string(),
            );
        }
        schema
    }
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
