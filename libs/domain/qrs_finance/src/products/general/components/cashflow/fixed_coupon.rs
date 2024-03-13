use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

use super::CouponBase;

// -----------------------------------------------------------------------------
// FixedCoupon
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[derivative(
    Debug(bound = "CouponBase<Ts>: std::fmt::Debug, Ts::Number: std::fmt::Debug"),
    Clone(bound = "CouponBase<Ts>: Clone, Ts::Number: Clone"),
    PartialEq(bound = "CouponBase<Ts>: PartialEq, Ts::Number: PartialEq")
)]
#[component(_use_from_qrs_finance, category = "Cashflow")]
#[serde(bound(
    serialize = "CouponBase<Ts>: Serialize, Ts::Number: Serialize",
    deserialize = "CouponBase<Ts>: Deserialize<'de>, Ts::Number: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema, CouponBase<Ts>: JsonSchema, Ts::Number: JsonSchema")]
pub struct FixedCoupon<Ts: VariableTypes> {
    #[serde(rename = "coupon_base")]
    pub base: CouponBase<Ts>,
    pub rate: Ts::Number,
}
