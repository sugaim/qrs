use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// OvernightRate
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[component(category = "Market")]
pub struct OvernightRate {
    pub reference: crate::market::ir::OvernightRate,
}
