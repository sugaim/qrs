use std::fmt::Display;

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

// -----------------------------------------------------------------------------
// Money
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Money<V> {
    pub amount: V,
    pub ccy: Ccy,
}

//
// display, serde
//
impl<V> Display for Money<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}[{}]", self.amount, self.ccy)
    }
}
