use qfincore::{daycount::Act365f, Volatility};
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
    ) -> anyhow::Result<Volatility<Act365f, V>> {
        let base = self.adjster.bsvol(coord)?;
        let adj = slice.bsvol(coord)?;
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
            vol: base.vol + &adj.vol,
            dvdy: base.dvdy + &adj.dvdy,
            d2vdy2: base.d2vdy2 + &adj.d2vdy2,
        })
    }
}
