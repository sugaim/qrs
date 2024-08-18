use qmath::num::Real;

use crate::lnvol::{
    curve::{StrikeDer, VolCurve},
    LnCoord,
};

use super::VolCurveAdjust;

// -----------------------------------------------------------------------------
// Bump
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Bump<S> {
    pub adjster: S,
}

impl<V: Real, S: VolCurve<Value = V>> VolCurveAdjust<V> for Bump<S> {
    #[inline]
    fn adjusted_bsvol<AS: VolCurve<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<V> {
        let base = self.adjster.bs_totalvol(coord)?;
        let adj = slice.bs_totalvol(coord)?;
        Ok(base + &adj)
    }

    #[inline]
    fn adjusted_bsvol_der<AS: VolCurve<Value = V>>(
        &self,
        slice: &AS,
        coord: &LnCoord<V>,
    ) -> anyhow::Result<StrikeDer<V>> {
        let base = self.adjster.bsvol_der(coord)?;
        let adj = slice.bsvol_der(coord)?;

        Ok(StrikeDer {
            vol: (base.vol.sqrt() + &adj.vol.sqrt()).powi(2),
            dvdy: base.dvdy + &adj.dvdy,
            d2vdy2: base.d2vdy2 + &adj.d2vdy2,
        })
    }
}
