use qrs_finance_derive::Component;

use crate::products::general::VariableTypes;

// -----------------------------------------------------------------------------
// StraightLeg
//
#[derive(Debug, Clone, PartialEq, Component)]
#[component(_use_from_qrs_finance, category = "Leg")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(tag = "type", rename_all = "snake_case"),
    serde(bound(
        serialize = "Ts::CashflowRef: serde::Serialize",
        deserialize = "Ts::CashflowRef: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema, Ts::CashflowRef: schemars::JsonSchema")
)]
pub struct StraightLeg<Ts: VariableTypes> {
    #[component(field(category = "Cashflow"))]
    pub cashflows: Vec<Ts::CashflowRef>,
}
