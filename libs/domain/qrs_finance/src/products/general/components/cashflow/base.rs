use crate::{core::Money, products::general::VariableTypes};

// -----------------------------------------------------------------------------
// CouponBase
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "Ts::DateTime: serde::Serialize, Ts::Number: serde::Serialize, Ts::DayCount: serde::Serialize",
        deserialize = "Ts::DateTime: serde::Deserialize<'de>, Ts::Number: serde::Deserialize<'de>, Ts::DayCount: serde::Deserialize<'de>"
    )),
    schemars(
        bound = "Ts: schemars::JsonSchema, Ts::DateTime: schemars::JsonSchema, Ts::Number: schemars::JsonSchema, Ts::DayCount: schemars::JsonSchema"
    )
)]
pub struct CouponBase<Ts: VariableTypes> {
    pub notional: Money<Ts::Number>,
    pub entitle: Ts::DateTime,
    pub period_start: Ts::DateTime,
    pub period_end: Ts::DateTime,
    pub daycount: Ts::DayCount,
    pub payment: Ts::DateTime,
}
