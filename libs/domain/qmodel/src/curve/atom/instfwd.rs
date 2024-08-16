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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use qchrono::timepoint::DateTime;
    use qfincore::{
        daycount::{Act365f, YearFrac},
        Yield,
    };
    use qmath::num::{Func1d, Integrable1d};
    use rstest::rstest;

    use crate::curve::YieldCurve;

    use super::Instfwd;

    struct X;

    impl Func1d<DateTime> for X {
        type Output = Yield<Act365f, f64>;
        type Error = anyhow::Error;

        fn eval(&self, x: &DateTime) -> Result<Self::Output, Self::Error> {
            let today = DateTime::from_str("2021-01-01T00:00:00Z").unwrap();
            let year = Act365f.year_frac(&today, x).unwrap();
            Ok(Yield {
                day_count: Act365f,
                value: year,
            })
        }
    }

    impl Integrable1d<DateTime> for X {
        type Integrated = f64;

        fn integrate(
            &self,
            from: &DateTime,
            to: &DateTime,
        ) -> Result<Self::Integrated, Self::Error> {
            let today = DateTime::from_str("2021-01-01T00:00:00Z").unwrap();
            let fyear = Act365f.year_frac(&today, from).unwrap();
            let tyear = Act365f.year_frac(&today, to).unwrap();
            Ok((tyear.powi(2) - fyear.powi(2)) * 0.5)
        }
    }

    #[rstest]
    #[case(DateTime::from_str("2021-01-01T00:00:00Z").unwrap())]
    #[case(DateTime::from_str("2021-01-02T00:00:00Z").unwrap())]
    #[case(DateTime::from_str("2021-02-01T00:00:00Z").unwrap())]
    #[case(DateTime::from_str("2022-01-01T00:00:00Z").unwrap())]
    #[case(DateTime::from_str("2020-01-01T00:00:00Z").unwrap())]
    fn test_inst_forward_rate(#[case] t: DateTime) {
        let instfwd = Instfwd { inst_fwd: X };

        let rate = instfwd.forward_rate(&t, &t).unwrap();

        assert_eq!(rate.day_count, Act365f);
        assert_eq!(rate.value, X.eval(&t).unwrap().value);
    }

    #[rstest]
    #[case(
        DateTime::from_str("2021-01-01T00:00:00Z").unwrap(),
        DateTime::from_str("2021-01-02T00:00:00Z").unwrap(),
        (1. / 365.) * 0.5
    )]
    #[case(
        DateTime::from_str("2021-01-05T00:00:00Z").unwrap(),
        DateTime::from_str("2021-01-10T00:00:00Z").unwrap(),
        (13. / 365.) * 0.5
    )]
    fn test_forward_rate(#[case] s: DateTime, #[case] t: DateTime, #[case] expected: f64) {
        let instfwd = Instfwd { inst_fwd: X };

        let rate = instfwd.forward_rate(&s, &t).unwrap();
        let inv = instfwd.forward_rate(&t, &s).unwrap();

        assert_eq!(rate.day_count, Act365f);
        assert_eq!(inv.day_count, Act365f);
        approx::assert_abs_diff_eq!(rate.value, expected, epsilon = 1e-10);
        approx::assert_abs_diff_eq!(rate.value, inv.value, epsilon = 1e-10);
    }
}
