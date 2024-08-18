use qchrono::timepoint::DateTime;
use qfincore::{
    daycount::{Act365f, YearFrac},
    Yield,
};
use qmath::num::Scalar;

use super::super::YieldCurve;

// -----------------------------------------------------------------------------
// Joint
// -----------------------------------------------------------------------------
/// A combination of short term and long term yield curves.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct Joint<S, L = S> {
    pub switch_point: DateTime,
    pub short: S,
    pub long: L,
}

//
// methods
//
impl<S, L> YieldCurve for Joint<S, L>
where
    S: YieldCurve,
    L: YieldCurve<Value = S::Value>,
{
    type Value = S::Value;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Yield<Act365f, Self::Value>> {
        if to < from {
            return self.forward_rate(to, from);
        }
        if to < &self.switch_point {
            return self.short.forward_rate(from, to);
        }
        if &self.switch_point <= from {
            return self.long.forward_rate(from, to);
        }
        let short_contrib = self
            .short
            .forward_rate(from, &self.switch_point)?
            .to_ratio(from, &self.switch_point)
            .unwrap();
        let long_contrib = self
            .long
            .forward_rate(&self.switch_point, to)?
            .to_ratio(&self.switch_point, to)
            .unwrap();

        let total_contrib = short_contrib + &long_contrib;
        let dcf = Act365f.year_frac(from, to).unwrap();
        Ok(Yield {
            day_count: Act365f,
            value: total_contrib / &<S::Value as Scalar>::nearest_value_of_f64(dcf),
        })
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use crate::curve::atom::Flat;

    use super::*;

    #[rstest]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-01T00:00:00Z".parse().unwrap(), 0.01)]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-02T00:00:00Z".parse().unwrap(), 0.01)]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.015)]
    #[case("2021-01-01T00:00:00Z".parse().unwrap(), "2021-01-05T00:00:00Z".parse().unwrap(), 0.0175)]
    #[case("2021-01-02T00:00:00Z".parse().unwrap(), "2021-01-03T00:00:00Z".parse().unwrap(), 0.02)]
    fn test_forward_rate(#[case] stt: DateTime, #[case] end: DateTime, #[case] expected: f64) {
        let curve = Joint {
            short: Flat { rate: 0.01 },
            long: Flat { rate: 0.02 },
            switch_point: "2021-01-02T00:00:00Z".parse().unwrap(),
        };

        let res = curve.forward_rate(&stt, &end).unwrap().value;
        let rev = curve.forward_rate(&end, &stt).unwrap().value;

        approx::assert_abs_diff_eq!(expected, res, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(expected, rev, epsilon = 1e-10);
    }
}
