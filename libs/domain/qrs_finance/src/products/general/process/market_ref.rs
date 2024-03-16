use qrs_collections::NonEmpty;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::{VariableTypes, WithId};

// -----------------------------------------------------------------------------
// MarketRef
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Process")]
#[serde(bound(
    serialize = "WithId<Ts::MarketRef>: Serialize",
    deserialize = "WithId<Ts::MarketRef>: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, WithId<Ts::MarketRef>: JsonSchema")]
pub struct MarketRef<Ts: VariableTypes> {
    #[component(field(category = "Market"))]
    #[serde(rename = "references")]
    pub refs: NonEmpty<Vec<WithId<Ts::MarketRef>>>,
}
