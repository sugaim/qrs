use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Ccy;

// -----------------------------------------------------------------------------
// Collateral
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Collateral {
    Money { ccy: Ccy },
    Share { company: String },
}
