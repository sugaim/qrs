mod ir;

use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use ir::OvernightRate;

// -----------------------------------------------------------------------------
// Market
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[component(_use_from_qrs_finance)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Market {
    OvernightRate(OvernightRate),
}
