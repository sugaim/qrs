use crate::products::core::Collateral;

use super::{components::Components, VariableTypes};

// -----------------------------------------------------------------------------
// GeneralProduct
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "Components<Ts>: serde::Serialize",
        deserialize = "Components<Ts>: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema, Components<Ts>: schemars::JsonSchema")
)]
pub struct GeneralProduct<Ts: VariableTypes> {
    pub collateral: Collateral,
    pub components: Components<Ts>,
}
