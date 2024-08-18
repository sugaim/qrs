use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Yield};
use qmath::num::Real;

use crate::curve::{atom::Atom, YieldCurve};

use super::{Bump, Lookback, YieldCurveAdj};

// -----------------------------------------------------------------------------
// Adj
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Adj<V> {
    Bump(Bump<Atom<V>>),
    Lookback(Lookback),
}

impl<V: Real> YieldCurveAdj<V> for Adj<V> {
    #[inline]
    fn adjusted_forward_rate<Y: YieldCurve<Value = V>>(
        &self,
        curve: &Y,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, V>> {
        match self {
            Adj::Bump(adj) => adj.adjusted_forward_rate(curve, from, to),
            Adj::Lookback(adj) => adj.adjusted_forward_rate(curve, from, to),
        }
    }
}
