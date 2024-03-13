use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{core::Money, products::general::core::VariableTypes};

// -----------------------------------------------------------------------------
// CouponBase
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(bound(
    serialize = "Ts::DateTime: Serialize, Ts::Number: Serialize, Ts::DayCount: Serialize",
    deserialize = "Ts::DateTime: Deserialize<'de>, Ts::Number: Deserialize<'de>, Ts::DayCount: Deserialize<'de>"
))]
#[schemars(
    bound = "Ts: JsonSchema, Ts::DateTime: JsonSchema, Ts::Number: JsonSchema, Ts::DayCount: JsonSchema"
)]
pub struct CouponBase<Ts: VariableTypes> {
    pub notional: Money<Ts::Number>,
    pub entitle: Ts::DateTime,
    pub period_start: Ts::DateTime,
    pub period_end: Ts::DateTime,
    pub daycount: Ts::DayCount,
    pub payment: Ts::DateTime,
}
