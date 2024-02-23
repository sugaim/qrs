use num::Zero;
use qrs_core::{
    chrono::{DateTime, Rate},
    num::Scalar,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// Component
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Component<C> {
    pub weight: f64,
    pub curve: C,
}

//
// methods
//
impl<C: YieldCurve> YieldCurve for Component<C> {
    type Value = C::Value;
    type Error = C::Error;

    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> anyhow::Result<Rate<Self::Value>> {
        Ok(self.curve.forward_rate(from, to)?
            * &<C::Value as Scalar>::nearest_value_of(self.weight))
    }
}

// -----------------------------------------------------------------------------
// CompositeCurve
//
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CompositeCurve<C> {
    pub components: Vec<Component<C>>,
}

//
// methods
//
impl<C: YieldCurve> YieldCurve for CompositeCurve<C> {
    type Value = C::Value;
    type Error = C::Error;

    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> anyhow::Result<Rate<Self::Value>> {
        let mut sum = Rate::zero();
        for c in &self.components {
            let r = c.forward_rate(from, to)?;
            sum += r;
        }
        Ok(sum)
    }
}
