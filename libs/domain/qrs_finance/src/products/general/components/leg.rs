mod straight;

use crate::products::general::core::VariableTypes;

use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub use straight::StraightLeg;

// -----------------------------------------------------------------------------
// Leg
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[component(_use_from_qrs_finance)]
#[derivative(
    Debug(bound = "StraightLeg<Ts>: std::fmt::Debug"),
    Clone(bound = "StraightLeg<Ts>: Clone"),
    PartialEq(bound = "StraightLeg<Ts>: PartialEq")
)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "StraightLeg<Ts>: Serialize",
        deserialize = "StraightLeg<Ts>: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema, StraightLeg<Ts>: JsonSchema")]
pub enum Leg<Ts: VariableTypes> {
    Straight(StraightLeg<Ts>),
}
