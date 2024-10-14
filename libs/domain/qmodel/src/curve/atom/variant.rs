use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, quantity::Yield};
use qmath::{interp1d::Pwconst1d, num::Real};

use crate::curve::YieldCurve;

use super::{instfwd::Instfwd, Flat};

// -----------------------------------------------------------------------------
// Atom
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Atom<V> {
    Flat(Flat<V>),
    InstfwdPwconst(Instfwd<Pwconst1d<DateTime, Yield<Act365f, V>>>),
}

impl<V: schemars::JsonSchema> schemars::JsonSchema for Atom<V> {
    fn schema_name() -> String {
        format!("AtomCurve_for_{}", V::schema_name())
    }

    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qmodel::curve::atom::Atom<{}>", V::schema_id()).into()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        #[derive(schemars::JsonSchema)]
        #[serde(tag = "type", rename_all = "snake_case")]
        #[allow(dead_code)]
        enum ForSchema<V> {
            Flat(Flat<V>),
            InstfwdPwconst(Instfwd<Pwconst1d<DateTime, Yield<Act365f, V>>>),
        }
        ForSchema::<V>::json_schema(gen)
    }
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
            Atom::InstfwdPwconst(instfwd) => instfwd.forward_rate(from, to),
        }
    }
}
