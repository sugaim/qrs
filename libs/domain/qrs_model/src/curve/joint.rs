use qrs_core::{
    chrono::{DateTime, Rate},
    num::Real,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// JointCurve
//

/// A curve which has the different behavior between short/long term.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct JointCurve<S, L> {
    #[serde(rename = "short_term_curve")]
    pub short: S,
    #[serde(rename = "long_term_curve")]
    pub long: L,
    #[serde(rename = "branching_point")]
    pub sep: DateTime,
}

//
// methods
//
impl<V, S, L> YieldCurve for JointCurve<S, L>
where
    V: Real,
    S: YieldCurve<Value = V>,
    L: YieldCurve<Value = V>,
{
    type Value = V;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<qrs_core::chrono::Rate<V>> {
        if to < from {
            return self.forward_rate(to, from);
        }
        if to <= &self.sep {
            return self.short.forward_rate(from, to);
        }
        if &self.sep <= from {
            return self.long.forward_rate(from, to);
        }
        // mixed
        let short = self.short.forward_rate(from, &self.sep)?;
        let long = self.long.forward_rate(&self.sep, to)?;
        let exponent = short * (self.sep - from) + long * (to - self.sep);
        Ok(Rate::new(exponent, to - from))
    }
}
