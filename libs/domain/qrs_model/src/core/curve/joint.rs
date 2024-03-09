use qrs_chrono::DateTime;
use qrs_finance::core::daycount::{Act365f, Act365fRate, DayCountRate};
use qrs_math::num::Real;

use super::YieldCurve;

// -----------------------------------------------------------------------------
// JointCurve
//

/// A curve which has the different behavior between short/long term.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
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

    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> anyhow::Result<Act365fRate<V>> {
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
        Ok(Act365f
            .ratio_to_rate(exponent, from, to)
            .expect("zero-division does not occur"))
    }
}
