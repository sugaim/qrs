use std::collections::HashMap;

use derivative::Derivative;
use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// Deterministic
//
#[derive(Debug, Clone, Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Process")]
#[derivative(PartialEq(bound = "Ts::DateTime: Eq + std::hash::Hash, Ts::Number: PartialEq  "))]
#[serde(bound(
    serialize = "Ts::DateTime: Eq + std::hash::Hash + Serialize, Ts::Number: Serialize",
    deserialize = "Ts::DateTime: Eq + std::hash::Hash + Deserialize<'de>, Ts::Number: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, Ts::Number: JsonSchema, Ts::DateTime: JsonSchema")]
pub struct DeterministicFloat<Ts: VariableTypes> {
    #[component(field(category = "Constant"))]
    #[allow(clippy::type_complexity)]
    pub series: NonEmpty<Vec<NonEmpty<HashMap<Ts::DateTime, Ts::Number>>>>,
}
