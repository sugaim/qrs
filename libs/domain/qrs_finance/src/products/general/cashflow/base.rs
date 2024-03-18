use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::products::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// CouponBase
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(bound(
    serialize = "Ts::DateTime: Serialize, Ts::Money: Serialize, Ts::DayCount: Serialize",
    deserialize = "Ts::DateTime: Deserialize<'de>, Ts::Money: Deserialize<'de>, Ts::DayCount: Deserialize<'de>"
))]
#[schemars(
    bound = "Ts: JsonSchema, Ts::DateTime: JsonSchema, Ts::Money: JsonSchema, Ts::DayCount: JsonSchema"
)]
pub struct CouponBase<Ts: VariableTypes> {
    /// Notional amount
    pub notional: Ts::Money,
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
        Ts::Money: Into<Ts2::Money>,
        Ts::DateTime: Into<Ts2::DateTime>,
        Ts::DayCount: Into<Ts2::DayCount>,
    {
        CouponBase {
            notional: self.notional.into(),
            entitle: self.entitle.into(),
            period_start: self.period_start.into(),
            period_end: self.period_end.into(),
            daycount: self.daycount.into(),
            payment: self.payment.into(),
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use crate::{
        products::general::{core::ValueOrId, VariableTypesForData},
        Ccy, Money,
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct MockVariableTypes<Ts: VariableTypes = VariableTypesForData>(
        std::marker::PhantomData<Ts>,
    );

    impl<Ts: VariableTypes> VariableTypes for MockVariableTypes<Ts> {
        type Money = Option<Ts::Money>;
        type Boolean = Option<Ts::Boolean>;
        type Number = Option<Ts::Number>;
        type DateTime = Option<Ts::DateTime>;
        type DayCount = Option<Ts::DayCount>;
        type Calendar = Option<Ts::Calendar>;
        type CashflowRef = Option<Ts::CashflowRef>;
        type InArrearsConvention = Option<Ts::InArrearsConvention>;
        type Integer = Option<Ts::Integer>;
        type LegRef = Option<Ts::LegRef>;
        type MarketRef = Option<Ts::MarketRef>;
        type ProcessRef = Option<Ts::ProcessRef>;
        type Rounding = Option<Ts::Rounding>;
    }

    #[test]
    fn test_change_variable_types_to() {
        let coupon: CouponBase<VariableTypesForData> = CouponBase {
            notional: ValueOrId::Value(Money {
                amount: 100.0,
                ccy: Ccy::USD,
            }),
            entitle: ValueOrId::Id("entitle".to_string()),
            period_start: ValueOrId::Id("period_start".to_string()),
            period_end: ValueOrId::Id("period_end".to_string()),
            daycount: ValueOrId::Id("daycount".to_string()),
            payment: ValueOrId::Id("payment".to_string()),
        };
        let expected: CouponBase<MockVariableTypes> = CouponBase {
            notional: Some(coupon.notional.clone()),
            entitle: Some(coupon.entitle.clone()),
            period_start: Some(coupon.period_start.clone()),
            period_end: Some(coupon.period_end.clone()),
            daycount: Some(coupon.daycount.clone()),
            payment: Some(coupon.payment.clone()),
        };

        let actual = coupon.change_variable_types_to::<MockVariableTypes>();

        assert_eq!(actual, expected);
    }
}
