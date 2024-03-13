use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::core::Ccy;

// -----------------------------------------------------------------------------
// Collateral
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Collateral {
    /// Money
    #[serde(with = "ccy_serde")]
    #[schemars(with = "ccy_serde::Ccy")]
    Money(Ccy),

    /// Equity shares
    Share { company: String },
}

mod ccy_serde {
    use schemars::JsonSchema;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize, JsonSchema)]
    pub(super) struct Ccy {
        ccy: crate::core::Ccy,
    }

    pub(super) fn serialize<S>(ccy: &crate::core::Ccy, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Ccy { ccy: *ccy }.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<crate::core::Ccy, D::Error>
    where
        D: Deserializer<'de>,
    {
        let Ccy { ccy } = Deserialize::deserialize(deserializer)?;
        Ok(ccy)
    }
}
