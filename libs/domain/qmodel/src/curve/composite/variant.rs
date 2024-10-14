use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, quantity::Yield};

use crate::curve::{adjust::YieldCurveAdj, YieldCurve};

use super::{Adjusted, Joint, Weighted};

// -----------------------------------------------------------------------------
// Composite
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq)]
pub enum Composite<C, Adj = ()> {
    Atom(C),
    Adjusted(Adjusted<Box<Self>, Adj>),
    Joint(Joint<Box<Self>>),
    Weighted(Weighted<Box<Self>>),
}

impl<C: YieldCurve, Adj: YieldCurveAdj<C::Value>> YieldCurve for Composite<C, Adj> {
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        match self {
            Composite::Atom(curve) => curve.forward_rate(from, to),
            Composite::Adjusted(adj) => adj.forward_rate(from, to),
            Composite::Joint(joint) => joint.forward_rate(from, to),
            Composite::Weighted(comp) => comp.forward_rate(from, to),
        }
    }
}
