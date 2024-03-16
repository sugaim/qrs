mod constant;
mod deterministic;
mod market_ref;

use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

pub use constant::ConstantFloat;
pub use deterministic::DeterministicFloat;
pub use market_ref::MarketRef;

// -----------------------------------------------------------------------------
// ValueType
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum ValueType {
    Number { dim: usize },
    Boolean { dim: usize },
    Integer { dim: usize },
}

// -----------------------------------------------------------------------------
// Process
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[derivative(
    Debug(bound = "DeterministicFloat<Ts>: std::fmt::Debug,
        ConstantFloat<Ts>: std::fmt::Debug,
        MarketRef<Ts>: std::fmt::Debug"),
    Clone(bound = "DeterministicFloat<Ts>: Clone,
        ConstantFloat<Ts>: Clone,
        MarketRef<Ts>: Clone"),
    PartialEq(bound = "DeterministicFloat<Ts>: PartialEq,
        ConstantFloat<Ts>: PartialEq,
        MarketRef<Ts>: PartialEq")
)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    bound(
        serialize = "DeterministicFloat<Ts>: Serialize,
            ConstantFloat<Ts>: Serialize,
            MarketRef<Ts>: Serialize",
        deserialize = "DeterministicFloat<Ts>: Deserialize<'de>,
            ConstantFloat<Ts>: Deserialize<'de>,
            MarketRef<Ts>: Deserialize<'de>"
    )
)]
#[schemars(bound = "Ts: JsonSchema,
        DeterministicFloat<Ts>: JsonSchema,
        ConstantFloat<Ts>: JsonSchema,
        MarketRef<Ts>: JsonSchema")]
pub enum Process<Ts: VariableTypes> {
    DeterministicFloat(DeterministicFloat<Ts>),
    ConstantFloat(ConstantFloat<Ts>),
    MarketRef(MarketRef<Ts>),
}

//
// methods
//
impl<Ts: VariableTypes> Process<Ts> {
    #[inline]
    pub fn value_type(&self) -> ValueType
    where
        Ts::ProcessRef: AsRef<Self>,
    {
        match self {
            Process::ConstantFloat(c) => ValueType::Number {
                dim: c.values.len(),
            },
            Process::DeterministicFloat(d) => ValueType::Number {
                dim: d.series.len(),
            },
            Process::MarketRef(m) => ValueType::Number { dim: m.refs.len() },
        }
    }
}
