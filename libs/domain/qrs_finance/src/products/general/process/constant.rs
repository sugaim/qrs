use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// ConstantFloat
//
#[derive(Debug, Clone, PartialEq, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Process")]
#[serde(bound(
    serialize = "Ts::Number: Serialize",
    deserialize = "Ts::Number: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, Ts::Number: JsonSchema")]
pub struct ConstantFloat<Ts: VariableTypes> {
    #[component(field(category = "Constant"))]
    pub values: NonEmpty<Vec<Ts::Number>>,
}
