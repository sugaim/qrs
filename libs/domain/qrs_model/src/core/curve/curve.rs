use std::ops::Div;

use qrs_chrono::{DateTime, Duration, Velocity};
use qrs_fincore::daycount::Act365fRate;
use qrs_math::{
    interp1d::{CHermite1d, CatmullRomScheme, Lerp1d, PwConst1d},
    num::{Real, RelPos},
};

use super::{FlatCurve, InstFwdCurve, LogDfCurve, YieldCurve, ZeroRateCurve};

// -----------------------------------------------------------------------------
// ComponentCurve
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize, schemars::JsonSchema),
    serde(bound(
        serialize = "V: Real<BaseFloat = <DateTime as RelPos>::Output> + serde::Serialize",
        deserialize = "V: 'static + Real<BaseFloat = <DateTime as RelPos>::Output> + Div<Duration, Output = Velocity<V>> + serde::Deserialize<'de>"
    )),
    serde(tag = "type", rename_all = "snake_case")
)]
pub enum ComponentCurve<V> {
    Flat(FlatCurve<V>),
    LogLerp(LogDfCurve<Lerp1d<DateTime, V>>),
    LogCR(LogDfCurve<CHermite1d<DateTime, V, CatmullRomScheme>>),
    ZeroRateLerp(ZeroRateCurve<Lerp1d<DateTime, Act365fRate<V>>>),
    ZeroRateCr(ZeroRateCurve<CHermite1d<DateTime, Act365fRate<V>, CatmullRomScheme>>),
    InstFwdLerp(InstFwdCurve<Lerp1d<DateTime, Act365fRate<V>>>),
    InstFwdPwConst(InstFwdCurve<PwConst1d<DateTime, Act365fRate<V>>>),
}

//
// methods
//
impl<V: Real<BaseFloat = <DateTime as RelPos>::Output>> YieldCurve for ComponentCurve<V>
where
    V: Div<Duration, Output = Velocity<V>>,
{
    type Value = V;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        use ComponentCurve::*;
        match self {
            Flat(c) => c.forward_rate(from, to),
            LogLerp(c) => c.forward_rate(from, to),
            LogCR(c) => c.forward_rate(from, to),
            ZeroRateLerp(c) => c.forward_rate(from, to),
            ZeroRateCr(c) => c.forward_rate(from, to),
            InstFwdLerp(c) => c.forward_rate(from, to),
            InstFwdPwConst(c) => c.forward_rate(from, to),
        }
    }
}
