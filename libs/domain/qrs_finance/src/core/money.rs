use std::fmt::Display;

use super::Ccy;

// -----------------------------------------------------------------------------
// Money
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
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
