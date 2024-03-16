use derivative::Derivative;
use qrs_chrono::DateTime;
use qrs_finance::daycount::Act365fRate;
use qrs_math::num::Real;

use crate::core::curve::{Bump, Shift, YieldCurve, YieldCurveAdjust};

// -----------------------------------------------------------------------------
// IrCurveAdjust
//
#[derive(Debug, Clone, Derivative)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(tag = "type", rename_all = "snake_case"),
    serde(bound(
        serialize = "V: qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat> + serde::Serialize",
        deserialize = "V: qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat> + serde::Deserialize<'de>"
    ))
)]
#[derivative(PartialEq(
    bound = "V: PartialOrd + qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat>"
))]
pub enum IrCurveAdjust<V> {
    Bump(Bump<V>),
    Shift(Shift),
}

//
// methods
//
impl<V: Real> YieldCurveAdjust<V> for IrCurveAdjust<V> {
    #[inline]
    fn adjusted_forward_rate<C: YieldCurve<Value = V>>(
        &self,
        curve: &C,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<C::Value>> {
        match self {
            IrCurveAdjust::Bump(bump) => bump.adjusted_forward_rate(curve, from, to),
            IrCurveAdjust::Shift(shift) => shift.adjusted_forward_rate(curve, from, to),
        }
    }
}
