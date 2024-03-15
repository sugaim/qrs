use qrs_finance::{products::Collateral, Ccy};

// -----------------------------------------------------------------------------
// DiscountKey
//
/// Key for discount curve
///
/// Discount curve is specified by currency and collateral.
/// `collateral` is [`None`] for uncollateralized products.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct DiscountKey {
    pub ccy: Ccy,
    pub collateral: Option<Collateral>,
}
