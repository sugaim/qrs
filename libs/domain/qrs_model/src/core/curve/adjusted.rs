use qrs_finance::daycount::Act365fRate;

use super::{YieldCurve, YieldCurveAdjust};

// -----------------------------------------------------------------------------
// AdjustedCurve
//
#[derive(Debug, Clone, PartialEq)]
pub struct AdjustedCurve<C, A> {
    pub base: C,
    pub adjustments: Vec<A>,
}

//
// methods
//

impl<C, A> YieldCurve for AdjustedCurve<C, A>
where
    C: YieldCurve,
    A: YieldCurveAdjust<C::Value>,
{
    type Value = C::Value;

    fn forward_rate(
        &self,
        from: &qrs_chrono::DateTime,
        to: &qrs_chrono::DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        let adjusted = _AdjustedCurve {
            base: &self.base,
            adjusters: &self.adjustments,
        };
        adjusted.forward_rate(from, to)
    }
}

// -----------------------------------------------------------------------------
// _AdjustedCurve
//

struct _AdjustedCurve<'a, C, A> {
    pub base: &'a C,
    pub adjusters: &'a [A],
}

impl<'a, C, A> YieldCurve for _AdjustedCurve<'a, C, A>
where
    C: YieldCurve,
    A: YieldCurveAdjust<C::Value>,
{
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &qrs_chrono::DateTime,
        to: &qrs_chrono::DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        match self.adjusters.split_last() {
            Some((tail, rem)) => {
                let next = _AdjustedCurve {
                    base: self.base,
                    adjusters: rem,
                };
                tail.adjusted_forward_rate(&next, from, to)
            }
            None => self.base.forward_rate(from, to),
        }
    }
}
