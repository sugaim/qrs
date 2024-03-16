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

use crate::products::general::core::VariableTypes;

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
