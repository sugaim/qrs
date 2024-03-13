use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// MarketRef
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[component(_use_from_qrs_finance, category = "Process", value_type = "Number")]
#[serde(bound(
    serialize = "Ts::MarketRef: Serialize",
    deserialize = "Ts::MarketRef: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, Ts::MarketRef: JsonSchema")]
pub struct MarketRef<Ts: VariableTypes> {
    #[component(field(category = "Market"))]
    #[serde(rename = "references")]
    pub refs: NonEmpty<Vec<Ts::MarketRef>>,
}
