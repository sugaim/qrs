mod number;

pub use number::{ConstantFloat, DeterministicFloat, MarketRef};

use crate::products::general::VariableTypes;

// -----------------------------------------------------------------------------
// Process
//
#[derive(Debug, PartialEq, Clone, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(tag = "type", rename_all = "snake_case"),
    serde(bound(
        serialize = "DeterministicFloat<Ts>: serde::Serialize,
            ConstantFloat<Ts>: serde::Serialize,
            MarketRef<Ts>: serde::Serialize",
        deserialize = "DeterministicFloat<Ts>: serde::Deserialize<'de>,
            ConstantFloat<Ts>: serde::Deserialize<'de>,
            MarketRef<Ts>: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema,
        DeterministicFloat<Ts>: schemars::JsonSchema,
        ConstantFloat<Ts>: schemars::JsonSchema,
        MarketRef<Ts>: schemars::JsonSchema")
)]
pub enum Process<Ts: VariableTypes> {
    DeterministicFloat(DeterministicFloat<Ts>),
    ConstantFloat(ConstantFloat<Ts>),
    Market(MarketRef<Ts>),
}
