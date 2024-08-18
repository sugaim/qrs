use qmath::num::Real;

use crate::lnvol::{
    curve::{StrikeDer, VolCurve},
    LnCoord,
};

// -----------------------------------------------------------------------------
// VolCurveAdjust
// -----------------------------------------------------------------------------
pub trait VolCurveAdjust<V: Real> {
    fn adjusted_bsvol<S: VolCurve<Value = V>>(
        &self,
        slice: &S,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<V>;

    fn adjusted_bsvol_der<S: VolCurve<Value = V>>(
        &self,
        slice: &S,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<StrikeDer<V>>;
}
