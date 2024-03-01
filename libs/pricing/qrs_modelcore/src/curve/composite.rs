use qrs_chrono::DateTime;
use qrs_finance::rate::RateAct365f;
use qrs_math::num::{FloatBased, Zero};
#[cfg(feature = "serde")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// WeightedCurve
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, JsonSchema))]
pub struct WeightedCurve<C> {
    pub weight: f64,
    pub curve: C,
}

//
// methods
//
impl<C: YieldCurve> YieldCurve for WeightedCurve<C> {
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<RateAct365f<Self::Value>> {
        Ok(self.curve.forward_rate(from, to)?
            * &<C::Value as FloatBased>::nearest_base_float_of(self.weight))
    }
}

// -----------------------------------------------------------------------------
// CompositeCurve
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize, JsonSchema))]
pub struct CompositeCurve<C> {
    pub components: Vec<WeightedCurve<C>>,
}

//
// methods
//
impl<C: YieldCurve> YieldCurve for CompositeCurve<C> {
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<RateAct365f<Self::Value>> {
        let mut sum = Zero::zero();
        for c in &self.components {
            let r = c.forward_rate(from, to)?;
            sum += &r;
        }
        Ok(sum)
    }
}
