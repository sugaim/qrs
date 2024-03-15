mod overnight_rate;

use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use overnight_rate::OvernightRate;

// -----------------------------------------------------------------------------
// Market
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Market {
    OvernightRate(OvernightRate),
}
