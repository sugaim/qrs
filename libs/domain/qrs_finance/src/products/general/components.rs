use std::collections::HashMap;

use self::{constant::Constant, market::Market};

use super::VariableTypes;

pub mod cashflow;
pub mod constant;
pub mod leg;
pub mod market;
pub mod process;

// -----------------------------------------------------------------------------
// Components
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "process::Process<Ts>: serde::Serialize,
            cashflow::Cashflow<Ts>: serde::Serialize,
            leg::Leg<Ts>: serde::Serialize",
        deserialize = "process::Process<Ts>: serde::Deserialize<'de>,
            cashflow::Cashflow<Ts>: serde::Deserialize<'de>,
            leg::Leg<Ts>: serde::Deserialize<'de>",
    )),
    schemars(bound = "Ts: schemars::JsonSchema,
        process::Process<Ts>: schemars::JsonSchema,
        cashflow::Cashflow<Ts>: schemars::JsonSchema,
        leg::Leg<Ts>: schemars::JsonSchema")
)]
pub struct Components<Ts: VariableTypes> {
    pub constants: HashMap<String, Constant>,
    pub markets: HashMap<String, Market>,
    pub processes: HashMap<String, process::Process<Ts>>,
    pub cashflows: HashMap<String, cashflow::Cashflow<Ts>>,
    pub legs: HashMap<String, leg::Leg<Ts>>,
}
