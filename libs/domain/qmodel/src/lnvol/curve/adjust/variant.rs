use qmath::num::Real;

use crate::lnvol::{
    curve::{atom::Atom, StrikeDer, VolCurve},
    LnCoord,
};

use super::{shift::Shift, Bump, VolCurveAdjust};

// -----------------------------------------------------------------------------
// Adj
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(bound(deserialize = "V: serde::Deserialize<'de> + Real"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Adj<V> {
    Bump(Bump<Atom<V>>),
    Shift(Shift<V>),
}

impl<V: Real> VolCurveAdjust<V> for Adj<V> {
    #[inline]
    fn adjusted_bsvol<AS: VolCurve<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<V> {
        match self {
            Adj::Bump(adj) => adj.adjusted_bsvol(slice, coord),
            Adj::Shift(adj) => adj.adjusted_bsvol(slice, coord),
        }
    }

    #[inline]
    fn adjusted_bsvol_der<AS: VolCurve<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<StrikeDer<V>> {
        match self {
            Adj::Bump(adj) => adj.adjusted_bsvol_der(slice, coord),
            Adj::Shift(adj) => adj.adjusted_bsvol_der(slice, coord),
        }
    }
}
