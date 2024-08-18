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
    fn bs_totalvol(&self, coord: &LnCoord<V>) -> anyhow::Result<V> {
        match self {
            Atom::Flat(flat) => flat.bs_totalvol(coord),
            Atom::Svi(svi) => svi.bs_totalvol(coord),
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
