use qrs_collections::NonEmpty;

use crate::products::general::ValueType;

// -----------------------------------------------------------------------------
// ConstantFloat
//
#[derive(Debug, Clone, PartialEq, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance, category = "Process", value_type = "Float")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct ConstantFloat {
    #[component(field(category = "Constant"))]
    pub values: NonEmpty<Vec<String>>,
}
