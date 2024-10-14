use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, quantity::Yield};
use qmath::num::Real;

use crate::curve::YieldCurve;

use super::YieldCurveAdj;

// -----------------------------------------------------------------------------
// Bump
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Bump<C> {
    pub adjuster: C,
}

impl<C: YieldCurve, R: Real> YieldCurveAdj<R> for Bump<C>
where
    C::Value: Into<R>,
{
    #[inline]
    fn adjusted_forward_rate<Y: YieldCurve<Value = R>>(
        &self,
        curve: &Y,
        f: &DateTime,
        t: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, R>> {
        let base = curve.forward_rate(f, t)?;
        let adj = self.adjuster.forward_rate(f, t)?;

        Ok(Yield {
            day_count: Act365f,
            value: base.value + &adj.value.into(),
        })
    }
}
