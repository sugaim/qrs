use qrs_core::{
    chrono::{DateTime, Rate},
    func1d::Func1dIntegrable,
    num::Real,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// ForwardRateCurve
//

/// A curve based on instant forward rates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InstFwdCurve<F> {
    /// Forward rate curve, which is a function from datetime to annualized(ACT/365Fixed) rate.
    #[serde(rename = "instant_forward_rate")]
    pub inst_fwd: F,
}

//
// methods
//
impl<F, V: Real> YieldCurve for InstFwdCurve<F>
where
    F: Func1dIntegrable<DateTime, Output = Rate<V>, Integrated = V>,
    F::Output: Real,
{
    type Value = V;
    type Error = anyhow::Error;

    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> anyhow::Result<Rate<Self::Value>> {
        if to < from {
            return self.forward_rate(to, from);
        }
        if from == to {
            return Ok(self.inst_fwd.eval(from));
        }
        let exponent = self.inst_fwd.integrate(from, to);
        Ok(Rate::new(exponent, to - from))
    }
}
