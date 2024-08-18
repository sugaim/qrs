use crate::lnvol::{
    curve::{adjust::VolCurveAdjust, StrikeDer, VolCurve},
    LnCoord,
};

// -----------------------------------------------------------------------------
// Adjusted
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Adjusted<S, A> {
    pub base: S,
    pub adjust: Vec<A>,
}

impl<S: VolCurve, A: VolCurveAdjust<S::Value>> VolCurve for Adjusted<S, A> {
    type Value = S::Value;

    #[inline]
    fn bs_totalvol(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<Self::Value> {
        let slice = _Adj {
            base: &self.base,
            adjust: &self.adjust,
        };
        slice.bs_totalvol(coord)
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<StrikeDer<Self::Value>> {
        let slice = _Adj {
            base: &self.base,
            adjust: &self.adjust,
        };
        slice.bsvol_der(coord)
    }
}

struct _Adj<'a, S, A> {
    base: &'a S,
    adjust: &'a [A],
}

impl<'a, S: VolCurve, A: VolCurveAdjust<S::Value>> VolCurve for _Adj<'a, S, A> {
    type Value = S::Value;

    #[inline]
    fn bs_totalvol(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<Self::Value> {
        match self.adjust.split_last() {
            Some((last, rest)) => {
                let slice = _Adj {
                    base: self.base,
                    adjust: rest,
                };
                last.adjusted_bsvol(&slice, coord)
            }
            None => self.base.bs_totalvol(coord),
        }
    }

    #[inline]
    fn bsvol_der(&self, coord: &LnCoord<Self::Value>) -> anyhow::Result<StrikeDer<Self::Value>> {
        match self.adjust.split_last() {
            Some((last, rest)) => {
                let slice = _Adj {
                    base: self.base,
                    adjust: rest,
                };
                last.adjusted_bsvol_der(&slice, coord)
            }
            None => self.base.bsvol_der(coord),
        }
    }
}
