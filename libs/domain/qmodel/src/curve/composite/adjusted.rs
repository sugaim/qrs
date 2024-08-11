use qchrono::timepoint::DateTime;
use qfincore::{daycount::Act365f, Yield};

use super::super::{adj::YieldCurveAdj, YieldCurve};

// -----------------------------------------------------------------------------
// Adjusted
// -----------------------------------------------------------------------------
#[derive(
    Debug, Clone, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Adjusted<C, V> {
    pub curve: C,
    pub adjustment: Vec<V>,
}

//
// ctor
//
impl<C, V> Adjusted<C, V> {
    /// Create a new adjusted curve.
    ///
    /// The second argument is adjustments which will be applied.
    /// The order of application is from the first element to the last.
    #[inline]
    pub fn new(curve: C, adjustment: Vec<V>) -> Self
    where
        C: YieldCurve,
        V: YieldCurveAdj<C::Value>,
    {
        Adjusted { curve, adjustment }
    }
}

//
// methods
//
struct _AdjCurve<'a, C, V> {
    curve: &'a C,
    adjustment: &'a [V],
}

impl<'a, C, V> YieldCurve for _AdjCurve<'a, C, V>
where
    C: YieldCurve,
    V: YieldCurveAdj<C::Value>,
{
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        match self.adjustment.split_last() {
            None => self.curve.forward_rate(from, to),
            Some((last, adjustment)) => {
                let curve = _AdjCurve {
                    curve: self.curve,
                    adjustment,
                };
                last.adjusted_forward_rate(&curve, from, to)
            }
        }
    }
}

impl<C, V> YieldCurve for Adjusted<C, V>
where
    C: YieldCurve,
    V: YieldCurveAdj<C::Value>,
{
    type Value = C::Value;

    #[inline]
    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        let curve = _AdjCurve {
            curve: &self.curve,
            adjustment: &self.adjustment,
        };
        curve.forward_rate(from, to)
    }
}

#[cfg(test)]
mod tests {
    use qchrono::duration::Tenor;

    use crate::curve::{
        adj::{Bump, Lookback},
        composite::Joint,
    };

    use super::*;

    enum Adj {
        Lookback(Lookback),
        Bump(Bump<f64>),
    }

    impl YieldCurveAdj<f64> for Adj {
        fn adjusted_forward_rate<Y: YieldCurve<Value = f64>>(
            &self,
            curve: &Y,
            from: &DateTime,
            to: &DateTime,
        ) -> anyhow::Result<Yield<Act365f, f64>> {
            match self {
                Adj::Lookback(adj) => adj.adjusted_forward_rate(curve, from, to),
                Adj::Bump(adj) => adj.adjusted_forward_rate(curve, from, to),
            }
        }
    }

    #[test]
    fn test_forward_rate_adj_order() {
        let base = Joint {
            switch_point: "2021-01-04T00:00:00Z".parse().unwrap(),
            short: crate::curve::atom::Flat { rate: 0.01 },
            long: crate::curve::atom::Flat { rate: 0.02 },
        };
        let adjs = vec![
            Adj::Lookback(Lookback {
                tenor: Tenor::Days(1),
            }),
            Adj::Bump(Bump::with_from(
                0.03,
                "2021-01-04T00:00:00Z".parse().unwrap(),
            )),
        ];
        let curve = Adjusted::new(base, adjs);
        let stt = "2021-01-04T00:00:00Z".parse().unwrap();
        let end = "2021-01-05T00:00:00Z".parse().unwrap();

        let res = curve.forward_rate(&stt, &end).unwrap();

        // if bump is applied at first, the result should be 0.01
        approx::assert_abs_diff_eq!(res.value, 0.04, epsilon = 1e-10);
    }
}
