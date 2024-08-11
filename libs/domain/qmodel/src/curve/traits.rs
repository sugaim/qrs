use std::sync::Arc;

use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Yield};
use qmath::num::{Exp, Real};

// -----------------------------------------------------------------------------
// YieldCurve
// -----------------------------------------------------------------------------
pub trait YieldCurve {
    type Value: Real;

    /// Calculate the forward rate between two dates.
    /// When 'from' is equivalent to 'to', implementations must return a short rate.
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>>;

    /// Calculate the discount factor between two dates.
    #[inline]
    fn discount(&self, from: &DateTime, to: &DateTime) -> anyhow::Result<Self::Value> {
        let yld = self.forward_rate(from, to)?;
        let ratio = yld.to_ratio(from, to).expect("Act365f should not fail");
        Ok((-ratio).exp())
    }
}

impl<C: YieldCurve> YieldCurve for Box<C> {
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        self.as_ref().forward_rate(from, to)
    }
}

impl<C: YieldCurve> YieldCurve for Arc<C> {
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        self.as_ref().forward_rate(from, to)
    }
}
