use std::sync::Arc;

use qrs_core::{
    chrono::{DateTime, Rate},
    num::{Exp, Real},
};

// -----------------------------------------------------------------------------
// YieldCurve
//
pub trait YieldCurve {
    type Value: Real;
    type Error;

    /// Calculate the forward rate.
    ///
    /// In terms of instant forward rate 'r',
    /// forward rate meant by this method is a average of instant forward rate over the period.
    ///
    /// When `from` is equal to `to`, the instant forward rate is returned.
    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> Rate<Self::Value>;

    /// Calculate the discount factor.
    ///
    /// implementations can be override the default implementation
    /// but relation between `forward_rate` and `discount` must be satisfied.
    #[inline]
    fn discount(&self, from: &DateTime, to: &DateTime) -> Self::Value {
        let rate = self.forward_rate(from, to);
        let exponent = -rate * (to - from);
        exponent.exp()
    }
}

impl<C: YieldCurve> YieldCurve for Arc<C> {
    type Value = C::Value;
    type Error = C::Error;

    #[inline]
    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> Rate<Self::Value> {
        self.as_ref().forward_rate(from, to)
    }
}
