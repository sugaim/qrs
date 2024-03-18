use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::product::general::core::VariableTypes;

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
    #[has_dependency]
    pub base: CouponBase<Ts>,

    #[has_dependency(ref_category = "Constant")]
    pub rate: Ts::Number,

    #[has_dependency(ref_category = "Constant")]
    pub accrual_daycount: Ts::DayCount,

    /// rounding method for calculate coupon amount
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[has_dependency(ref_category = "Constant")]
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
            accrual_daycount: self.accrual_daycount.into(),
            rounding: self.rounding.map(|x| x.into()),
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use maplit::hashset;

    use crate::{
        product::general::{
            core::{Component, ComponentCategory, HasDependency, ValueOrId},
            VariableTypesForData,
        },
        Ccy, Money,
    };

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct OptVariableTypes<Ts: VariableTypes = VariableTypesForData>(std::marker::PhantomData<Ts>);

    impl<Ts: VariableTypes> VariableTypes for OptVariableTypes<Ts> {
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

    fn cpn() -> FixedCoupon<VariableTypesForData> {
        FixedCoupon {
            base: CouponBase {
                notional: ValueOrId::Value(Money {
                    amount: 100.0,
                    ccy: Ccy::USD,
                }),
                entitle: ValueOrId::Id("entitle".into()),
                period_start: ValueOrId::Id("period_start".into()),
                period_end: ValueOrId::Id("period_end".into()),
                daycount: ValueOrId::Id("daycount".into()),
                payment: ValueOrId::Id("payment".into()),
            },
            accrual_daycount: ValueOrId::Id("accrual".into()),
            rate: ValueOrId::Value(0.05),
            rounding: ValueOrId::Id("rounding".into()).into(),
        }
    }

    #[test]
    fn test_change_variable_types_to() {
        let coupon = cpn();
        let expected: FixedCoupon<OptVariableTypes> = FixedCoupon {
            base: coupon.base.clone().change_variable_types_to(),
            rate: Some(coupon.rate.clone()),
            accrual_daycount: Some(coupon.accrual_daycount.clone()),
            rounding: coupon.rounding.clone().map(Into::into),
        };

        let actual = coupon.change_variable_types_to::<OptVariableTypes>();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_category() {
        let node = cpn();

        let cat = node.category();

        assert_eq!(cat, ComponentCategory::Cashflow);
    }

    #[test]
    fn test_depends_on() {
        let node = cpn();
        let expected = hashset! {
            ("entitle", ComponentCategory::Constant),
            ("period_start", ComponentCategory::Constant),
            ("period_end", ComponentCategory::Constant),
            ("daycount", ComponentCategory::Constant),
            ("payment", ComponentCategory::Constant),
            ("accrual", ComponentCategory::Constant),
            ("rounding", ComponentCategory::Constant)
        };

        let deps = node.depends_on().into_iter().collect::<HashSet<_>>();

        assert_eq!(deps, expected);
    }
}
