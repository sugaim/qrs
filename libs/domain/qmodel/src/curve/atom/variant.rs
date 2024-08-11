use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Yield};
use qmath::num::Real;

use crate::curve::YieldCurve;

use super::Flat;

// -----------------------------------------------------------------------------
// Atom
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum Atom<V> {
    Flat(Flat<V>),
}

impl<V: Real> YieldCurve for Atom<V> {
    type Value = V;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        match self {
            Atom::Flat(flat) => flat.forward_rate(from, to),
        }
    }
}
