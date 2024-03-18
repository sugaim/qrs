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
    Debug(bound = "CouponBase<Ts>: std::fmt::Debug,
            Ts::Number: std::fmt::Debug,
            Ts::DayCount: std::fmt::Debug,
            Ts::Rounding: std::fmt::Debug"),
    Clone(bound = "CouponBase<Ts>: Clone,
        Ts::Number: Clone,
        Ts::DayCount: Clone,
        Ts::Rounding: Clone"),
    PartialEq(bound = "CouponBase<Ts>: PartialEq,
        Ts::Number: PartialEq,
        Ts::DayCount: PartialEq,
        Ts::Rounding: PartialEq")
)]
#[component(category = "Cashflow")]
#[serde(bound(
    serialize = "CouponBase<Ts>: Serialize,
        Ts::Number: Serialize,
        Ts::DayCount: Serialize,
        Ts::Rounding: Serialize",
    deserialize = "CouponBase<Ts>: Deserialize<'de>,
        Ts::Number: Deserialize<'de>,
        Ts::DayCount: Deserialize<'de>,
        Ts::Rounding: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema,
        CouponBase<Ts>: JsonSchema,
        Ts::Number: JsonSchema,
        Ts::DayCount: JsonSchema,
        Ts::Rounding: JsonSchema")]
pub struct FixedCoupon<Ts: VariableTypes> {
    #[serde(rename = "coupon_base")]
    pub base: CouponBase<Ts>,
    pub rate: Ts::Number,
    pub accrual: Ts::DayCount,

    /// rounding method for calculate coupon amount
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Ts::Rounding>,
}

//
// methods
//
impl<Ts: VariableTypes> FixedCoupon<Ts> {
    #[inline]
    pub fn change_variable_types_to<Ts2: VariableTypes>(self) -> FixedCoupon<Ts2>
    where
        Ts::Number: Into<Ts2::Number>,
        Ts::Money: Into<Ts2::Money>,
        Ts::DateTime: Into<Ts2::DateTime>,
        Ts::DayCount: Into<Ts2::DayCount>,
        Ts::Rounding: Into<Ts2::Rounding>,
    {
        FixedCoupon {
            base: self.base.change_variable_types_to(),
            rate: self.rate.into(),
            accrual: self.accrual.into(),
            rounding: self.rounding.map(|x| x.into()),
        }
    }
}
