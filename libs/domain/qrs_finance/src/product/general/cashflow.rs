mod base;
mod fixed_coupon;
mod overnight_index_coupon;

use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use base::CouponBase;
pub use fixed_coupon::FixedCoupon;
pub use overnight_index_coupon::{OvernightIndexCoupon, OvernightIndexFixing};

use crate::product::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// Cashflow
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[derivative(
    Debug(bound = "FixedCoupon<Ts>: std::fmt::Debug,
        OvernightIndexCoupon<Ts>: std::fmt::Debug"),
    Clone(bound = "FixedCoupon<Ts>: Clone,
        OvernightIndexCoupon<Ts>: Clone"),
    PartialEq(bound = "FixedCoupon<Ts>: PartialEq,
        OvernightIndexCoupon<Ts>: PartialEq")
)]
#[serde(
    bound(
        serialize = "FixedCoupon<Ts>: Serialize,
            OvernightIndexCoupon<Ts>: Serialize",
        deserialize = "FixedCoupon<Ts>: Deserialize<'de>,
            OvernightIndexCoupon<Ts>: Deserialize<'de>"
    ),
    tag = "type",
    rename_all = "snake_case"
)]
#[schemars(bound = "Ts: JsonSchema,
        FixedCoupon<Ts>: JsonSchema,
        OvernightIndexCoupon<Ts>: JsonSchema")]
pub enum Cashflow<Ts: VariableTypes> {
    FixedCoupon(FixedCoupon<Ts>),
    OvernightIndexCoupon(OvernightIndexCoupon<Ts>),
}

//
// methods
//
impl<Ts: VariableTypes> Cashflow<Ts> {
    #[inline]
    pub fn change_variable_types_to<Ts2: VariableTypes>(self) -> Cashflow<Ts2>
    where
        Ts::Number: Into<Ts2::Number>,
        Ts::Money: Into<Ts2::Money>,
        Ts::DateTime: Into<Ts2::DateTime>,
        Ts::DayCount: Into<Ts2::DayCount>,
        Ts::Rounding: Into<Ts2::Rounding>,
        Ts::InArrearsConvention: Into<Ts2::InArrearsConvention>,
        Ts::MarketRef: Into<Ts2::MarketRef>,
    {
        match self {
            Cashflow::FixedCoupon(c) => Cashflow::FixedCoupon(c.change_variable_types_to()),
            Cashflow::OvernightIndexCoupon(c) => {
                Cashflow::OvernightIndexCoupon(c.change_variable_types_to())
            }
        }
    }
}

// -----------------------------------------------------------------------------
// CashflowFixing
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CashflowFixing {
    OvernightIndexCoupon(OvernightIndexFixing),
}

// -----------------------------------------------------------------------------
// CashflowWithFixing
//
#[derive(Derivative)]
#[derivative(
    Debug(bound = "FixedCoupon<Ts>: std::fmt::Debug,
        OvernightIndexCoupon<Ts>: std::fmt::Debug"),
    Clone(bound = "FixedCoupon<Ts>: Clone,
        OvernightIndexCoupon<Ts>: Clone"),
    PartialEq(bound = "FixedCoupon<Ts>: PartialEq,
        OvernightIndexCoupon<Ts>: PartialEq")
)]
pub enum CashflowWithFixing<Ts: VariableTypes> {
    FixedCoupon(FixedCoupon<Ts>),
    OvernightIndexCoupon(OvernightIndexCoupon<Ts>, Option<OvernightIndexFixing>),
}

//
// methods
//
impl<Ts: VariableTypes> CashflowWithFixing<Ts> {
    #[inline]
    pub fn change_variable_types_to<Ts2: VariableTypes>(self) -> CashflowWithFixing<Ts2>
    where
        Ts::Number: Into<Ts2::Number>,
        Ts::Money: Into<Ts2::Money>,
        Ts::DateTime: Into<Ts2::DateTime>,
        Ts::DayCount: Into<Ts2::DayCount>,
        Ts::Rounding: Into<Ts2::Rounding>,
        Ts::InArrearsConvention: Into<Ts2::InArrearsConvention>,
        Ts::MarketRef: Into<Ts2::MarketRef>,
    {
        match self {
            CashflowWithFixing::FixedCoupon(c) => {
                CashflowWithFixing::FixedCoupon(c.change_variable_types_to())
            }
            CashflowWithFixing::OvernightIndexCoupon(c, f) => {
                CashflowWithFixing::OvernightIndexCoupon(c.change_variable_types_to(), f)
            }
        }
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use rstest::rstest;

    use crate::{
        product::general::{
            core::{Component, ComponentCategory, HasDependency, ValueLess, ValueOrId, WithId},
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

    fn cpn_base() -> CouponBase<VariableTypesForData> {
        CouponBase {
            notional: ValueOrId::Value(Money {
                amount: 100.0,
                ccy: Ccy::USD,
            }),
            entitle: ValueOrId::Id("entitle".into()),
            period_start: ValueOrId::Id("period_start".into()),
            period_end: ValueOrId::Id("period_end".into()),
            daycount: ValueOrId::Id("daycount".into()),
            payment: ValueOrId::Id("payment".into()),
        }
    }

    fn fixed() -> FixedCoupon<VariableTypesForData> {
        FixedCoupon {
            base: cpn_base(),
            accrual_daycount: ValueOrId::Id("accrual".into()),
            rate: ValueOrId::Value(0.05),
            rounding: ValueOrId::Id("rounding".into()).into(),
        }
    }

    fn ois_cpn() -> OvernightIndexCoupon<VariableTypesForData> {
        OvernightIndexCoupon {
            base: cpn_base(),
            convention: ValueOrId::Id("convention".into()),
            gearing: Some(ValueOrId::Value(1.0)),
            reference_rate: WithId {
                id: "reference_rate".into(),
                value: ValueLess,
            },
            spread: Some(ValueOrId::Value(0.01)),
            rounding: ValueOrId::Id("rounding".into()).into(),
        }
    }

    #[rstest]
    #[case(Cashflow::FixedCoupon(fixed()))]
    #[case(Cashflow::OvernightIndexCoupon(ois_cpn()))]
    fn test_cf_category(#[case] cf: Cashflow<VariableTypesForData>) {
        let cat = cf.category();

        assert_eq!(cat, ComponentCategory::Cashflow);
    }

    #[rstest]
    #[case(Cashflow::FixedCoupon(fixed()))]
    #[case(Cashflow::OvernightIndexCoupon(ois_cpn()))]
    fn test_cf_depends_on(#[case] cf: Cashflow<VariableTypesForData>) {
        let expected: HashSet<_> = match &cf {
            Cashflow::FixedCoupon(c) => c.depends_on().into_iter().collect(),
            Cashflow::OvernightIndexCoupon(c) => c.depends_on().into_iter().collect(),
        };

        let actual: HashSet<_> = cf.depends_on().into_iter().collect();

        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case(Cashflow::FixedCoupon(fixed()))]
    #[case(Cashflow::OvernightIndexCoupon(ois_cpn()))]
    fn test_cf_change_variable_types_to(#[case] cf: Cashflow<VariableTypesForData>) {
        let expected: Cashflow<OptVariableTypes> = match cf.clone() {
            Cashflow::FixedCoupon(c) => Cashflow::FixedCoupon(c.change_variable_types_to()),
            Cashflow::OvernightIndexCoupon(c) => {
                Cashflow::OvernightIndexCoupon(c.change_variable_types_to())
            }
        };

        let actual = cf.change_variable_types_to::<OptVariableTypes>();

        assert_eq!(actual, expected);
    }

    #[rstest]
    #[case(CashflowWithFixing::FixedCoupon(fixed()))]
    #[case(CashflowWithFixing::OvernightIndexCoupon(ois_cpn(), None))]
    #[case(CashflowWithFixing::OvernightIndexCoupon(ois_cpn(), OvernightIndexFixing { rate: 0.05}.into()))]
    fn test_cfwf_change_variable_types_to(#[case] cf: CashflowWithFixing<VariableTypesForData>) {
        let expected: CashflowWithFixing<OptVariableTypes> = match cf.clone() {
            CashflowWithFixing::FixedCoupon(c) => {
                CashflowWithFixing::FixedCoupon(c.change_variable_types_to())
            }
            CashflowWithFixing::OvernightIndexCoupon(c, f) => {
                CashflowWithFixing::OvernightIndexCoupon(c.change_variable_types_to(), f)
            }
        };

        let actual = cf.change_variable_types_to::<OptVariableTypes>();

        assert_eq!(actual, expected);
    }
}
