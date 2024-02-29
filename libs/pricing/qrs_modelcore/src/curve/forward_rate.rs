use qrs_chrono::DateTime;
use qrs_finance::rate::RateAct365f;
use qrs_math::{func1d::Func1dIntegrable, num::Real};
#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// ForwardRateCurve
//

/// A curve based on instant forward rates.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, JsonSchema))]
pub struct InstFwdCurve<F> {
    /// Forward rate curve, which is a function from datetime to annualized(ACT/365Fixed) rate.
    #[cfg_attr(feature = "serde", serde(rename = "instant_forward_rate"))]
    pub inst_fwd: F,
}

//
// methods
//
impl<F, V: Real> YieldCurve for InstFwdCurve<F>
where
    F: Func1dIntegrable<DateTime, Output = RateAct365f<V>, Integrated = V>,
{
    type Value = V;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<RateAct365f<Self::Value>> {
        if to < from {
            return self.forward_rate(to, from);
        }
        if from == to {
            return Ok(self.inst_fwd.eval(from));
        }
        let exponent = self.inst_fwd.integrate(from, to);
        Ok(RateAct365f::from_ratio(exponent, to - from).expect("zero-division does not occur"))
    }
}
