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
// construction
//
impl<Ts: VariableTypes> CashflowWithFixing<Ts> {
    pub fn try_combine(cf: Cashflow<Ts>, fixing: Option<CashflowFixing>) -> Option<Self> {
        use Cashflow as Cf;
        use CashflowFixing as Fix;

        match (cf, fixing) {
            (Cf::FixedCoupon(c), None) => Some(Self::FixedCoupon(c)),
            (Cf::OvernightIndexCoupon(c), Some(Fix::OvernightIndexCoupon(f))) => {
                Some(Self::OvernightIndexCoupon(c, Some(f)))
            }
            _ => None,
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
