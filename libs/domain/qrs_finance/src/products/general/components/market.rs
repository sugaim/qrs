mod ir;

pub use ir::OvernightRate;

// -----------------------------------------------------------------------------
// Market
//
#[derive(Debug, Clone, PartialEq, Eq, Hash, qrs_finance_derive::Component)]
#[component(_use_from_qrs_finance)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum Market {
    OvernightRate(OvernightRate),
}
