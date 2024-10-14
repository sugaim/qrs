use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, quantity::Yield};
use qmath::num::Real;

use super::super::YieldCurve;

// -----------------------------------------------------------------------------
// Flat
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Flat<V> {
    /// Flat yield(value, not a percent nor a bps) in Act/365F.
    pub rate: Yield<Act365f, V>,
}

//
// methods
//
impl<V: Real> YieldCurve for Flat<V> {
    type Value = V;

    #[inline]
    fn forward_rate(
        &self,
        _: &DateTime,
        _: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        Ok(self.rate.clone())
    }
}
