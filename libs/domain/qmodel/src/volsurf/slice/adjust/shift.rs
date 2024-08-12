use qmath::num::Real;

use crate::volsurf::slice::{LnCoord, LnVolSlice, StrikeDer};

use super::LnVolSliceAdj;

// -----------------------------------------------------------------------------
// LnCoordShift
// -----------------------------------------------------------------------------
/// Coordinate shifter along strike dimension.
///
/// Note that this shifts vol slices rather than coordinates.
/// Hence, shift is implemented with negative sign, 'f(x - shift)'.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LnCoordShift<V> {
    pub shift: V,
}

impl<V: Real> LnVolSliceAdj<V> for LnCoordShift<V> {
    #[inline]
    fn adjusted_lnvol<AS: LnVolSlice<Value = V>>(
        &self,
        slice: &AS,
        coord: &crate::volsurf::slice::LnCoord<V>,
    ) -> anyhow::Result<V> {
        let coord = LnCoord(coord.0.clone() - &self.shift);
        slice.lnvol(&coord)
    }

    #[inline]
    fn adjusted_lnvol_der<AS: LnVolSlice<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<StrikeDer<V>> {
        let coord = LnCoord(coord.0.clone() - &self.shift);
        slice.lnvol_der(&coord)
    }
}
