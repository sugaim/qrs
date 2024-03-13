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
        Ts::ProcessRef: std::fmt::Debug,
        Ts::Number: std::fmt::Debug,
        Ts::CompoundingConvention: std::fmt::Debug"),
    Clone(bound = "CouponBase<Ts>: Clone,
        Ts::ProcessRef: Clone,
        Ts::Number: Clone,
        Ts::CompoundingConvention: Clone"),
    PartialEq(bound = "CouponBase<Ts>: PartialEq,
        Ts::ProcessRef: PartialEq,
        Ts::Number: PartialEq,
        Ts::CompoundingConvention: PartialEq")
)]
#[component(_use_from_qrs_finance, category = "Cashflow")]
#[serde(bound(
    serialize = "CouponBase<Ts>: Serialize,
            Ts::Number: Serialize,
            Ts::ProcessRef: Serialize,
            Ts::CompoundingConvention: Serialize",
    deserialize = "CouponBase<Ts>: Deserialize<'de>,
            Ts::Number: Deserialize<'de>,
            Ts::ProcessRef: Deserialize<'de>,
            Ts::CompoundingConvention: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema,
            CouponBase<Ts>: JsonSchema,
            Ts::Number: JsonSchema,
            Ts::ProcessRef: JsonSchema,
            Ts::CompoundingConvention: JsonSchema")]
pub struct OvernightIndexCoupon<Ts: VariableTypes> {
    #[serde(rename = "coupon_base")]
    pub base: CouponBase<Ts>,

    pub convention: Ts::CompoundingConvention,

    #[component(field(category = "Process", value_type = "Number"))]
    pub reference_rate: Ts::ProcessRef,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<Ts::Number>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gearing: Option<Ts::Number>,
}
