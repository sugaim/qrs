use schemars::{schema::Schema, JsonSchema};
use serde::{Deserialize, Serialize};

use crate::{finance::daycount::Act365F, num::Real};

use super::traits::Rate;

// -----------------------------------------------------------------------------
// RateAct365F
//
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct RateAct365F<V>(V);

//
// display, serde
//
impl<V: JsonSchema> JsonSchema for RateAct365F<V> {
    fn schema_name() -> String {
        format!("RateAct365F_for_{}", V::schema_name())
    }
    fn schema_id() -> std::borrow::Cow<'static, str> {
        format!("qrs_core::finance::RateAct365F<{}>", V::schema_id()).into()
    }
    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> Schema {
        let mut schema = V::json_schema(gen);
        if let Schema::Object(ref mut schema) = schema {
            schema.metadata().description = Some(
                "Annual rate with Act/365 fixed convention. Unit is 1. Not percentage nor bps."
                    .to_string(),
            );
        }
        schema
    }
}

//
// methods
//
impl<V> RateAct365F<V> {
    /// Create a new `RateAct365F` instance with the given annual rate.
    ///
    /// Unit of the argument is 1. Not percent nor bps.
    #[inline]
    pub fn with_annual_rate(value: V) -> Self {
        Self(value)
    }
}

impl<V: Real> Rate for RateAct365F<V> {
    type Value = V;
    type Convention = Act365F;

    #[inline]
    fn convention(&self) -> Self::Convention {
        Act365F
    }

    #[inline]
    fn value(&self) -> Self::Value {
        self.0.clone()
    }
}
