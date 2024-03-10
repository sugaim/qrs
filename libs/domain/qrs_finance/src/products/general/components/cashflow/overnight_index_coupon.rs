use qrs_finance_derive::Component;

use crate::products::{core::CompoundingConvention, general::VariableTypes};

use super::CouponBase;

// -----------------------------------------------------------------------------
// OvernightIndexCoupon
//
#[derive(Clone, Debug, PartialEq, Component)]
#[component(_use_from_qrs_finance, category = "Cashflow")]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "CouponBase<Ts>: serde::Serialize,
            Ts::Number: serde::Serialize,
            Ts::ProcessRef: serde::Serialize,
            Ts::General<CompoundingConvention<Ts::DayCount, Ts::Calendar>>: serde::Serialize",
        deserialize = "CouponBase<Ts>: serde::Deserialize<'de>,
            Ts::Number: serde::Deserialize<'de>,
            Ts::ProcessRef: serde::Deserialize<'de>,
            Ts::General<CompoundingConvention<Ts::DayCount, Ts::Calendar>>: serde::Deserialize<'de>"
    )),
    schemars(bound = "Ts: schemars::JsonSchema,
            CouponBase<Ts>: schemars::JsonSchema,
            Ts::Number: schemars::JsonSchema,
            Ts::ProcessRef: schemars::JsonSchema,
            Ts::General<CompoundingConvention<Ts::DayCount, Ts::Calendar>>: schemars::JsonSchema")
)]
pub struct OvernightIndexCoupon<Ts: VariableTypes> {
    #[cfg_attr(feature = "serde", serde(rename = "coupon_base"))]
    pub base: CouponBase<Ts>,

    pub convention: Ts::General<CompoundingConvention<Ts::DayCount, Ts::Calendar>>,

    #[component(field(category = "Process", value_type = "Number"))]
    pub reference_rate: Ts::ProcessRef,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub spread: Option<Ts::Number>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub gearing: Option<Ts::Number>,
}
