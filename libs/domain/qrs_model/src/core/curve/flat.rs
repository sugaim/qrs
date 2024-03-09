// -----------------------------------------------------------------------------
// FlatCurve
//

use qrs_chrono::DateTime;
use qrs_finance::core::daycount::Act365fRate;
use qrs_math::num::Real;

use super::YieldCurve;

/// A flat curve is a curve that has a constant value for all tenors.
#[derive(Debug, Clone, PartialEq, PartialOrd)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize, schemars::JsonSchema),
    serde(bound(
        serialize = "V: qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat> + serde::Serialize",
        deserialize = "V: qrs_math::num::FloatBased + qrs_math::num::Vector<V::BaseFloat> + serde::Deserialize<'de>"
    ))
)]
pub struct FlatCurve<V> {
    pub rate: Act365fRate<V>,
}

//
// methods
//
impl<V: Real> YieldCurve for FlatCurve<V> {
    type Value = V;

    fn forward_rate(
        &self,
        _from: &DateTime,
        _to: &DateTime,
    ) -> anyhow::Result<Act365fRate<Self::Value>> {
        Ok(self.rate.clone())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::vec;

    use approx::assert_abs_diff_eq;
    use itertools::iproduct;
    use qrs_chrono::{DateTimeBuilder, Tz};

    use super::*;

    #[test]
    fn test_forward_rate() {
        let dt_builder = DateTimeBuilder::new()
            .with_hms(9, 30, 54)
            .with_timezone(Tz::fixed_offset(9 * 3600).unwrap());
        let curve = FlatCurve {
            rate: Act365fRate::from_rate(0.05),
        };
        let dates = vec![
            dt_builder.clone().with_ymd(2021, 1, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 2).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 3).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 4).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 5).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 4, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 7, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 10, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2022, 1, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2022, 4, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2022, 7, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2031, 1, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2041, 1, 1).build().unwrap(),
        ];
        for (from, to) in iproduct!(dates.iter(), dates.iter()) {
            let rate = curve.forward_rate(from, to).unwrap();
            assert_eq!(rate, Act365fRate::from_rate(0.05));
        }
    }

    #[test]
    fn test_discount() {
        let dt_builder = DateTimeBuilder::new()
            .with_hms(9, 30, 54)
            .with_timezone(Tz::fixed_offset(9 * 3600).unwrap());
        let curve = FlatCurve {
            rate: Act365fRate::from_rate(0.05),
        };
        let dates = vec![
            dt_builder.clone().with_ymd(2021, 1, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 2).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 3).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 4).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 1, 5).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 4, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 7, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2021, 10, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2022, 1, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2022, 4, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2022, 7, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2031, 1, 1).build().unwrap(),
            dt_builder.clone().with_ymd(2041, 1, 1).build().unwrap(),
        ];
        for (from, to) in iproduct!(dates.iter(), dates.iter()) {
            const MILLSEC_PER_YEAR: f64 = 1000. * 60. * 60. * 24. * 365.0;
            let df = curve.discount(from, to).unwrap();
            let dcf = (to - from).millsecs() as f64 / MILLSEC_PER_YEAR;
            assert_abs_diff_eq!(df, (-0.05 * dcf).exp(), epsilon = 1e-15);
        }
    }
}
