use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
// FiniteDiffMethod
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FiniteDiffMethod {
    /// Forward difference
    Forward,
    /// Backward difference
    Backward,
    /// Central difference
    Central,
}
