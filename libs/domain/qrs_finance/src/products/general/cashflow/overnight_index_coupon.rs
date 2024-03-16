use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

use super::CouponBase;

// -----------------------------------------------------------------------------
// OvernightIndexCoupon
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[derivative(
    Debug(bound = "CouponBase<Ts>: std::fmt::Debug,
        Ts::MarketRef: std::fmt::Debug,
        Ts::Number: std::fmt::Debug,
        Ts::InArrearsConvention: std::fmt::Debug,
        Ts::Rounding: std::fmt::Debug"),
    Clone(bound = "CouponBase<Ts>: Clone,
        Ts::MarketRef: Clone,
        Ts::Number: Clone,
        Ts::InArrearsConvention: Clone,
        Ts::Rounding: Clone"),
    PartialEq(bound = "CouponBase<Ts>: PartialEq,
        Ts::MarketRef: PartialEq,
        Ts::Number: PartialEq,
        Ts::InArrearsConvention: PartialEq,
        Ts::Rounding: PartialEq")
)]
#[component(category = "Cashflow")]
#[serde(bound(
    serialize = "CouponBase<Ts>: Serialize,
            Ts::Number: Serialize,
            Ts::MarketRef: Serialize,
            Ts::InArrearsConvention: Serialize,
            Ts::Rounding: Serialize",
    deserialize = "CouponBase<Ts>: Deserialize<'de>,
            Ts::Number: Deserialize<'de>,
            Ts::MarketRef: Deserialize<'de>,
            Ts::InArrearsConvention: Deserialize<'de>,
            Ts::Rounding: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema,
            CouponBase<Ts>: JsonSchema,
            Ts::Number: JsonSchema,
            Ts::MarketRef: JsonSchema,
            Ts::InArrearsConvention: JsonSchema,
            Ts::Rounding: JsonSchema")]
pub struct OvernightIndexCoupon<Ts: VariableTypes> {
    #[serde(rename = "coupon_base")]
    pub base: CouponBase<Ts>,

    pub convention: Ts::InArrearsConvention,

    #[component(field(category = "Market"))]
    pub reference_rate: Ts::MarketRef,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<Ts::Number>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gearing: Option<Ts::Number>,

    /// rounding method for calculate coupon amount
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Ts::Rounding>,
}

// -----------------------------------------------------------------------------
// OvernightIndexFixing
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OvernightIndexFixing {
    pub rate: f64,
}
