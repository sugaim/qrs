use qrs_chrono::{DateTime, Duration, Velocity};
use qrs_finance::daycount::{Act365f, RateAct365f, RateDayCount};
use qrs_math::{func1d::Func1dDer1, num::Real};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// LogDfCurve
//
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema)
)]
pub struct LogDfCurve<F> {
    pub logdf: F,
}

//
// methods
//
impl<F, V> YieldCurve for LogDfCurve<F>
where
    F: Func1dDer1<DateTime, Output = V, Der1 = Velocity<V>>,
    V: Real,
{
    type Value = V;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<RateAct365f<Self::Value>> {
        if to < from {
            return self.forward_rate(to, from);
        }
        if from == to {
            let rate = self.logdf.der1(from).to_change(Duration::with_days(365));
            return Ok(Act365f.to_rate(rate));
        }
        let log_df = self.logdf.eval(from) - &self.logdf.eval(to);
        Ok(Act365f
            .ratio_to_rate(log_df, from, to)
            .expect("zero-division does not occur"))
    }
}
