use std::ops::Mul;

use qrs_chrono::{DateTime, Duration};
use qrs_finance::daycount::{Act365f, Act365fRate, DayCountRate, Rate};
use qrs_math::{func1d::Func1dDer1, num::Real};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// ZeroRateCurve
//
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize, serde::Serialize, schemars::JsonSchema)
)]
pub struct ZeroRateCurve<F> {
    /// Zero rate curve, which is a function from datetime to annualized(ACT/365Fixed) rate.
    pub zero_rate: F,

    /// Base date of zero rates.
    pub base_date: DateTime,
}

//
// methods
//
impl<F, V: Real> YieldCurve for ZeroRateCurve<F>
where
    F: Func1dDer1<DateTime, Output = Act365fRate<V>>,
    F::Der1: Mul<Duration, Output = Act365fRate<V>>,
{
    type Value = V;

    fn forward_rate(
        &self,
        from: &DateTime,
        to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        if to < from {
            return self.forward_rate(to, from);
        }
        if from == to {
            // with f = from, t = to, b = self.base_date, z = self.zero_rate, we have
            // forward(f, t)
            //   = (z(t) * (t - b) - z(f) * (f - b)) / (t - f)
            //   = [(z(t) - z(f)) * (t - b) + z(f) * (t - b) - z(f) * (f - b)] / (t -f)
            //   = (z(t) - z(f)) / (t - f) * (t - b) + z(f)
            //   -> z'(f) * (f - b) + z(f) as t -> f
            let (zf, zp) = self.zero_rate.der01(from);
            return Ok(zf + &(zp * (from - self.base_date)));
        }
        let zf = self.zero_rate.eval(from);
        let zt = self.zero_rate.eval(to);

        let ef = zf.into_ratio_between(&self.base_date, from);
        let et = zt.into_ratio_between(&self.base_date, to);

        // (zt * t - zf * f) / (t - f)
        Ok(Act365f
            .ratio_to_rate(et - &ef, from, to)
            .expect("zero-division does not occur"))
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    use approx::assert_abs_diff_eq;

    #[test]
    fn test_forward_rate() {
        let dt_builder = qrs_chrono::DateTimeBuilder::new()
            .with_timezone(qrs_chrono::Tz::fixed_offset(0).unwrap())
            .with_hms(0, 0, 0);
        let zero_rate = qrs_math::interp1d::Lerp1d::new(
            vec![
                dt_builder.clone().with_ymd(2021, 1, 6).build().unwrap(),
                dt_builder.clone().with_ymd(2021, 1, 11).build().unwrap(),
                dt_builder.clone().with_ymd(2021, 1, 16).build().unwrap(),
            ],
            vec![
                Act365fRate::from_rate(0.05f64),
                Act365fRate::from_rate(0.03f64),
                Act365fRate::from_rate(0.02f64),
            ],
        )
        .unwrap();
        let curve = super::ZeroRateCurve {
            zero_rate,
            base_date: dt_builder.clone().with_ymd(2021, 1, 1).build().unwrap(),
        };

        let from = dt_builder.clone().with_ymd(2021, 1, 1).build().unwrap();
        let to = dt_builder.clone().with_ymd(2021, 1, 6).build().unwrap();
        let fwd = curve.forward_rate(&from, &to).unwrap();
        assert_abs_diff_eq!(fwd.into_value(), 0.05, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from).unwrap());

        let from = dt_builder.clone().with_ymd(2021, 1, 6).build().unwrap();
        let to = dt_builder.clone().with_ymd(2021, 1, 11).build().unwrap();
        let fwd = curve.forward_rate(&from, &to).unwrap();
        assert_abs_diff_eq!(fwd.into_value(), 0.01, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from).unwrap());

        let from = dt_builder.clone().with_ymd(2021, 1, 11).build().unwrap();
        let to = dt_builder.clone().with_ymd(2021, 1, 16).build().unwrap();
        let fwd = curve.forward_rate(&from, &to).unwrap();
        assert_abs_diff_eq!(fwd.into_value(), 0.0, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from).unwrap());

        let from = dt_builder.clone().with_ymd(2021, 1, 6).build().unwrap();
        let to = dt_builder.clone().with_ymd(2021, 1, 16).build().unwrap();
        let fwd = curve.forward_rate(&from, &to).unwrap();
        assert_abs_diff_eq!(fwd.into_value(), 0.005, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from).unwrap());

        let from = dt_builder.clone().with_ymd(2021, 1, 13).build().unwrap();
        let to = dt_builder.clone().with_ymd(2021, 1, 13).build().unwrap();
        let fwd = curve.forward_rate(&from, &to).unwrap();
        assert_abs_diff_eq!(
            fwd.into_value(),
            0.026 + (-0.002) * 12.0, // 0.026 = z(13d), -0.002 = z'(13d), 12.0 = (13d - 1d)
            epsilon = 1e-10
        );
    }
}
