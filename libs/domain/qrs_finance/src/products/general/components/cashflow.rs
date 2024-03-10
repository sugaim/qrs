mod base;
mod fixed_coupon;
mod overnight_index_coupon;

pub use base::CouponBase;
pub use fixed_coupon::FixedCoupon;
pub use overnight_index_coupon::OvernightIndexCoupon;
use qrs_finance_derive::Component;

use crate::products::general::VariableTypes;

// -----------------------------------------------------------------------------
// Cashflow
//
#[derive(Clone, Debug, PartialEq, Component)]
#[component(_use_from_qrs_finance)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "FixedCoupon<Ts>: serde::Serialize,
            OvernightIndexCoupon<Ts>: serde::Serialize",
        deserialize = "FixedCoupon<Ts>: serde::Deserialize<'de>,
            OvernightIndexCoupon<Ts>: serde::Deserialize<'de>"
    )),
    serde(tag = "type", rename_all = "snake_case"),
    schemars(bound = "Ts: schemars::JsonSchema,
        FixedCoupon<Ts>: schemars::JsonSchema,
        OvernightIndexCoupon<Ts>: schemars::JsonSchema")
)]
pub enum Cashflow<Ts: VariableTypes> {
    FixedCoupon(FixedCoupon<Ts>),
    OvernightIndexCoupon(OvernightIndexCoupon<Ts>),
}
