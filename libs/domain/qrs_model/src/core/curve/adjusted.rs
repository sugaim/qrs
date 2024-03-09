use qrs_finance::core::daycount::Act365fRate;

use super::{YieldCurve, YieldCurveAdjust};

// -----------------------------------------------------------------------------
// AdjustedCurve
//
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustedCurve<C, A> {
    pub base: C,
    pub adjuster: A,
}

//
// methods
//
impl<'a, C, A> YieldCurve for AdjustedCurve<&'a C, &'a [A]>
where
    C: YieldCurve,
    A: YieldCurveAdjust<Self> + YieldCurveAdjust<C>,
{
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &qrs_chrono::DateTime,
        to: &qrs_chrono::DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        match self.adjuster.split_last() {
            Some((tail, rem)) => {
                let next = AdjustedCurve {
                    base: self.base,
                    adjuster: rem,
                };
                tail.adjusted_forward_rate(&next, from, to)
            }
            None => self.base.forward_rate(from, to),
        }
    }
}

impl<C, A> YieldCurve for AdjustedCurve<C, Vec<A>>
where
    C: YieldCurve,
    A: YieldCurveAdjust<C>,
    for<'a> A: YieldCurveAdjust<AdjustedCurve<&'a C, &'a [A]>>,
{
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &qrs_chrono::DateTime,
        to: &qrs_chrono::DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        let adjusted = AdjustedCurve {
            base: &self.base,
            adjuster: self.adjuster.as_slice(),
        };
        adjusted.forward_rate(from, to)
    }
}
