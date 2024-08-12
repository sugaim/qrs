use qmath::num::Real;

use crate::volsurf::slice::{LnCoord, LnVolSlice, StrikeDer};

use super::LnVolSliceAdj;

// -----------------------------------------------------------------------------
// VolBump
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct VolBump<S> {
    pub adjster: S,
}

impl<V: Real, S: LnVolSlice<Value = V>> LnVolSliceAdj<V> for VolBump<S> {
    #[inline]
    fn adjusted_lnvol<AS: LnVolSlice<Value = V>>(
        &self,
        slice: &AS,
        coord: &crate::volsurf::slice::LnCoord<V>,
    ) -> anyhow::Result<V> {
        let base = self.adjster.lnvol(coord)?;
        let adj = slice.lnvol(coord)?;
        Ok(base + &adj)
    }

    #[inline]
    fn adjusted_lnvol_der<AS: LnVolSlice<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<StrikeDer<V>> {
        let base = self.adjster.lnvol_der(coord)?;
        let adj = slice.lnvol_der(coord)?;

        Ok(StrikeDer {
            vol: base.vol + &adj.vol,
            dvdy: base.dvdy + &adj.dvdy,
            d2vdy2: base.d2vdy2 + &adj.d2vdy2,
        })
    }
}
