use qrs_collections::NonEmpty;

use crate::products::general::VariableTypes;

// -----------------------------------------------------------------------------
// ConstantFloat
//
#[derive(Debug, Clone, PartialEq, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance, category = "Process", value_type = "Number")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "Ts::Number: serde::Serialize",
        deserialize = "Ts::Number: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema, Ts::Number: schemars::JsonSchema")
)]
pub struct ConstantFloat<Ts: VariableTypes> {
    #[component(field(category = "Constant", value_type = "Number"))]
    pub values: NonEmpty<Vec<Ts::Number>>,
}
