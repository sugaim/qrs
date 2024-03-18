use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::{VariableTypes, WithId};

use super::CouponBase;

// -----------------------------------------------------------------------------
// OvernightIndexCoupon
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[derivative(
    Debug(bound = "CouponBase<Ts>: std::fmt::Debug,
        WithId<Ts::MarketRef>: std::fmt::Debug,
        Ts::Number: std::fmt::Debug,
        Ts::InArrearsConvention: std::fmt::Debug,
        Ts::Rounding: std::fmt::Debug"),
    Clone(bound = "CouponBase<Ts>: Clone,
        WithId<Ts::MarketRef>: Clone,
        Ts::Number: Clone,
        Ts::InArrearsConvention: Clone,
        Ts::Rounding: Clone"),
    PartialEq(bound = "CouponBase<Ts>: PartialEq,
        WithId<Ts::MarketRef>: PartialEq,
        Ts::Number: PartialEq,
        Ts::InArrearsConvention: PartialEq,
        Ts::Rounding: PartialEq")
)]
#[component(category = "Cashflow")]
#[serde(bound(
    serialize = "CouponBase<Ts>: Serialize,
            Ts::Number: Serialize,
            WithId<Ts::MarketRef>: Serialize,
            Ts::InArrearsConvention: Serialize,
            Ts::Rounding: Serialize",
    deserialize = "CouponBase<Ts>: Deserialize<'de>,
            Ts::Number: Deserialize<'de>,
            WithId<Ts::MarketRef>: Deserialize<'de>,
            Ts::InArrearsConvention: Deserialize<'de>,
            Ts::Rounding: Deserialize<'de>"
))]
#[schemars(bound = "Ts: JsonSchema,
            CouponBase<Ts>: JsonSchema,
            Ts::Number: JsonSchema,
            WithId<Ts::MarketRef>: JsonSchema,
            Ts::InArrearsConvention: JsonSchema,
            Ts::Rounding: JsonSchema")]
pub struct OvernightIndexCoupon<Ts: VariableTypes> {
    #[serde(rename = "coupon_base")]
    pub base: CouponBase<Ts>,

    pub convention: Ts::InArrearsConvention,

    #[component(field(category = "Market"))]
    pub reference_rate: WithId<Ts::MarketRef>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spread: Option<Ts::Number>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gearing: Option<Ts::Number>,

    /// rounding method for calculate coupon amount
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Ts::Rounding>,
}

//
// methods
//
impl<Ts: VariableTypes> OvernightIndexCoupon<Ts> {
    #[inline]
    pub fn change_variable_types_to<Ts2: VariableTypes>(self) -> OvernightIndexCoupon<Ts2>
    where
        Ts::Number: Into<Ts2::Number>,
        Ts::Money: Into<Ts2::Money>,
        Ts::DateTime: Into<Ts2::DateTime>,
        Ts::DayCount: Into<Ts2::DayCount>,
        Ts::InArrearsConvention: Into<Ts2::InArrearsConvention>,
        Ts::MarketRef: Into<Ts2::MarketRef>,
        Ts::Rounding: Into<Ts2::Rounding>,
    {
        OvernightIndexCoupon {
            base: self.base.change_variable_types_to(),
            convention: self.convention.into(),
            reference_rate: WithId {
                id: self.reference_rate.id,
                value: self.reference_rate.value.into(),
            },
            spread: self.spread.map(Into::into),
            gearing: self.gearing.map(Into::into),
            rounding: self.rounding.map(Into::into),
        }
    }
}

// -----------------------------------------------------------------------------
// OvernightIndexFixing
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OvernightIndexFixing {
    pub rate: f64,
}
