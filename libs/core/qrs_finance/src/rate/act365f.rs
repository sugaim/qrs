use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use qrs_chrono::{Duration, Velocity};
use qrs_math::num::{Arithmetic, FloatBased, Real, RelPos, Scalar, Vector, Zero};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::daycount::Act365f;

use super::Rate;

// -----------------------------------------------------------------------------
// RateAct365f
//
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
pub struct RateAct365f<V>(V);

//
// display, serde
//
#[cfg(feature = "serde")]
impl<V: schemars::JsonSchema> schemars::JsonSchema for RateAct365f<V> {
    fn schema_name() -> String {
        format!("RateAct365f_for_{}", V::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_finance::rate::RateAct365f<{}>", V::schema_id()).into()
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
impl<V> RateAct365f<V> {
    /// Create a new `RateAct365F` instance with the given annual rate.
    ///
    /// Unit of the argument is 1. Not percent nor bps.
    /// Note that user must ensure that the given value is rate in Act/365F convention.
    #[inline]
    pub fn from_rate(value: V) -> Self {
        Self(value)
    }
}

impl<V: Scalar> RateAct365f<V> {
    /// Create a new rate object from change ratio and duration.
    ///
    /// Return `None` if the duration is zero.
    ///
    /// Since this class is based on Act/365F convention,
    /// duration is normalized with 365 days per year.
    #[inline]
    pub fn from_ratio(ratio: V, dur: Duration) -> Option<Self> {
        if dur.is_zero() {
            return None;
        }
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        let milsec = dur.millsecs() as f64;
        let dcf = V::nearest_value_of(milsec / MILSEC_PER_YEAR);
        Some(Self(ratio / &dcf))
    }
}

impl<V: Real> Rate for RateAct365f<V> {
    type Value = V;
    type Convention = Act365f;

    #[inline]
    fn convention(&self) -> Self::Convention {
        Act365f
    }

    #[inline]
    fn value(&self) -> Self::Value {
        self.0.clone()
    }
}

//
// operators
//
impl<V: FloatBased> FloatBased for RateAct365f<V> {
    type BaseFloat = V::BaseFloat;
}

impl<V: Arithmetic> Zero for RateAct365f<V> {
    #[inline]
    fn zero() -> Self {
        Self(V::zero())
    }

    #[inline]
    fn is_zero(&self) -> bool {
        self.0.is_zero()
    }
}

impl<V: Arithmetic> Neg for RateAct365f<V> {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl<V: Arithmetic> Add for RateAct365f<V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<V: Arithmetic> Add<&Self> for RateAct365f<V> {
    type Output = Self;

    #[inline]
    fn add(self, rhs: &Self) -> Self::Output {
        Self(self.0 + &rhs.0)
    }
}

impl<V: Arithmetic> AddAssign<&Self> for RateAct365f<V> {
    #[inline]
    fn add_assign(&mut self, rhs: &Self) {
        self.0 += &rhs.0;
    }
}

impl<V: Arithmetic> Sub<&Self> for RateAct365f<V> {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: &Self) -> Self::Output {
        Self(self.0 - &rhs.0)
    }
}

impl<V: Arithmetic> SubAssign<&Self> for RateAct365f<V> {
    #[inline]
    fn sub_assign(&mut self, rhs: &Self) {
        self.0 -= &rhs.0;
    }
}

impl<K: Arithmetic, V: Vector<K>> Mul<&K> for RateAct365f<V> {
    type Output = RateAct365f<V>;

    #[inline]
    fn mul(self, rhs: &K) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl<K: Arithmetic, V: Vector<K>> MulAssign<&K> for RateAct365f<V> {
    #[inline]
    fn mul_assign(&mut self, rhs: &K) {
        self.0 *= rhs;
    }
}

impl<K: Arithmetic, V: Vector<K>> Div<&K> for RateAct365f<V> {
    type Output = RateAct365f<V>;

    #[inline]
    fn div(self, rhs: &K) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl<K: Arithmetic, V: Vector<K>> DivAssign<&K> for RateAct365f<V> {
    #[inline]
    fn div_assign(&mut self, rhs: &K) {
        self.0 /= rhs;
    }
}

impl<V: Real> RelPos for RateAct365f<V> {
    type Output = V;

    #[inline]
    fn relpos_between(&self, left: &Self, right: &Self) -> Self::Output {
        let denom = right.0.clone() - &left.0;
        let nume = self.0.clone() - &left.0;
        nume / &denom
    }
}

impl<V: Real> Div<Duration> for RateAct365f<V> {
    type Output = Velocity<Self>;

    #[inline]
    fn div(self, rhs: Duration) -> Self::Output {
        Velocity::new(self, rhs)
    }
}

impl<V: FloatBased + Vector<V::BaseFloat>> Mul<Duration> for RateAct365f<V> {
    type Output = V;

    #[inline]
    fn mul(self, rhs: Duration) -> Self::Output {
        const MILSEC_PER_YEAR: f64 = 1000.0 * 60.0 * 60.0 * 24.0 * 365.0;
        let milsec = rhs.millsecs() as f64;
        let dcf = V::nearest_base_float_of(milsec / MILSEC_PER_YEAR);
        self.0 * &dcf
    }
}
