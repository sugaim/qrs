use std::sync::{Arc, Mutex};

use qrs_chrono::DateTime;
use qrs_finance::daycount::Act365fRate;
use qrs_math::num::{Exp, Real};

// -----------------------------------------------------------------------------
// YieldCurve
//
pub trait YieldCurve {
    type Value: Real;

    /// Calculate the forward rate.
    ///
    /// In terms of instant forward rate 'r',
    /// forward rate meant by this method is a average of instant forward rate over the period.
    ///
    /// When `from` is equal to `to`, the instant forward rate is returned.
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>>;

    /// Calculate the discount factor.
    ///
    /// implementations can be override the default implementation
    /// but relation between `forward_rate` and `discount` must be satisfied.
    ///
    /// Note that this just obeys the `forward_rate` method.
    /// Hence, even if the concrete implementation has `today`,
    /// `discount` method should not make rate zero before `today`.
    /// If you want to do so, `forward_rate` should return zero rate before `today`.
    #[inline]
    fn discount(&self, from: &DateTime, to: &DateTime) -> anyhow::Result<Self::Value> {
        let rate = self.forward_rate(from, to)?;
        let exponent = -rate * (to - from);
        Ok(exponent.exp())
    }
}

impl<C: YieldCurve> YieldCurve for Arc<C> {
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        self.as_ref().forward_rate(from, to)
    }
}

impl<C: YieldCurve> YieldCurve for Mutex<C> {
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        self.lock().unwrap().forward_rate(from, to)
    }
}

// -----------------------------------------------------------------------------
// YieldCurveAdjust
//
pub trait YieldCurveAdjust<V: Real> {
    fn adjusted_forward_rate<C: YieldCurve<Value = V>>(
        &self,
        curve: &C,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<V>>;
}
