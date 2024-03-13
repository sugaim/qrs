use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// StraightLeg
//
#[derive(Debug, Clone, PartialEq, Component, Serialize, Deserialize, JsonSchema)]
#[component(_use_from_qrs_finance, category = "Leg")]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "Ts::CashflowRef: Serialize",
        deserialize = "Ts::CashflowRef: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema, Ts::CashflowRef: JsonSchema")]
pub struct StraightLeg<Ts: VariableTypes> {
    #[component(field(category = "Cashflow"))]
    pub cashflows: Vec<Ts::CashflowRef>,
}
