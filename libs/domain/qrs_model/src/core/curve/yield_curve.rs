use std::sync::Arc;

use qrs_chrono::DateTime;
use qrs_finance::core::daycount::{Act365fRate, InterestRate};
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
        let exponent = -rate.into_ratio_between(from, to);
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

// -----------------------------------------------------------------------------
// YieldCurveAdjust
//
pub trait YieldCurveAdjust<C: YieldCurve> {
    fn adjusted_forward_rate(
        &self,
        curve: &C,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<C::Value>>;
}
