use qrs_core::{
    chrono::{DateTime, Rate},
    func1d::{Func1d, Func1dDer1},
    num::Real,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::YieldCurve;

// -----------------------------------------------------------------------------
// ZeroRateCurve
//
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ZeroRateCurve<F> {
    /// Zero rate curve, which is a function from datetime to annualized(ACT/365Fixed) rate.
    pub zero_rate: F,

    /// Base date of zero rates.
    pub base_date: DateTime,
}

//
// methods
//
impl<F> YieldCurve for ZeroRateCurve<F>
where
    F: Func1dDer1<DateTime, Der1 = Rate<<F as Func1d<DateTime>>::Output>>,
    <F as Func1d<DateTime>>::Output: Real,
{
    type Value = <F as Func1d<DateTime>>::Output;
    type Error = anyhow::Error;

    fn forward_rate(&self, from: &DateTime, to: &DateTime) -> Rate<Self::Value> {
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
            return Rate::with_annual(zf + &(zp * (from - self.base_date)));
        }
        let zf = Rate::with_annual(self.zero_rate.eval(from));
        let zt = Rate::with_annual(self.zero_rate.eval(to));

        let durf = from - self.base_date;
        let durt = to - self.base_date;
        let dur = to - from;

        Rate::new(zt * durt - &(zf * durf), dur)
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use super::YieldCurve;
    use approx::assert_abs_diff_eq;

    #[test]
    fn test_forward_rate() {
        let dt_builder = qrs_core::chrono::DateTimeBuilder::new()
            .with_timezone(qrs_core::chrono::TimeZone::FixedOffset(
                chrono::FixedOffset::east_opt(0).unwrap(),
            ))
            .with_hms(0, 0, 0)
            .unwrap();
        let zero_rate = qrs_core::interp1d::Lerp1d::new(
            vec![
                dt_builder.with_ymd(2021, 1, 6).unwrap().build(),
                dt_builder.with_ymd(2021, 1, 11).unwrap().build(),
                dt_builder.with_ymd(2021, 1, 16).unwrap().build(),
            ],
            vec![0.05f64, 0.03f64, 0.02f64],
        )
        .unwrap();
        let curve = super::ZeroRateCurve {
            zero_rate,
            base_date: dt_builder.with_ymd(2021, 1, 1).unwrap().build(),
        };

        let from = dt_builder.with_ymd(2021, 1, 1).unwrap().build();
        let to = dt_builder.with_ymd(2021, 1, 6).unwrap().build();
        let fwd = curve.forward_rate(&from, &to);
        assert_abs_diff_eq!(fwd.to_annual_change(), 0.05, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from));

        let from = dt_builder.with_ymd(2021, 1, 6).unwrap().build();
        let to = dt_builder.with_ymd(2021, 1, 11).unwrap().build();
        let fwd = curve.forward_rate(&from, &to);
        assert_abs_diff_eq!(fwd.to_annual_change(), 0.01, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from));

        let from = dt_builder.with_ymd(2021, 1, 11).unwrap().build();
        let to = dt_builder.with_ymd(2021, 1, 16).unwrap().build();
        let fwd = curve.forward_rate(&from, &to);
        assert_abs_diff_eq!(fwd.to_annual_change(), 0.0, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from));

        let from = dt_builder.with_ymd(2021, 1, 6).unwrap().build();
        let to = dt_builder.with_ymd(2021, 1, 16).unwrap().build();
        let fwd = curve.forward_rate(&from, &to);
        assert_abs_diff_eq!(fwd.to_annual_change(), 0.005, epsilon = 1e-10);
        assert_eq!(fwd, curve.forward_rate(&to, &from));

        let from = dt_builder.with_ymd(2021, 1, 13).unwrap().build();
        let to = dt_builder.with_ymd(2021, 1, 13).unwrap().build();
        let fwd = curve.forward_rate(&from, &to);
        assert_abs_diff_eq!(
            fwd.to_annual_change(),
            0.026 + (-0.002) * 12.0, // 0.026 = z(13d), -0.002 = z'(13d), 12.0 = (13d - 1d)
            epsilon = 1e-10
        );
    }
}
