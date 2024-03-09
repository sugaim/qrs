// -----------------------------------------------------------------------------
// Ccy
//
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, strum::Display, strum::EnumIter, strum::EnumString,
)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "UPPERCASE")
)]
pub enum Ccy {
    JPY,
    USD,
}
