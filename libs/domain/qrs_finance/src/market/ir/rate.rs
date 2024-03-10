// -----------------------------------------------------------------------------
// OvernightRate
//
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(rename_all = "UPPERCASE")
)]
#[allow(clippy::upper_case_acronyms)]
pub enum OvernightRate {
    TONA,
    SOFR,
}
