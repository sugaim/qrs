use qmath::num::Real;

use crate::volsurf::slice::{LnCoord, LnVolSlice, StrikeDer};

use super::Flat;

// -----------------------------------------------------------------------------
// LnVolSliceAtom
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(bound(deserialize = "V: serde::Deserialize<'de> + Real"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LnVolSliceAtom<V> {
    Flat(Flat<V>),
}

impl<V: Real> LnVolSlice for LnVolSliceAtom<V> {
    type Value = V;

    #[inline]
    fn lnvol(&self, coord: &LnCoord<V>) -> anyhow::Result<V> {
        match self {
            LnVolSliceAtom::Flat(flat) => flat.lnvol(coord),
        }
    }

    #[inline]
    fn lnvol_der(&self, coord: &LnCoord<V>) -> anyhow::Result<StrikeDer<V>> {
        match self {
            LnVolSliceAtom::Flat(flat) => flat.lnvol_der(coord),
        }
    }
}
