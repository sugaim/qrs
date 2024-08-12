use crate::volsurf::slice::{adj::LnVolSliceAdj, LnVolSlice};

// -----------------------------------------------------------------------------
// Adjusted
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Adjusted<S, A> {
    pub base: S,
    pub adjust: Vec<A>,
}

impl<S: LnVolSlice, A: LnVolSliceAdj<S::Value>> LnVolSlice for Adjusted<S, A> {
    type Value = S::Value;

    #[inline]
    fn lnvol(
        &self,
        coord: &crate::volsurf::slice::LnCoord<Self::Value>,
    ) -> anyhow::Result<Self::Value> {
        let slice = _Adj {
            base: &self.base,
            adjust: &self.adjust,
        };
        slice.lnvol(coord)
    }

    #[inline]
    fn lnvol_der(
        &self,
        coord: &crate::volsurf::slice::LnCoord<Self::Value>,
    ) -> anyhow::Result<crate::volsurf::slice::StrikeDer<Self::Value>> {
        let slice = _Adj {
            base: &self.base,
            adjust: &self.adjust,
        };
        slice.lnvol_der(coord)
    }
}

struct _Adj<'a, S, A> {
    base: &'a S,
    adjust: &'a [A],
}

impl<'a, S: LnVolSlice, A: LnVolSliceAdj<S::Value>> LnVolSlice for _Adj<'a, S, A> {
    type Value = S::Value;

    #[inline]
    fn lnvol(
        &self,
        coord: &crate::volsurf::slice::LnCoord<Self::Value>,
    ) -> anyhow::Result<Self::Value> {
        match self.adjust.split_last() {
            Some((last, rest)) => {
                let slice = _Adj {
                    base: self.base,
                    adjust: rest,
                };
                last.adjusted_lnvol(&slice, coord)
            }
            None => self.base.lnvol(coord),
        }
    }

    #[inline]
    fn lnvol_der(
        &self,
        coord: &crate::volsurf::slice::LnCoord<Self::Value>,
    ) -> anyhow::Result<crate::volsurf::slice::StrikeDer<Self::Value>> {
        match self.adjust.split_last() {
            Some((last, rest)) => {
                let slice = _Adj {
                    base: self.base,
                    adjust: rest,
                };
                last.adjusted_lnvol_der(&slice, coord)
            }
            None => self.base.lnvol_der(coord),
        }
    }
}
