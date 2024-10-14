use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, quantity::Yield};
use qmath::num::Real;

use crate::curve::YieldCurve;

// -----------------------------------------------------------------------------
// YieldCurveAdj
// -----------------------------------------------------------------------------
pub trait YieldCurveAdj<V: Real> {
    /// Calculate adjusted forward rate between two dates.
    fn adjusted_forward_rate<Y: YieldCurve<Value = V>>(
        &self,
        curve: &Y,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, V>>;
}

impl<V: Real> YieldCurveAdj<V> for () {
    #[inline]
    fn adjusted_forward_rate<Y: YieldCurve<Value = V>>(
        &self,
        curve: &Y,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, V>> {
        curve.forward_rate(from, to)
    }
}
