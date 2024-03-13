use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// ConstantFloat
//
#[derive(Debug, Clone, PartialEq, Component, Serialize, Deserialize, JsonSchema)]
#[component(_use_from_qrs_finance, category = "Process", value_type = "Number")]
#[serde(bound(
    serialize = "Ts::Number: Serialize",
    deserialize = "Ts::Number: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, Ts::Number: JsonSchema")]
pub struct ConstantFloat<Ts: VariableTypes> {
    #[component(field(category = "Constant", value_type = "Number"))]
    pub values: NonEmpty<Vec<Ts::Number>>,
}
