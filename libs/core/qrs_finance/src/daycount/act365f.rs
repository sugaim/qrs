use std::ops::{Div, Mul, MulAssign};

use qrs_chrono::{Duration, Velocity};
use qrs_math::num::{FloatBased, Real, RelPos, Vector};

use super::{DayCount, DayCountRate, Rate, _ops::define_vector_behavior};

// -----------------------------------------------------------------------------
// Act365f
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Act365f;

impl Default for Act365f {
    #[inline]
    fn default() -> Self {
        Self
    }
}

//
// display, serde
//
impl std::fmt::Display for Act365f {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Act/365f")
    }
}

//
// methods
//
impl DayCount for Act365f {
    #[inline]
    fn dcf(&self, from: &qrs_chrono::DateTime, to: &qrs_chrono::DateTime) -> f64 {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        (to - from).millsecs() as f64 / MILSEC_PER_YEAR
    }
}

impl DayCountRate for Act365f {
    type Rate<V: Real> = Act365fRate<V>;

    /// Create a Act365F rate from the given annual rate.
    /// Note that the unit of the argument is 1. Not percent nor bps.
    #[inline]
    fn to_rate<V: Real>(&self, annual_rate: V) -> Self::Rate<V> {
        Act365fRate::from_rate(annual_rate)
    }
}

// -----------------------------------------------------------------------------
// Act365fRate
//
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Act365fRate<V>(V);

//
// display, serde
//
#[cfg(feature = "serde")]
impl<V: schemars::JsonSchema> schemars::JsonSchema for Act365fRate<V> {
    fn schema_name() -> String {
        format!("Act365fRate_for_{}", V::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_finance::daycount::Act365fRate<{}>", V::schema_id()).into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        let mut schema = V::json_schema(gen);
        if let schemars::schema::Schema::Object(ref mut schema) = schema {
            schema.metadata().description = Some(
                "Annual rate with Act/365 fixed convention. Unit is 1. Not percentage nor bps."
                    .to_string(),
            );
        }
        schema
    }
}

//
// methods
//
impl<V> Act365fRate<V> {
    /// Create a new `Act365fRate` instance with the given annual rate.
    ///
    /// Unit of the argument is 1. Not percent nor bps.
    /// Note that user must ensure that the given value is rate in Act/365F convention.
    #[inline]
    pub fn from_rate(value: V) -> Self {
        Self(value)
    }
}

impl<V: Real> Rate for Act365fRate<V> {
    type Value = V;
    type Convention = Act365f;

    #[inline]
    fn convention(&self) -> Self::Convention {
        Act365f
    }

    #[inline]
    fn into_value(self) -> Self::Value {
        self.0
    }
}

//
// operators
//
define_vector_behavior!(Act365fRate);

impl<V: Real> RelPos for Act365fRate<V> {
    type Output = V;

    #[inline]
    fn relpos_between(&self, left: &Self, right: &Self) -> Self::Output {
        let denom = right.0.clone() - &left.0;
        let nume = self.0.clone() - &left.0;
        nume / &denom
    }
}

impl<V: FloatBased + Vector<V::BaseFloat>> Mul<Duration> for Act365fRate<V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: Duration) -> Self::Output {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        let milsec = rhs.millsecs() as f64;
        let dcf = V::nearest_base_float_of(milsec / MILSEC_PER_YEAR);
        self.0 * &dcf
    }
}
