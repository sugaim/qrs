use qfincore::{daycount::Act365f, Volatility};
use qmath::num::Real;

use crate::lnvol::{
    curve::{adjust::VolCurveAdjust, atom::Atom, StrikeDer, VolCurve},
    LnCoord,
};

use super::{Adjusted, Scaled, Weighted};

// -----------------------------------------------------------------------------
// Composite
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(bound(deserialize = "V: serde::Deserialize<'de> + Real, Adj: serde::Deserialize<'de>"))]
pub enum Composite<V, Adj> {
    Atom(Atom<V>),
    Adjusted(Adjusted<Box<Self>, Adj>),
    Scaled(Scaled<Box<Self>, V>),
    Weighted(Weighted<Box<Self>>),
}

impl<V, Adj> VolCurve for Composite<V, Adj>
where
    V: Real,
    Adj: VolCurveAdjust<V>,
{
    type Value = V;

    #[inline]
    fn bsvol(
        &self,
        coord: &LnCoord<Self::Value>,
    ) -> anyhow::Result<Volatility<Act365f, Self::Value>> {
        match &self {
            Composite::Atom(atom) => atom.bsvol(coord),
            Composite::Adjusted(adj) => adj.bsvol(coord),
            Composite::Scaled(scaled) => scaled.bsvol(coord),
            Composite::Weighted(weighted) => weighted.bsvol(coord),
        }
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<StrikeDer<Self::Value>> {
        match self {
            Composite::Atom(atom) => atom.bsvol_der(coord),
            Composite::Adjusted(adj) => adj.bsvol_der(coord),
            Composite::Scaled(scaled) => scaled.bsvol_der(coord),
            Composite::Weighted(weighted) => weighted.bsvol_der(coord),
        }
    }
}
