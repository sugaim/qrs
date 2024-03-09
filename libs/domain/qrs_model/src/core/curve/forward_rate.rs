use qrs_chrono::DateTime;
use qrs_finance::core::daycount::{Act365f, Act365fRate, DayCountRate};
use qrs_math::{func1d::Func1dIntegrable, num::Real};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// ForwardRateCurve
//

/// A curve based on instant forward rates.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
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
    F: Func1dIntegrable<DateTime, Output = Act365fRate<V>, Integrated = V>,
{
    type Value = V;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        if to < from {
            return self.forward_rate(to, from);
        }
        if from == to {
            return Ok(self.inst_fwd.eval(from));
        }
        let exponent = self.inst_fwd.integrate(from, to);
        Ok(Act365f
            .ratio_to_rate(exponent, from, to)
            .expect("zero-division does not occur"))
    }
}
