use qrs_chrono::DateTime;
use qrs_finance::rate::RateAct365f;
use qrs_math::num::Real;
#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// JointCurve
//

/// A curve which has the different behavior between short/long term.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, JsonSchema))]
pub struct JointCurve<S, L> {
    #[cfg_attr(feature = "serde", serde(rename = "short_term_curve"))]
    pub short: S,
    #[cfg_attr(feature = "serde", serde(rename = "long_term_curve"))]
    pub long: L,
    #[cfg_attr(feature = "serde", serde(rename = "branching_point"))]
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

    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> anyhow::Result<RateAct365f<V>> {
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
        Ok(RateAct365f::from_ratio(exponent, to - from).expect("zero-division does not occur"))
    }
}
