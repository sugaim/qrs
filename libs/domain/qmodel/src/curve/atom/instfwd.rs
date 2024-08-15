use qchrono::timepoint::DateTime;
use qfincore::{
    daycount::{Act365f, YearFrac},
    Yield,
};
use qmath::num::{Integrable1d, Real};

use crate::curve::YieldCurve;

// -----------------------------------------------------------------------------
// Instfwd
// -----------------------------------------------------------------------------
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Instfwd<C> {
    pub inst_fwd: C,
}

impl<V, C> YieldCurve for Instfwd<C>
where
    V: Real,
    C: Integrable1d<DateTime, Output = Yield<Act365f, V>, Integrated = V>,
    anyhow::Error: From<C::Error>,
{
    type Value = V;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        if from == to {
            return self.inst_fwd.eval(from).map_err(Into::into);
        }
        let integrated = self.inst_fwd.integrate(from, to).map_err(Into::into)?;
        let dcf = Act365f.year_frac(from, to).unwrap();
        Ok(Yield {
            day_count: Act365f,
            value: integrated / &V::nearest_base_float_of_f64(dcf),
        })
    }
}
