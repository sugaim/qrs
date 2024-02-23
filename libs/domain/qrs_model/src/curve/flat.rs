// -----------------------------------------------------------------------------
// FlatCurve
//

use qrs_core::{
    chrono::{DateTime, Rate},
    num::Real,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::YieldCurve;

/// A flat curve is a curve that has a constant value for all tenors.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(bound(
    serialize = "V: qrs_core::num::FloatBased + qrs_core::num::Vector<V::BaseFloat> + Serialize",
    deserialize = "V: qrs_core::num::FloatBased + qrs_core::num::Vector<V::BaseFloat> + Deserialize<'de>"
))]
pub struct FlatCurve<V> {
    pub rate: Rate<V>,
}

//
// comparison
//
impl<V> PartialEq for FlatCurve<V>
where
    Rate<V>: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.rate == other.rate
    }
}

//
// methods
//
impl<V: Real> YieldCurve for FlatCurve<V> {
    type Value = V;
    type Error = anyhow::Error;

    fn forward_rate(&self, _from: &DateTime, _to: &DateTime) -> anyhow::Result<Rate<Self::Value>> {
        Ok(self.rate.clone())
    }
}

// =============================================================================
#[cfg(test)]
mod tests {
    use std::vec;

    use approx::assert_abs_diff_eq;
    use itertools::iproduct;
    use qrs_core::chrono::{DateTimeBuilder, TimeZone};

    use super::*;

    #[test]
    fn test_forward_rate() {
        let dt_builder = DateTimeBuilder::new()
            .with_hms(9, 30, 54)
            .unwrap()
            .with_timezone(TimeZone::FixedOffset(
                chrono::FixedOffset::east_opt(9 * 3600).unwrap(),
            ));
        let curve = FlatCurve {
            rate: Rate::with_annual(0.05),
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
            assert_eq!(rate, Rate::with_annual(0.05));
        }
    }

    #[test]
    fn test_discount() {
        let dt_builder = DateTimeBuilder::new()
            .with_hms(9, 30, 54)
            .unwrap()
            .with_timezone(TimeZone::FixedOffset(
                chrono::FixedOffset::east_opt(9 * 3600).unwrap(),
            ));
        let curve = FlatCurve {
            rate: Rate::with_annual(0.05),
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
