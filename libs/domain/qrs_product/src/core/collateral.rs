use qrs_fincore::Ccy;

// -----------------------------------------------------------------------------
// Collateral
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "snake_case", tag = "type")
)]
pub enum Collateral {
    #[cfg_attr(
        feature = "serde",
        serde(with = "ccy_serde"),
        schemars(with = "ccy_serde::Ccy")
    )]
    Money(Ccy),
}

#[cfg(feature = "serde")]
mod ccy_serde {
    use schemars::JsonSchema;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize, JsonSchema)]
    pub(super) struct Ccy {
        ccy: qrs_fincore::Ccy,
    }

    pub(super) fn serialize<S>(ccy: &qrs_fincore::Ccy, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Ccy { ccy: *ccy }.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<qrs_fincore::Ccy, D::Error>
    where
        D: Deserializer<'de>,
    {
        let Ccy { ccy } = Deserialize::deserialize(deserializer)?;
        Ok(ccy)
    }
}
