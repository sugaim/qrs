use qmath::num::Real;

use crate::volsurf::slice::LnVolSlice;

// -----------------------------------------------------------------------------
// LnVolSliceAdj
// -----------------------------------------------------------------------------
pub trait LnVolSliceAdj<V: Real> {
    fn adjusted_lnvol<S: LnVolSlice<Value = V>>(
        &self,
        slice: &S,
        coord: &crate::volsurf::slice::LnCoord<V>,
    ) -> anyhow::Result<V>;

    fn adjusted_lnvol_der<S: LnVolSlice<Value = V>>(
        &self,
        slice: &S,
        coord: &crate::volsurf::slice::LnCoord<V>,
    ) -> anyhow::Result<crate::volsurf::slice::StrikeDer<V>>;
}
