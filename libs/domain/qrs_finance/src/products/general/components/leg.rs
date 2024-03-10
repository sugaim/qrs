mod straight;

use crate::products::general::VariableTypes;

use qrs_finance_derive::Component;
pub use straight::StraightLeg;

// -----------------------------------------------------------------------------
// Leg
//
#[derive(Debug, Clone, PartialEq, Component)]
#[component(_use_from_qrs_finance)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(tag = "type", rename_all = "snake_case"),
    serde(bound(
        serialize = "StraightLeg<Ts>: serde::Serialize",
        deserialize = "StraightLeg<Ts>: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema, StraightLeg<Ts>: schemars::JsonSchema")
)]
pub enum Leg<Ts: VariableTypes> {
    Straight(StraightLeg<Ts>),
}
