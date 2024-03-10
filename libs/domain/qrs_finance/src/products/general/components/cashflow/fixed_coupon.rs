use qrs_finance_derive::Component;

use crate::products::general::VariableTypes;

use super::CouponBase;

// -----------------------------------------------------------------------------
// FixedCoupon
//
#[derive(Clone, Debug, PartialEq, Component)]
#[component(_use_from_qrs_finance, category = "Cashflow")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "CouponBase<Ts>: serde::Serialize,
            Ts::Number: serde::Serialize",
        deserialize = "CouponBase<Ts>: serde::Deserialize<'de>,
            Ts::Number: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema,
            CouponBase<Ts>: schemars::JsonSchema,
            Ts::Number: schemars::JsonSchema")
)]
pub struct FixedCoupon<Ts: VariableTypes> {
    #[cfg_attr(feature = "serde", serde(rename = "coupon_base"))]
    pub base: CouponBase<Ts>,
    pub rate: Ts::Number,
}
