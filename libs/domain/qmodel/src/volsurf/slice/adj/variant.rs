use qmath::num::Real;

use crate::volsurf::slice::{atom::LnVolSliceAtom, LnCoord, LnVolSlice, StrikeDer};

use super::{shift::LnCoordShift, LnVolSliceAdj, VolBump};

// -----------------------------------------------------------------------------
// LnVolSliceAdjVariant
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(bound(deserialize = "V: serde::Deserialize<'de> + Real"))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LnVolSliceAdjVariant<V> {
    Bump(VolBump<LnVolSliceAtom<V>>),
    Shift(LnCoordShift<V>),
}

impl<V: Real> LnVolSliceAdj<V> for LnVolSliceAdjVariant<V> {
    #[inline]
    fn adjusted_lnvol<AS: LnVolSlice<Value = V>>(
        &self,
        slice: &AS,
        coord: &crate::volsurf::slice::LnCoord<V>,
    ) -> anyhow::Result<V> {
        match self {
            LnVolSliceAdjVariant::Bump(adj) => adj.adjusted_lnvol(slice, coord),
            LnVolSliceAdjVariant::Shift(adj) => adj.adjusted_lnvol(slice, coord),
        }
    }

    #[inline]
    fn adjusted_lnvol_der<AS: LnVolSlice<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<StrikeDer<V>> {
        match self {
            LnVolSliceAdjVariant::Bump(adj) => adj.adjusted_lnvol_der(slice, coord),
            LnVolSliceAdjVariant::Shift(adj) => adj.adjusted_lnvol_der(slice, coord),
        }
    }
}
