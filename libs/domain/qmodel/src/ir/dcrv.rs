use std::{fmt::Debug, sync::Arc};

use qchrono::timepoint::DateTime;
use qfincore::{
    daycount::Act365f,
    quantity::{Ccy, Yield},
};
use qmath::num::Real;
use qproduct::Collateral;

use crate::curve::{
    adjust::Adj,
    atom::Atom,
    composite::{Composite, CompositeReq, CompositeSrc},
    CurveSrc, YieldCurve,
};

// -----------------------------------------------------------------------------
// DCrv
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub struct DCrv<V>(Arc<_Data<V>>);

#[derive(Debug, Clone, PartialEq)]
struct _Data<V> {
    ccy: Ccy,
    col: Collateral,
    crv: Composite<Arc<Atom<V>>, Adj<V>>,
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
// ResolveDCrv
// -----------------------------------------------------------------------------
pub trait DCrvSrc {
    type Value: Real;

    fn get_dcrv(&self, ccy: &Ccy, col: &Collateral) -> anyhow::Result<DCrv<Self::Value>>;
}

pub trait ResolveDCrv {
    type Value: Real;

    fn resolve_dcrv(
        &self,
        ccy: &Ccy,
        col: &Collateral,
    ) -> anyhow::Result<CompositeReq<Adj<Self::Value>>>;
}

impl<S> DCrvSrc for S
where
    S: ResolveDCrv,
    S: CurveSrc<Curve = Arc<Atom<S::Value>>>,
{
    type Value = S::Value;

    #[inline]
    fn get_dcrv(&self, ccy: &Ccy, col: &Collateral) -> anyhow::Result<DCrv<Self::Value>> {
        let req = self.resolve_dcrv(ccy, col)?;
        let crv = self.get_composite_curve(req)?;
        Ok(DCrv(Arc::new(_Data {
            ccy: *ccy,
            col: col.clone(),
            crv,
        })))
    }
}
