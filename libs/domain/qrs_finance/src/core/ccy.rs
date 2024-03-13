use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

// -----------------------------------------------------------------------------
// Ccy
//
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Display,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
    JsonSchema,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Ccy {
    JPY,
    USD,
}
