mod base;
mod fixed_coupon;
mod overnight_index_coupon;

use derivative::Derivative;
use qrs_finance_derive::Component;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use base::CouponBase;
pub use fixed_coupon::FixedCoupon;
pub use overnight_index_coupon::OvernightIndexCoupon;

use crate::products::general::core::VariableTypes;

// -----------------------------------------------------------------------------
// Cashflow
//
#[derive(Derivative, Component, Serialize, Deserialize, JsonSchema)]
#[component(_use_from_qrs_finance)]
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
