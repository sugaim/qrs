use qrs_collections::NonEmpty;

use crate::products::general::VariableTypes;

// -----------------------------------------------------------------------------
// MarketRef
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance, category = "Process", value_type = "Number")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "Ts::MarketRef: serde::Serialize",
        deserialize = "Ts::MarketRef: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema, Ts::MarketRef: schemars::JsonSchema")
)]
pub struct MarketRef<Ts: VariableTypes> {
    #[component(field(category = "Market"))]
    #[cfg_attr(feature = "serde", serde(rename = "references"))]
    pub refs: NonEmpty<Vec<Ts::MarketRef>>,
}
