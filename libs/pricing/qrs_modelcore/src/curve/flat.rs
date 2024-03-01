// -----------------------------------------------------------------------------
// FlatCurve
//

use qrs_chrono::DateTime;
use qrs_finance::rate::RateAct365f;
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
    pub rate: RateAct365f<V>,
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
    ) -> anyhow::Result<RateAct365f<Self::Value>> {
        Ok(self.rate.clone())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::vec;

    use approx::assert_abs_diff_eq;
    use itertools::iproduct;
    use qrs_chrono::{DateTimeBuilder, TimeZone};

    use super::*;

    #[test]
    fn test_forward_rate() {
        let dt_builder = DateTimeBuilder::new()
            .with_hms(9, 30, 54)
            .unwrap()
            .with_timezone(TimeZone::fixed_offset(9 * 3600).unwrap());
        let curve = FlatCurve {
            rate: RateAct365f::from_rate(0.05),
        };
        let dates = vec![
            dt_builder.with_ymd(2021, 1, 1).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 2).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 3).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 4).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 5).unwrap().build(),
            dt_builder.with_ymd(2021, 4, 1).unwrap().build(),
            dt_builder.with_ymd(2021, 7, 1).unwrap().build(),
            dt_builder.with_ymd(2021, 10, 1).unwrap().build(),
            dt_builder.with_ymd(2022, 1, 1).unwrap().build(),
            dt_builder.with_ymd(2022, 4, 1).unwrap().build(),
            dt_builder.with_ymd(2022, 7, 1).unwrap().build(),
            dt_builder.with_ymd(2031, 1, 1).unwrap().build(),
            dt_builder.with_ymd(2041, 1, 1).unwrap().build(),
        ];
        for (from, to) in iproduct!(dates.iter(), dates.iter()) {
            let rate = curve.forward_rate(from, to).unwrap();
            assert_eq!(rate, RateAct365f::from_rate(0.05));
        }
    }

    #[test]
    fn test_discount() {
        let dt_builder = DateTimeBuilder::new()
            .with_hms(9, 30, 54)
            .unwrap()
            .with_timezone(TimeZone::fixed_offset(9 * 3600).unwrap());
        let curve = FlatCurve {
            rate: RateAct365f::from_rate(0.05),
        };
        let dates = vec![
            dt_builder.with_ymd(2021, 1, 1).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 2).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 3).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 4).unwrap().build(),
            dt_builder.with_ymd(2021, 1, 5).unwrap().build(),
            dt_builder.with_ymd(2021, 4, 1).unwrap().build(),
            dt_builder.with_ymd(2021, 7, 1).unwrap().build(),
            dt_builder.with_ymd(2021, 10, 1).unwrap().build(),
            dt_builder.with_ymd(2022, 1, 1).unwrap().build(),
            dt_builder.with_ymd(2022, 4, 1).unwrap().build(),
            dt_builder.with_ymd(2022, 7, 1).unwrap().build(),
            dt_builder.with_ymd(2031, 1, 1).unwrap().build(),
            dt_builder.with_ymd(2041, 1, 1).unwrap().build(),
        ];
        for (from, to) in iproduct!(dates.iter(), dates.iter()) {
            const MILLSEC_PER_YEAR: f64 = 1000. * 60. * 60. * 24. * 365.0;
            let df = curve.discount(from, to).unwrap();
            let dcf = (to - from).millsecs() as f64 / MILLSEC_PER_YEAR;
            assert_abs_diff_eq!(df, (-0.05 * dcf).exp(), epsilon = 1e-15);
        }
    }
}
