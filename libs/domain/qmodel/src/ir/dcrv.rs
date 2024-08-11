use std::{fmt::Debug, sync::Arc};

use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Ccy, Yield};
use qmath::num::Real;
use qproduct::Collateral;

use crate::curve::{adj::Adj, atom::Atom, Curve, CurveReq, CurveSrc, CurveSrcInduce, YieldCurve};

// -----------------------------------------------------------------------------
// DCrv
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub struct DCrv<V>(Arc<_Data<V>>);

#[derive(Debug, Clone, PartialEq)]
struct _Data<V> {
    ccy: Ccy,
    col: Collateral,
    crv: Curve<Arc<Atom<V>>, Adj<V>>,
}

//
// methods
//
impl<V: Real> YieldCurve for DCrv<V> {
    type Value = V;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        self.0.crv.forward_rate(from, to)
    }
}

// -----------------------------------------------------------------------------
// DCrvSrc
// DCrvSrcInduce
// -----------------------------------------------------------------------------
pub trait DCrvSrc {
    type Value: Real;

    fn get_dcrv(&self, ccy: &Ccy, col: &Collateral) -> anyhow::Result<DCrv<Self::Value>>;
}

pub trait DCrvSrcInduce: CurveSrcInduce<AtomCurve = Arc<Atom<Self::Value>>> {
    type Value: Real;

    fn resolve_dcrv_req(
        &self,
        ccy: &Ccy,
        col: &Collateral,
    ) -> anyhow::Result<CurveReq<Adj<Self::Value>>>;
}

impl<S: DCrvSrcInduce> DCrvSrc for S {
    type Value = S::Value;

    #[inline]
    fn get_dcrv(&self, ccy: &Ccy, col: &Collateral) -> anyhow::Result<DCrv<Self::Value>> {
        let req = self.resolve_dcrv_req(ccy, col)?;
        let crv = self.get_curve(req)?;
        Ok(DCrv(Arc::new(_Data {
            ccy: *ccy,
            col: col.clone(),
            crv,
        })))
    }
}
