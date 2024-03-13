use std::fmt::Display;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::Ccy;

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
