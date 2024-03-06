use std::ops::{Div, Mul, MulAssign};

use chrono::Datelike;
use qrs_chrono::{Duration, Velocity};
use qrs_math::num::Real;

use super::{DayCount, DayCountRate, Rate, _ops::define_vector_behavior};

// -----------------------------------------------------------------------------
// NL360
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NL360;

impl Default for NL360 {
    #[inline]
    fn default() -> Self {
        Self
    }
}

//
// display, serde
//
impl std::fmt::Display for NL360 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NL/360")
    }
}

//
// methods
//
impl DayCount for NL360 {
    fn dcf(&self, from: &qrs_chrono::DateTime, to: &qrs_chrono::DateTime) -> f64 {
        match from.cmp(to) {
            std::cmp::Ordering::Less => {}
            std::cmp::Ordering::Equal => return 0.0,
            std::cmp::Ordering::Greater => return -self.dcf(to, from),
        };
        if to < from {
            return -self.dcf(to, from);
        }

        const MILSEC_PER_DAY: f64 = 1000.0 * 60.0 * 60.0 * 24.0;
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 360.0;
        let is_leap_year = |year: i32| year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);

        let mut leap_days = 0;
        let from_year = from.year();
        let to_year = to.year();
        if from_year == to_year && is_leap_year(from_year) {
            if (from.month() <= 2) && (2 < to.month()) {
                leap_days += 1;
            }
        } else {
            if is_leap_year(from_year) && (from.month() <= 2) {
                leap_days += 1;
            }
            if is_leap_year(to_year) && (2 < to.month()) {
                leap_days += 1;
            }
        }
        for year in (from_year + 1)..to_year {
            if is_leap_year(year) {
                leap_days += 1;
            }
        }
        ((to - from).millsecs() as f64 - leap_days as f64 * MILSEC_PER_DAY) / MILSEC_PER_YEAR
    }
}

impl DayCountRate for NL360 {
    type Rate<V: Real> = RateNL360<V>;

    /// Create a Act360F rate from the given annual rate.
    /// Note that the unit of the argument is 1. Not percent nor bps.
    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        RateNL360::from_rate(annual_rate)
    }
}

// -----------------------------------------------------------------------------
// RateNL360
//
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RateNL360<V>(V);

//
// display, serde
//
#[cfg(feature = "serde")]
impl<V: schemars::JsonSchema> schemars::JsonSchema for RateNL360<V> {
    fn schema_name() -> String {
        format!("RateNL360_for_{}", V::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_finance::daycount::RateNL360<{}>", V::schema_id()).into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = V::json_schema(gen);
        if let schemars::schema::Schema::Object(ref mut schema) = schema {
            schema.metadata().description = Some(
                "Annual rate with NL/360 convention. Unit is 1. Not percentage nor bps."
                    .to_string(),
            );
        }
        schema
    }
}

//
// methods
//
impl<V> RateNL360<V> {
    /// Create a new `RateNL360` instance with the given annual rate.
    ///
    /// Unit of the argument is 1. Not percent nor bps.
    /// Note that user must ensure that the given value is rate in NL/360F convention.
    #[inline]
    pub fn from_rate(value: V) -> Self {
        Self(value)
    }
}

impl<V: Real> Rate for RateNL360<V> {
    type Value = V;
    type Convention = NL360;

    #[inline]
    fn convention(&self) -> Self::Convention {
        NL360
    }

    #[inline]
    fn into_value(self) -> Self::Value {
        self.0
    }
}

//
// operators
//
define_vector_behavior!(RateNL360);
