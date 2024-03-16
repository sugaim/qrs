use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{products::general::core::VariableTypes, Money};

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
    /// Notional amount
    pub notional: Money<Ts::Number>,
    /// A date which the right of the coupon is granted
    pub entitle: Ts::DateTime,
    /// Accrual period start date
    pub period_start: Ts::DateTime,
    /// Accrual period end date
    pub period_end: Ts::DateTime,
    /// Day count convention to calculate dcf of accrual period
    pub daycount: Ts::DayCount,
    /// Payment date
    pub payment: Ts::DateTime,
}

//
// methods
//
impl<Ts: VariableTypes> CouponBase<Ts> {
    #[inline]
    pub fn change_variable_types_to<Ts2: VariableTypes>(self) -> CouponBase<Ts2>
    where
        Ts::Number: Into<Ts2::Number>,
        Ts::DateTime: Into<Ts2::DateTime>,
        Ts::DayCount: Into<Ts2::DayCount>,
    {
        CouponBase {
            notional: Money {
                amount: self.notional.amount.into(),
                ccy: self.notional.ccy,
            },
            entitle: self.entitle.into(),
            period_start: self.period_start.into(),
            period_end: self.period_end.into(),
            daycount: self.daycount.into(),
            payment: self.payment.into(),
        }
    }
}
