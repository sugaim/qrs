use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::{VariableTypes, WithId};

// -----------------------------------------------------------------------------
// StraightLeg
//
#[derive(Debug, Clone, PartialEq, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Leg")]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "WithId<Ts::CashflowRef>: Serialize",
        deserialize = "WithId<Ts::CashflowRef>: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema, WithId<Ts::CashflowRef>: JsonSchema")]
pub struct StraightLeg<Ts: VariableTypes> {
    #[component(field(category = "Cashflow"))]
    pub cashflows: Vec<WithId<Ts::CashflowRef>>,
}
