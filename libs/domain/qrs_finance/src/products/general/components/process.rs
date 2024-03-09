mod float;

pub use float::{ConstantFloat, DeterministicFloat};

// -----------------------------------------------------------------------------
// Process
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum Process {
    DeterministicFloat(DeterministicFloat),
    ConstantFloat(ConstantFloat),
}
