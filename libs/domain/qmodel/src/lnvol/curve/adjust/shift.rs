use qmath::num::Real;

use crate::lnvol::{
    curve::{StrikeDer, VolCurve},
    LnCoord,
};

use super::VolCurveAdjust;

// -----------------------------------------------------------------------------
// Shift
// -----------------------------------------------------------------------------
/// Coordinate shifter along strike dimension.
///
/// Note that this shifts vol slices rather than coordinates.
/// Hence, shift is implemented with negative sign, 'f(x - shift)'.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Shift<V> {
    pub shift: V,
}

impl<V: Real> VolCurveAdjust<V> for Shift<V> {
    #[inline]
    fn adjusted_bsvol<AS: VolCurve<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<V> {
        let coord = LnCoord(coord.0.clone() - &self.shift);
        slice.bs_totalvol(&coord)
    }

    #[inline]
    fn adjusted_bsvol_der<AS: VolCurve<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<StrikeDer<V>> {
        let coord = LnCoord(coord.0.clone() - &self.shift);
        slice.bsvol_der(&coord)
    }
}
