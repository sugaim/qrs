// -----------------------------------------------------------------------------
// OvernightRate
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance, category = "Market")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct OvernightRate {
    pub reference: crate::market::ir::OvernightRate,
}
