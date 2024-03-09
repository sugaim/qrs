use std::collections::HashMap;

use qrs_chrono::DateWithTag;
use qrs_collections::NonEmpty;

use crate::products::general::ValueType;

// -----------------------------------------------------------------------------
// Deterministic
//
#[derive(Debug, Clone, PartialEq, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance, category = "Process", value_type = "Float")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct DeterministicFloat {
    #[component(field(category = "Constant"))]
    pub series: NonEmpty<Vec<HashMap<DateWithTag, String>>>,
}
