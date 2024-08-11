use qchrono::{duration::Tenor, ext::chrono::offset::LocalResult, timepoint::DateTime};
use qfincore::{daycount::Act365f, Yield};
use qmath::num::Real;

use crate::curve::YieldCurve;

use super::YieldCurveAdj;

// -----------------------------------------------------------------------------
// Lookback
// -----------------------------------------------------------------------------
#[derive(
    Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
pub struct Lookback {
    pub tenor: Tenor,
}

//
// methods
//
impl<V: Real> YieldCurveAdj<V> for Lookback {
    #[inline]
    fn adjusted_forward_rate<Y: YieldCurve<Value = V>>(
        &self,
        curve: &Y,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, V>> {
        let add_tenor = |dt: &DateTime| match dt.add_tenor(-self.tenor) {
            LocalResult::Single(dt) => Ok(dt),
            LocalResult::None => anyhow::bail!(
                "Add tenor result does not exist due to timezone issue. dt: {}, tenor: {}",
                dt,
                self.tenor
            ),
            LocalResult::Ambiguous(_, _) => anyhow::bail!(
                "Add tenor result is ambiguous due to timezone issue. dt: {}, tenor: {}",
                dt,
                self.tenor
            ),
        };
        curve.forward_rate(&add_tenor(from)?, &add_tenor(to)?)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::curve::{atom::Flat, composite::Joint};

    use super::*;

    #[rstest]
    #[case("P0D".parse().unwrap(), 0.02)]
    #[case("P1D".parse().unwrap(), 0.02)]
    #[case("P2D".parse().unwrap(), 0.015)]
    #[case("P3D".parse().unwrap(), 0.01)]
    #[case("P4D".parse().unwrap(), 0.01)]
    fn test_adj(#[case] tenor: Tenor, #[case] expected: f64) {
        let curve = Joint {
            switch_point: "2021-01-02T00:00:00Z".parse().unwrap(),
            short: Flat { rate: 0.01 },
            long: Flat { rate: 0.02 },
        };
        let stt = "2021-01-03T00:00:00Z".parse().unwrap();
        let end = "2021-01-05T00:00:00Z".parse().unwrap();
        let adj = Lookback { tenor };

        let res = adj.adjusted_forward_rate(&curve, &stt, &end).unwrap();
        let rev = adj.adjusted_forward_rate(&curve, &end, &stt).unwrap();

        approx::assert_abs_diff_eq!(res.value, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(res.value, rev.value, epsilon = 1e-10);
    }

    #[rstest]
    #[case(
        "2021-01-01T00:00:00Z".parse().unwrap(),
        "2023-03-13T01:30:00-05:00[America/New_York]".parse().unwrap(),
        "P1D".parse().unwrap()
    )]
    #[case(
        "2023-03-13T01:30:00-05:00[America/New_York]".parse().unwrap(),
        "2026-05-01T00:00:00Z".parse().unwrap(),
        "P1D".parse().unwrap()
    )]
    #[case(
        "2021-01-01T00:00:00Z".parse().unwrap(),
        "2023-11-06T02:30:00-04:00[America/New_York]".parse().unwrap(),
        "P1D".parse().unwrap()
    )]
    #[case(
        "2023-11-06T02:30:00-04:00[America/New_York]".parse().unwrap(),
        "2025-01-01T00:00:00Z".parse().unwrap(),
        "P1D".parse().unwrap()
    )]
    fn test_adj_err(#[case] stt: DateTime, #[case] end: DateTime, #[case] tenor: Tenor) {
        let crv = Flat { rate: 0.01 };
        let adj = Lookback { tenor };

        let res = adj.adjusted_forward_rate(&crv, &stt, &end);

        assert!(res.is_err());
    }
}
