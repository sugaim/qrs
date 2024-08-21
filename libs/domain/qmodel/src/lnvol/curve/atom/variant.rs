use qfincore::{daycount::Act365f, Volatility};
use qmath::num::Real;

use crate::lnvol::{
    curve::{StrikeDer, VolCurve},
    LnCoord,
};

use super::{svi::Svi, Flat};

// -----------------------------------------------------------------------------
// Atom
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(bound(deserialize = "V: serde::Deserialize<'de> + Real"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Atom<V> {
    Flat(Flat<V>),
    Svi(Svi<V>),
}

impl<V: Real> VolCurve for Atom<V> {
    type Value = V;

    #[inline]
    fn bsvol(
        &self,
        coord: &LnCoord<Self::Value>,
    ) -> anyhow::Result<Volatility<Act365f, Self::Value>> {
        match self {
            Atom::Flat(flat) => flat.bsvol(coord),
            Atom::Svi(svi) => svi.bsvol(coord),
        }
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<V>) -> anyhow::Result<StrikeDer<V>> {
        match self {
            Atom::Flat(flat) => flat.bsvol_der(coord),
            Atom::Svi(svi) => svi.bsvol_der(coord),
        }
    }
}
