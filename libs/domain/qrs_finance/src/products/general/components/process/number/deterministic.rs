use std::collections::HashMap;

use qrs_collections::NonEmpty;

use crate::products::general::VariableTypes;

// -----------------------------------------------------------------------------
// Deterministic
//
#[derive(Debug, Clone, PartialEq, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance, category = "Process", value_type = "Number")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "Ts::DateTime: serde::Serialize, Ts::Number: serde::Serialize",
        deserialize = "Ts::DateTime: serde::Deserialize<'de>, Ts::Number: serde::Deserialize<'de>"
    )),
    schemars(
        bound = "Ts: schemars::JsonSchema, Ts::Number: schemars::JsonSchema, Ts::DateTime: schemars::JsonSchema"
    )
)]
pub struct DeterministicFloat<Ts: VariableTypes> {
    #[component(field(category = "Constant", value_type = "Number"))]
    #[allow(clippy::type_complexity)]
    pub series: NonEmpty<Vec<NonEmpty<HashMap<Ts::DateTime, Ts::Number>>>>,
}
