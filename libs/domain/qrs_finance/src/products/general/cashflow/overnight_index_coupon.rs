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

// =============================================================================
#[cfg(test)]
mod tests {
    use crate::{
        products::general::{
            core::{ValueLess, ValueOrId},
            VariableTypesForData,
        },
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
        let coupon: OvernightIndexCoupon<VariableTypesForData> = OvernightIndexCoupon {
            base: CouponBase {
                notional: ValueOrId::Value(Money {
                    amount: 100.0,
                    ccy: Ccy::USD,
                }),
                entitle: ValueOrId::Id("entitle".to_string()),
                period_start: ValueOrId::Id("period_start".to_string()),
                period_end: ValueOrId::Id("period_end".to_string()),
                daycount: ValueOrId::Id("daycount".to_string()),
                payment: ValueOrId::Id("payment".to_string()),
            },
            convention: ValueOrId::Id("convention".to_string()),
            gearing: Some(ValueOrId::Value(1.0)),
            reference_rate: WithId {
                id: "reference_rate".to_string(),
                value: ValueLess,
            },
            spread: Some(ValueOrId::Value(0.01)),
            rounding: ValueOrId::Id("rounding".to_string()).into(),
        };
        let expected: OvernightIndexCoupon<MockVariableTypes> = OvernightIndexCoupon {
            base: coupon.base.clone().change_variable_types_to(),
            convention: Some(coupon.convention.clone()),
            reference_rate: WithId {
                id: coupon.reference_rate.id.clone(),
                value: Some(ValueLess),
            },
            spread: Some(coupon.spread.clone()),
            gearing: Some(coupon.gearing.clone()),
            rounding: coupon.rounding.clone().map(Into::into),
        };

        let actual = coupon.change_variable_types_to::<MockVariableTypes>();

        assert_eq!(actual, expected);
    }
}
